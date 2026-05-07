use std::{path::Path, time::Duration};

use anyhow::Context;
use reqwest::{
    Client, StatusCode,
    multipart::{Form, Part},
};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct RuneClient {
    client: Client,
    server_url: String,
    /// URL for function invocation traffic (default: same host as server_url but port 3000).
    pub function_url: String,
    api_key: String,
}

impl RuneClient {
    pub fn new(server_url: impl Into<String>, api_key: impl Into<String>) -> anyhow::Result<Self> {
        Self::with_function_url(server_url, "", api_key)
    }

    pub fn with_function_url(
        server_url: impl Into<String>,
        function_url: impl Into<String>,
        api_key: impl Into<String>,
    ) -> anyhow::Result<Self> {
        let server_url = server_url.into();
        let api_key = api_key.into();
        let parsed = reqwest::Url::parse(&server_url)
            .with_context(|| format!("invalid server URL: '{server_url}'"))?;
        if parsed.scheme() != "https" {
            let host = parsed.host_str().unwrap_or_default();
            let is_loopback = matches!(host, "localhost" | "127.0.0.1" | "::1");
            if !(parsed.scheme() == "http" && is_loopback) {
                anyhow::bail!(
                    "insecure server URL scheme '{}': HTTPS is required",
                    parsed.scheme()
                );
            }
        }

        // Default function_url: same host as server_url but port 3000.
        let fu = function_url.into();
        let function_url = if fu.is_empty() {
            let mut u = parsed.clone();
            let _ = u.set_port(Some(3000));
            u.origin().ascii_serialization()
        } else {
            fu.trim_end_matches('/').to_string()
        };

        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .connect_timeout(Duration::from_secs(5))
                .build()
                .expect("failed to build HTTP client"),
            server_url: server_url.trim_end_matches('/').to_string(),
            function_url,
            api_key,
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}/api/{path}", self.server_url)
    }

    pub fn server_url(&self) -> &str {
        &self.server_url
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req.header("Authorization", format!("Bearer {}", self.api_key))
    }

    pub async fn deploy(
        &self,
        id: &str,
        route: Option<&str>,
        subdomain: Option<&str>,
        wasm_path: &Path,
    ) -> anyhow::Result<FunctionRecord> {
        let bytes = std::fs::read(wasm_path)
            .with_context(|| format!("failed to read '{}'", wasm_path.display()))?;

        let mut form = Form::new()
            .text("id", id.to_string())
            .part("wasm", Part::bytes(bytes).file_name("function.wasm"));

        if let Some(route) = route.filter(|r| !r.trim().is_empty()) {
            form = form.text("route", route.to_string());
        }

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

    /// Sends a GET request to the function's route and returns the response body.
    /// Uses function_url (e.g. localhost:3000), not the control-plane URL.
    pub async fn invoke_function(&self, route: &str) -> anyhow::Result<String> {
        let url = format!("{}{}", self.function_url, route);
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if status.is_success() { Ok(body) } else { anyhow::bail!("{status}: {body}") }
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

#[derive(Clone, Debug, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KeyRecord {
    pub id: String,
    pub name: String,
    pub created_at: i64,
}
