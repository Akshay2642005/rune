use std::{fs, fs::OpenOptions, io::Write, path::PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RuneConfig {
    /// Base URL of the control-plane API (e.g. "http://localhost:3001").
    pub server_url: Option<String>,
    /// Raw API key (rune_sk_…).
    pub api_key: Option<String>,
}

impl RuneConfig {
    pub fn load() -> anyhow::Result<Self> {
        let path = config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read config '{}'", path.display()))?;
        toml::from_str(&raw).with_context(|| format!("failed to parse config '{}'", path.display()))
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&path)
            .with_context(|| format!("failed to write config '{}'", path.display()))?;
        file.write_all(contents.as_bytes())
            .with_context(|| format!("failed to write config '{}'", path.display()))
    }

    pub fn require_server_url(&self) -> anyhow::Result<&str> {
        self.server_url
            .as_deref()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "no server URL configured — run: rune auth save --url <URL> --key <KEY>"
                )
            })
    }

    pub fn require_api_key(&self) -> anyhow::Result<&str> {
        self.api_key
            .as_deref()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "no API key configured — run: rune auth save --url <URL> --key <KEY>"
                )
            })
    }
}

fn config_path() -> anyhow::Result<PathBuf> {
    let base =
        dirs::config_dir().ok_or_else(|| anyhow::anyhow!("could not locate config directory"))?;
    Ok(base.join("rune").join("config.toml"))
}
