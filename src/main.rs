mod api;
mod bootstrap;
mod error;
mod handler;

use std::{env, sync::Arc};

use axum::{routing::any, Router};
use rune_registry::InMemoryFunctionStore;
use rune_runtime::Runtime;
use tokio::net::TcpListener;
use tracing_subscriber::{fmt, EnvFilter};

use crate::{
    api::{router as api_router, ApiState},
    bootstrap::load_deployments,
    handler::{handler, RuntimeState},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── Logging ───────────────────────────────────────────────────────────────
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // ── Config from environment ───────────────────────────────────────────────
    let db_path = env::var("RUNE_DB_PATH").unwrap_or_else(|_| ".rune/rune.db".to_string());
    let wasm_dir = env::var("RUNE_WASM_DIR").unwrap_or_else(|_| ".rune/functions".to_string());
    let fn_addr = env::var("RUNE_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    let api_addr = env::var("RUNE_API_ADDR").unwrap_or_else(|_| "127.0.0.1:3001".to_string());
    let base_domain = env::var("RUNE_DOMAIN").ok(); // e.g. "yourdomain.com"

    // ── Database ──────────────────────────────────────────────────────────────
    std::fs::create_dir_all(".rune")?;
    let pool = rune_registry::open(&db_path).await?;
    rune_registry::run_migrations(&pool).await?;

    // ── First-run: generate an API key if none exist ──────────────────────────
    let keys = rune_registry::list_api_keys(&pool).await?;
    if keys.is_empty() {
        let new_key = rune_registry::create_api_key(&pool, "default").await?;
        tracing::warn!("═══════════════════════════════════════════════════════");
        tracing::warn!("  First run — API key generated (shown once, save it):");
        tracing::warn!("  {}", new_key.raw);
        tracing::warn!("  Run:  rune auth save --key {}", new_key.raw);
        tracing::warn!("═══════════════════════════════════════════════════════");
    }

    // ── In-memory store (hot-path read cache) ─────────────────────────────────
    let store = Arc::new(InMemoryFunctionStore::new());
    let loaded = load_deployments(&pool, store.as_ref()).await?;
    tracing::info!("{loaded} function(s) loaded from database");

    // ── Runtime ───────────────────────────────────────────────────────────────
    let config = rune_core::RuntimeConfig {
        max_fuel: 1_000_000,
        max_memory_bytes: 64 * 1024 * 1024,
    };
    let runtime = Arc::new(Runtime::new(store.clone(), config)?);

    // ── Function-traffic router (public) ──────────────────────────────────────
    let rt_state = RuntimeState {
        runtime: runtime.clone(),
        store: store.clone(),
        base_domain,
    };
    let fn_router = Router::new()
        .route("/{*path}", any(handler))
        .with_state(rt_state);

    // ── Control-plane router (admin, localhost only by default) ───────────────
    let api_state = ApiState {
        pool: pool.clone(),
        store: store.clone(),
        wasm_dir,
    };
    let admin_router = api_router(api_state);

    // ── Serve both ────────────────────────────────────────────────────────────
    let fn_listener = TcpListener::bind(&fn_addr).await?;
    let api_listener = TcpListener::bind(&api_addr).await?;

    tracing::info!("Function traffic  →  http://{fn_addr}");
    tracing::info!("Control plane     →  http://{api_addr}  (localhost only)");

    tokio::try_join!(
        axum::serve(fn_listener, fn_router),
        axum::serve(api_listener, admin_router),
    )?;

    Ok(())
}

#[cfg(test)]
pub(crate) static _TEST_CWD_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
