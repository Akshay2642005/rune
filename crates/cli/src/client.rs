use std::{path::Path, time::Duration};

use anyhow::Context;
use reqwest::{
    Client, StatusCode,
    multipart::{Form, Part},
};
use serde::{Deserialize, Serialize};

pub struct RuneClient {
    client: Client,
    server_url: String,
    api_key: String,
}

impl RuneClient {
    pub fn new(server_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .connect_timeout(Duration::from_secs(5))
                .build()
                .expect("failed to build HTTP client"),
            server_url: server_url.into().trim_end_matches('/').to_string(),
            api_key: api_key.into(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/api/{path}", self.server_url)
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req.header("Authorization", format!("Bearer {}", self.api_key))
    }

    // ── Functions ─────────────────────────────────────────────────────────────

    pub async fn deploy(
        &self,
        id: &str,
        route: &str,
        subdomain: Option<&str>,
        wasm_path: &Path,
    ) -> anyhow::Result<FunctionRecord> {
        let bytes = std::fs::read(wasm_path)
            .with_context(|| format!("failed to read '{}'", wasm_path.display()))?;

        let mut form = Form::new()
            .text("id", id.to_string())
            .text("route", route.to_string())
            .part("wasm", Part::bytes(bytes).file_name("function.wasm"));

        if let Some(sub) = subdomain {
            form = form.text("subdomain", sub.to_string());
        }

        let resp = self
            .auth(self.client.post(self.url("functions")))
            .multipart(form)
            .send()
            .await
            .context("request failed")?;

        parse_response(resp).await
    }

    pub async fn list_functions(&self) -> anyhow::Result<Vec<FunctionRecord>> {
        let resp = self
            .auth(self.client.get(self.url("functions")))
            .send()
            .await?;
        parse_response(resp).await
    }

    pub async fn delete_function(&self, id: &str) -> anyhow::Result<()> {
        let resp = self
            .auth(self.client.delete(self.url(&format!("functions/{id}"))))
            .send()
            .await?;

        if resp.status() == StatusCode::NO_CONTENT {
            return Ok(());
        }
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
    }

    // ── API keys ──────────────────────────────────────────────────────────────

    pub async fn create_key(&self, name: &str) -> anyhow::Result<CreatedKey> {
        let resp = self
            .auth(self.client.post(self.url("keys")))
            .json(&serde_json::json!({ "name": name }))
            .send()
            .await?;
        parse_response(resp).await
    }

    pub async fn list_keys(&self) -> anyhow::Result<Vec<KeyRecord>> {
        let resp = self.auth(self.client.get(self.url("keys"))).send().await?;
        parse_response(resp).await
    }

    pub async fn revoke_key(&self, id: &str) -> anyhow::Result<()> {
        let resp = self
            .auth(self.client.delete(self.url(&format!("keys/{id}"))))
            .send()
            .await?;
        if resp.status() == StatusCode::NO_CONTENT {
            return Ok(());
        }
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
    }
}

async fn parse_response<T: for<'de> Deserialize<'de>>(
    resp: reqwest::Response,
) -> anyhow::Result<T> {
    let status = resp.status();
    if status.is_success() {
        resp.json::<T>()
            .await
            .context("failed to parse response JSON")
    } else {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
    }
}

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct FunctionRecord {
    pub id: String,
    pub subdomain: Option<String>,
    pub route: String,
    pub wasm_path: String,
}

#[derive(Debug, Deserialize)]
pub struct CreatedKey {
    pub id: String,
    pub key: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct KeyRecord {
    pub id: String,
    pub name: String,
    pub created_at: i64,
}
