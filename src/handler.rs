use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    http::header::{HeaderName, HeaderValue},
    response::Response,
};

use bytes::Bytes;
use http_body_util::BodyExt;

use rune_core::{CoreRequest, Headers, RuneError};
use rune_runtime::Runtime;

use crate::error::map_error;

pub async fn handler(State(runtime): State<Arc<Runtime>>, req: Request) -> Response {
    match handle_inner(runtime, req).await {
        Ok(resp) => resp,
        Err(err) => {
            // minimal error mapping for now
            let (status, msg) = map_error(err);

            Response::builder()
                .status(status)
                .body(Body::from(msg))
                .unwrap()
        }
    }
}

async fn handle_inner(runtime: Arc<Runtime>, req: Request) -> Result<Response, RuneError> {
    let (parts, body) = req.into_parts();

    // -------- method + path --------
    let method = parts.method.to_string();
    let path = parts.uri.path().to_string();

    // -------- headers (preserve duplicates + case-insensitive) --------
    let mut headers = Headers::new();

    for (name, value) in parts.headers.iter() {
        let name = name.as_str().to_string();

        // avoid panic on invalid utf8
        let value = match value.to_str() {
            Ok(v) => v.to_string(),
            Err(_) => continue, // skip invalid header values
        };

        headers.insert(name, value);
    }

    // -------- body (raw bytes) --------
    let body_bytes: Bytes = body
        .collect()
        .await
        .map_err(|e| RuneError::ExecutionError(e.to_string()))?
        .to_bytes();

    let core_req = CoreRequest {
        method,
        path,
        headers,
        body: body_bytes.to_vec(), // ByteBuf
    };

    // -------- runtime --------
    let core_resp = runtime.handle_request(core_req)?;

    // -------- build HTTP response --------
    let mut builder = Response::builder().status(core_resp.status);

    let headers_map = builder.headers_mut().unwrap();

    for (k, v) in core_resp.headers.iter() {
        let name = match HeaderName::from_bytes(k.as_bytes()) {
            Ok(name) => name,
            Err(_) => continue,
        };
        let value = match HeaderValue::from_str(v) {
            Ok(value) => value,
            Err(_) => continue,
        };
        headers_map.append(name, value);
    }

    let response = builder.body(Body::from(core_resp.body)).unwrap();

    Ok(response)
}
