use std::{
    fs,
    io::BufReader,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use rustls::ServerConfig;
use rustls_pemfile::{certs, private_key};
use serde::{Deserialize, Serialize};
use tokio_rustls::rustls;

/// How many days before expiry to start renewing.
const RENEW_BEFORE_DAYS: u64 = 30;

/// Handles on-disk cert/key files for a single domain.
#[derive(Clone)]
pub struct CertStore {
    domain: String,
    cert_dir: PathBuf,
}

impl CertStore {
    pub fn new(domain: &str, cert_dir: &str) -> Self {
        Self {
            domain: domain.to_string(),
            cert_dir: PathBuf::from(cert_dir),
        }
    }

    pub fn cert_path(&self) -> PathBuf {
        self.cert_dir.join(format!("{}.crt", self.domain))
    }

    pub fn key_path(&self) -> PathBuf {
        self.cert_dir.join(format!("{}.key", self.domain))
    }

    pub fn account_path(&self) -> PathBuf {
        self.cert_dir.join("account.json")
    }

    fn meta_path(&self) -> PathBuf {
        self.cert_dir.join(format!("{}.meta.json", self.domain))
    }

    /// Save the certificate chain PEM and private key PEM to disk.
    pub fn save(&self, cert_pem: &str, key_pem: &str, expires_at: u64) -> Result<()> {
        fs::write(self.cert_path(), cert_pem)
            .with_context(|| format!("failed to write cert to '{}'", self.cert_path().display()))?;

        // Restrict key file permissions (Unix only).
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(self.key_path())
                .and_then(|mut f| {
                    use std::io::Write;
                    f.write_all(key_pem.as_bytes())
                })
                .with_context(|| {
                    format!("failed to write key to '{}'", self.key_path().display())
                })?;
        }
        #[cfg(not(unix))]
        fs::write(self.key_path(), key_pem)
            .with_context(|| format!("failed to write key to '{}'", self.key_path().display()))?;

        // Write metadata (expiry timestamp).
        let meta = CertMeta { expires_at };
        let json = serde_json::to_string_pretty(&meta)?;
        fs::write(self.meta_path(), json)?;

        Ok(())
    }

    /// Save ACME account credentials JSON.
    pub fn save_account(&self, credentials_json: &str) -> Result<()> {
        fs::write(self.account_path(), credentials_json)
            .context("failed to write ACME account credentials")
    }

    /// Load ACME account credentials JSON if it exists.
    pub fn load_account(&self) -> Option<String> {
        fs::read_to_string(self.account_path()).ok()
    }

    /// Returns `true` when the cert doesn't exist or expires within
    /// `RENEW_BEFORE_DAYS`.
    pub fn needs_renewal(&self) -> Result<bool> {
        if !self.cert_path().exists() || !self.key_path().exists() {
            return Ok(true);
        }

        let meta_path = self.meta_path();
        if !meta_path.exists() {
            // Cert exists but no metadata — treat as needing renewal so we
            // rewrite metadata.
            return Ok(true);
        }

        let json = fs::read_to_string(&meta_path)
            .with_context(|| format!("failed to read cert metadata '{}'", meta_path.display()))?;

        let meta: CertMeta =
            serde_json::from_str(&json).context("failed to parse cert metadata")?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let threshold = RENEW_BEFORE_DAYS * 24 * 60 * 60;
        Ok(meta.expires_at.saturating_sub(now) < threshold)
    }

    /// Build a `rustls::ServerConfig` from the stored cert + key.
    pub fn build_server_config(&self) -> Result<ServerConfig> {
        // Load cert chain.
        let cert_file = fs::File::open(self.cert_path())
            .with_context(|| format!("failed to open '{}'", self.cert_path().display()))?;
        let mut reader = BufReader::new(cert_file);
        let cert_chain: Vec<rustls::pki_types::CertificateDer<'static>> = certs(&mut reader)
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("failed to parse certificate chain")?;

        // Load private key.
        let key_file = fs::File::open(self.key_path())
            .with_context(|| format!("failed to open '{}'", self.key_path().display()))?;
        let mut reader = BufReader::new(key_file);
        let key = private_key(&mut reader)
            .context("failed to parse private key")?
            .context("no private key found in key file")?;

        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, key)
            .context("failed to build TLS server config")?;

        Ok(config)
    }
}

#[derive(Serialize, Deserialize)]
struct CertMeta {
    /// Unix timestamp of cert expiry (NOT_AFTER).
    expires_at: u64,
}
