use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    http::header::{HeaderName, HeaderValue},
    response::Response,
};
use bytes::Bytes;
use http_body_util::BodyExt;

use rune_core::{CoreRequest, FunctionStore, Headers, RuneError};
use rune_runtime::Runtime;

use crate::error::map_error;

/// Shared state for the function-traffic router.
#[derive(Clone)]
pub struct RuntimeState {
    pub runtime: Arc<Runtime>,
    pub store: Arc<dyn FunctionStore>,
    pub base_domain: Option<String>, // e.g. Some("yourdomain.com")
}

pub async fn handler(State(state): State<RuntimeState>, req: Request) -> Response {
    match handle_inner(state, req).await {
        Ok(resp) => resp,
        Err(err) => {
            let (status, msg) = map_error(err);
            Response::builder()
                .status(status)
                .body(Body::from(msg))
                .unwrap()
        }
    }
}

async fn handle_inner(state: RuntimeState, req: Request) -> Result<Response, RuneError> {
    let (parts, body) = req.into_parts();

    let method = parts.method.to_string();
    let path = parts.uri.path().to_string();

    // ── Headers ───────────────────────────────────────────────────────────────
    let mut headers = Headers::new();
    for (name, value) in parts.headers.iter() {
        if let Ok(v) = value.to_str() {
            headers.insert(name.as_str().to_string(), v.to_string());
        }
    }

    // ── Subdomain routing ─────────────────────────────────────────────────────
    // If the Host header contains a subdomain of base_domain, look up by
    // subdomain first.  Fall back to path-based routing.
    let func = if let Some(base) = &state.base_domain {
        let host = headers.get("host").unwrap_or("");
        if let Some(sub) = extract_subdomain(host, base) {
            state.store.get_by_subdomain(sub)?
        } else {
            None
        }
    } else {
        None
    };

    // Fall back to path routing.
    let func = match func {
        Some(f) => f,
        None => state
            .store
            .get_by_route(&path)?
            .ok_or(RuneError::NotFound)?,
    };

    // ── Body ──────────────────────────────────────────────────────────────────
    let body_bytes: Bytes = body
        .collect()
        .await
        .map_err(|e| RuneError::ExecutionError(e.to_string()))?
        .to_bytes();

    let core_req = CoreRequest {
        method,
        path,
        headers,
        body: body_bytes.to_vec(),
    };

    // ── Dispatch ──────────────────────────────────────────────────────────────
    let runtime = state.runtime.clone();
    let core_resp =
        tokio::task::spawn_blocking(move || runtime.handle_request_with_function(core_req, func))
            .await
            .map_err(|e| RuneError::InternalError(e.to_string()))??;

    // ── Build HTTP response ───────────────────────────────────────────────────
    let mut builder = Response::builder().status(core_resp.status);
    let headers_map = builder.headers_mut().unwrap();
    for (k, v) in core_resp.headers.iter() {
        if let (Ok(name), Ok(val)) = (
            HeaderName::from_bytes(k.as_bytes()),
            HeaderValue::from_str(v),
        ) {
            headers_map.append(name, val);
        }
    }

    Ok(builder.body(Body::from(core_resp.body)).unwrap())
}

/// Extract the subdomain label from a Host header value.
///
/// `extract_subdomain("hello.yourdomain.com", "yourdomain.com")` → `Some("hello")`
fn extract_subdomain<'a>(host: &'a str, base_domain: &str) -> Option<&'a str> {
    // Strip optional port.
    let host = host.split(':').next()?;
    let suffix = format!(".{base_domain}");
    host.strip_suffix(suffix.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subdomain_extraction() {
        assert_eq!(
            extract_subdomain("hello.yourdomain.com", "yourdomain.com"),
            Some("hello")
        );
        assert_eq!(extract_subdomain("yourdomain.com", "yourdomain.com"), None);
        assert_eq!(
            extract_subdomain("hello.yourdomain.com:443", "yourdomain.com"),
            Some("hello")
        );
        assert_eq!(extract_subdomain("other.com", "yourdomain.com"), None);
    }
}
