mod api;
mod bootstrap;
mod error;
mod handler;
mod tls;

use std::sync::Arc;

use anyhow::Result;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
    routing::any,
    Router,
};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder as AutoBuilder,
    service::TowerToHyperService,
};
use rune_registry::InMemoryFunctionStore;
use rune_runtime::Runtime;
use tokio::net::TcpListener;
use tracing::{info, warn};
use tracing_subscriber::{fmt, EnvFilter};

use crate::{
    api::{router as api_router, ApiState},
    bootstrap::load_deployments,
    handler::{handler, RuntimeState},
    tls::{cert::CertStore, TlsManager},
};

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    // ── Crypto provider ───────────────────────────────────────────────────────
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("failed to install rustls crypto provider"))?;

    // ── Logging ───────────────────────────────────────────────────────────────
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // ── Config from environment ───────────────────────────────────────────────
    let db_path = env("RUNE_DB_PATH", ".rune/rune.db");
    let wasm_dir = env("RUNE_WASM_DIR", ".rune/functions");
    let api_addr = env("RUNE_API_ADDR", "127.0.0.1:3001");
    let base_domain = std::env::var("RUNE_DOMAIN").ok();
    let acme_email = std::env::var("RUNE_ACME_EMAIL").ok();
    let acme_staging = std::env::var("RUNE_ACME_STAGING")
        .map(|v| v == "1")
        .unwrap_or(false);
    let cert_dir = env("RUNE_CERT_DIR", ".rune/certs");

    // ── TLS mode detection ────────────────────────────────────────────────────
    let tls_mode = base_domain.is_some();
    if tls_mode && acme_email.is_none() {
        anyhow::bail!(
            "RUNE_DOMAIN is set but RUNE_ACME_EMAIL is missing. \
             Set RUNE_ACME_EMAIL to your contact address for Let's Encrypt."
        );
    }

    // Bind addresses depend on whether TLS is active.
    let fn_http_addr = env(
        "RUNE_ADDR",
        if tls_mode {
            "0.0.0.0:80"
        } else {
            "0.0.0.0:3000"
        },
    );
    let fn_https_addr = env("RUNE_HTTPS_ADDR", "0.0.0.0:443");

    // ── Database ──────────────────────────────────────────────────────────────
    std::fs::create_dir_all(".rune")?;
    let pool = rune_registry::open(&db_path).await?;
    let _ = rune_registry::run_migrations(&pool);

    // ── First-run: generate an API key if none exist ──────────────────────────
    let keys = rune_registry::list_api_keys(&pool).await?;
    if keys.is_empty() {
        let new_key = rune_registry::create_api_key(&pool, "default").await?;
        tracing::warn!("══════════════════════════════════════════════════════════");
        tracing::warn!("  First run — API key generated (shown once, save it):");
        tracing::warn!("  {}", new_key.raw);
        tracing::warn!("  Run:  rune auth save --key {}", new_key.raw);
        tracing::warn!("══════════════════════════════════════════════════════════");
    }

    // ── In-memory store ───────────────────────────────────────────────────────
    let store = Arc::new(InMemoryFunctionStore::new());
    let loaded = load_deployments(&pool, store.as_ref()).await?;
    info!("{loaded} function(s) loaded from database");

    // ── Runtime ───────────────────────────────────────────────────────────────
    let runtime = Arc::new(Runtime::new(
        store.clone(),
        rune_core::RuntimeConfig {
            max_fuel: 1_000_000,
            max_memory_bytes: 64 * 1024 * 1024,
        },
    )?);

    // ── Function-traffic router ───────────────────────────────────────────────
    let rt_state = RuntimeState {
        runtime: runtime.clone(),
        store: store.clone(),
        base_domain: base_domain.clone(),
    };
    let fn_router = Router::new()
        .route("/", any(handler))
        .route("/{*path}", any(handler))
        .with_state(rt_state);

    // ── Control-plane router ──────────────────────────────────────────────────
    let api_state = ApiState {
        pool: pool.clone(),
        store: store.clone(),
        wasm_dir,
    };
    let admin_router = api_router(api_state);

    // ── TLS provisioning (when RUNE_DOMAIN is set) ────────────────────────────
    let tls_manager = if let (Some(domain), Some(email)) = (&base_domain, &acme_email) {
        let mgr = TlsManager::load_or_provision(domain, email, &cert_dir, acme_staging).await?;

        // Spawn background renewal checker.
        let cert_store = CertStore::new(domain, &cert_dir);
        TlsManager::spawn_renewal_task(cert_store, domain.clone(), email.clone(), acme_staging);

        Some(mgr)
    } else {
        None
    };

    // ── Bind listeners ────────────────────────────────────────────────────────
    let api_listener = TcpListener::bind(&api_addr).await?;
    let http_listener = TcpListener::bind(&fn_http_addr).await?;

    if let Some(tls) = tls_manager {
        // ── TLS mode: :80 redirects to :443, :443 serves with TLS ────────────
        let https_listener = TcpListener::bind(&fn_https_addr).await?;
        let domain = base_domain.clone().unwrap();

        info!("HTTP  (redirect)  →  http://{fn_http_addr}");
        info!("HTTPS (functions) →  https://{fn_https_addr}");
        info!("Control plane     →  http://{api_addr}  (localhost only)");

        let redirect_domain = domain.clone();
        tokio::try_join!(
            serve_https(https_listener, fn_router, tls),
            serve_redirect(http_listener, redirect_domain),
            async {
                axum::serve(api_listener, admin_router)
                    .await
                    .map_err(anyhow::Error::from)
            },
        )?;
    } else {
        // ── Plain HTTP mode ───────────────────────────────────────────────────
        info!("Function traffic  →  http://{fn_http_addr}  (no TLS — set RUNE_DOMAIN to enable)");
        info!("Control plane     →  http://{api_addr}  (localhost only)");

        tokio::try_join!(
            axum::serve(http_listener, fn_router),
            axum::serve(api_listener, admin_router),
        )?;
    }

    Ok(())
}

// ── HTTPS accept loop ─────────────────────────────────────────────────────────

/// Accept TLS connections and serve the function router.
async fn serve_https(listener: TcpListener, router: Router, tls_manager: TlsManager) -> Result<()> {
    let acceptor = tls_manager.acceptor();

    loop {
        let (stream, remote_addr) = match listener.accept().await {
            Ok(v) => v,
            Err(e) => {
                warn!("TCP accept error: {e}");
                continue;
            }
        };

        let acceptor = acceptor.clone();
        let router = router.clone();

        tokio::spawn(async move {
            // TLS handshake.
            let tls_stream = match acceptor.accept(stream).await {
                Ok(s) => s,
                Err(e) => {
                    warn!("TLS handshake failed from {remote_addr}: {e}");
                    return;
                }
            };

            let io = TokioIo::new(tls_stream);
            let service = TowerToHyperService::new(router);

            if let Err(e) = AutoBuilder::new(TokioExecutor::new())
                .serve_connection(io, service)
                .await
            {
                warn!("HTTPS connection error from {remote_addr}: {e}");
            }
        });
    }
}

// ── HTTP → HTTPS redirect ─────────────────────────────────────────────────────

/// Serve permanent redirects from HTTP (:80) to HTTPS (:443).
async fn serve_redirect(listener: TcpListener, domain: String) -> Result<()> {
    let redirect_router = Router::new().fallback(move |req: Request<Body>| {
        let domain = domain.clone();
        async move { redirect_to_https(req, &domain) }
    });

    axum::serve(listener, redirect_router).await?;
    Ok(())
}

fn redirect_to_https(req: Request<Body>, domain: &str) -> Response<Body> {
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");

    let https_url = format!("https://{domain}{path_and_query}");

    Response::builder()
        .status(StatusCode::MOVED_PERMANENTLY)
        .header("location", https_url)
        .body(Body::empty())
        .unwrap()
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn env(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
