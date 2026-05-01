//! TLS management for rune-server.
//!
//! # Flow
//! 1. `TlsManager::new()` — load or provision a wildcard cert for `*.domain`
//! 2. `TlsManager::acceptor()` — get a `TlsAcceptor` for the HTTPS listener
//! 3. `TlsManager::spawn_renewal_task()` — background task that re-provisions
//!    30 days before expiry
//!
//! # Environment variables
//! - `RUNE_DOMAIN`      — base domain (e.g. `example.com`).  TLS is only
//!   active when this is set.
//! - `RUNE_ACME_EMAIL`  — contact address for Let's Encrypt account.
//! - `RUNE_ACME_STAGING` — set to `1` to use LE staging (for testing).
//! - `RUNE_CERT_DIR`    — override cert storage directory (default `.rune/certs`).

pub mod acme;
pub mod cert;

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use tokio_rustls::TlsAcceptor;
use tracing::{info, warn};

use cert::CertStore;

// ── Public types ──────────────────────────────────────────────────────────────

/// Manages TLS certificate lifecycle for rune-server.
pub struct TlsManager {
    acceptor: TlsAcceptor,
}

impl TlsManager {
    /// Load an existing cert or provision a new one via ACME DNS-01.
    ///
    /// This will block startup if a new cert must be provisioned — the user
    /// must set the DNS TXT record before pressing Enter.
    pub async fn load_or_provision(
        domain: &str,
        email: &str,
        cert_dir: &str,
        staging: bool,
    ) -> Result<Self> {
        std::fs::create_dir_all(cert_dir)
            .with_context(|| format!("failed to create cert dir '{cert_dir}'"))?;

        let store = CertStore::new(domain, cert_dir);

        // Provision if cert is missing or expiring within 30 days.
        if store.needs_renewal()? {
            info!("Provisioning new wildcard certificate for *.{domain}");
            acme::provision(&store, domain, email, staging)
                .await
                .context("ACME provisioning failed")?;
        } else {
            info!("Loaded existing certificate for *.{domain} (valid)");
        }

        let server_config = store.build_server_config()?;
        let acceptor = TlsAcceptor::from(Arc::new(server_config));

        Ok(Self { acceptor })
    }

    /// Returns a cloneable `TlsAcceptor` ready to use in the accept loop.
    pub fn acceptor(&self) -> TlsAcceptor {
        self.acceptor.clone()
    }

    /// Spawn a background task that checks for renewal every 12 hours.
    ///
    /// On renewal, the new cert is written to disk.  The server must be
    /// restarted to pick it up (acceptable for a self-hosted tool; SIGHUP
    /// reload can be added later).
    pub fn spawn_renewal_task(store: CertStore, domain: String, email: String, staging: bool) {
        tokio::spawn(async move {
            let interval = Duration::from_secs(12 * 60 * 60); // 12 h
            loop {
                tokio::time::sleep(interval).await;

                match store.needs_renewal() {
                    Ok(true) => {
                        info!("Certificate for *.{domain} is due for renewal — provisioning");
                        if let Err(e) = acme::provision(&store, &domain, &email, staging).await {
                            warn!("Renewal failed: {e:#}");
                        } else {
                            info!("Certificate renewed — restart rune-server to apply");
                        }
                    }
                    Ok(false) => {}
                    Err(e) => warn!("Could not check cert expiry: {e}"),
                }
            }
        });
    }
}
