use std::{fs, path::Path};

use axum::{
    extract::{Multipart, Path as AxumPath, State},
    http::StatusCode,
    response::Json,
    routing::{delete, post},
    Router,
};
use serde::Serialize;

use rune_core::FunctionMeta;

use crate::api::ApiState;

pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/functions", post(deploy).get(list))
        .route("/functions/{id}", delete(remove).get(get_one))
}

// ── Request/response types ────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct FunctionResponse {
    id: String,
    subdomain: Option<String>,
    route: String,
    wasm_path: String,
}

impl From<FunctionMeta> for FunctionResponse {
    fn from(m: FunctionMeta) -> Self {
        Self {
            id: m.id,
            subdomain: m.subdomain,
            route: m.route,
            wasm_path: m.wasm_path,
        }
    }
}

// ── POST /api/functions ───────────────────────────────────────────────────────
//
// Multipart fields:
//   id        — function identifier (required)
//   route     — URL path, must start with '/' (required)
//   subdomain — label for <subdomain>.<base_domain> (optional)
//   wasm      — the compiled .wasm file (required)

async fn deploy(
    State(state): State<ApiState>,
    mut multipart: Multipart,
) -> Result<Json<FunctionResponse>, (StatusCode, String)> {
    let mut id: Option<String> = None;
    let mut route: Option<String> = None;
    let mut subdomain: Option<String> = None;
    let mut wasm_bytes: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.map_err(bad_request)? {
        match field.name() {
            Some("id") => id = Some(field.text().await.map_err(bad_request)?),
            Some("route") => route = Some(field.text().await.map_err(bad_request)?),
            Some("subdomain") => subdomain = Some(field.text().await.map_err(bad_request)?),
            Some("wasm") => wasm_bytes = Some(field.bytes().await.map_err(bad_request)?.to_vec()),
            _ => {}
        }
    }

    let id = id.ok_or_else(|| bad_request("missing field: id"))?;
    let route = route.ok_or_else(|| bad_request("missing field: route"))?;
    let bytes = wasm_bytes.ok_or_else(|| bad_request("missing field: wasm"))?;

    if id.trim().is_empty() {
        return Err(bad_request("id cannot be empty"));
    }
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(bad_request("id must not contain path separators or '..'"));
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(bad_request(
            "id must contain only ASCII letters, numbers, '-' or '_'",
        ));
    }
    if !route.starts_with('/') {
        return Err(bad_request("route must start with '/'"));
    }

    // Strip empty subdomain string → None
    let subdomain = subdomain.filter(|s| !s.trim().is_empty());

    // Preflight route/subdomain conflicts before mutating disk or DB.
    if let Some(existing) = state
        .store
        .get_by_route(&route)
        .map_err(|e| internal(e.to_string()))?
    {
        if existing.id != id {
            return Err((
                StatusCode::CONFLICT,
                format!("route already registered: {route}"),
            ));
        }
    }
    if let Some(sub) = &subdomain {
        if let Some(existing) = state
            .store
            .get_by_subdomain(sub)
            .map_err(|e| internal(e.to_string()))?
        {
            if existing.id != id {
                return Err((
                    StatusCode::CONFLICT,
                    format!("subdomain already registered: {sub}"),
                ));
            }
        }
    }

    // Write WASM artifact to disk.
    fs::create_dir_all(&state.wasm_dir)
        .map_err(|e| internal(format!("failed to create wasm dir: {e}")))?;

    let wasm_path = Path::new(&state.wasm_dir).join(format!("{id}.wasm"));
    fs::write(&wasm_path, &bytes).map_err(|e| internal(format!("failed to write wasm: {e}")))?;

    let meta = FunctionMeta {
        id: id.clone(),
        subdomain: subdomain.clone(),
        route: route.clone(),
        wasm_path: wasm_path.to_string_lossy().into_owned(),
    };

    // Persist to SQLite.
    rune_registry::upsert_function(&state.pool, &meta)
        .await
        .map_err(|e| internal(e.to_string()))?;

    // Update the hot in-memory cache.
    state.store.register(meta.clone()).map_err(|e| {
        // If in-memory rejects (e.g. duplicate route from different id), roll back DB + artifact.
        let _ = tokio::spawn({
            let pool = state.pool.clone();
            let id = id.clone();
            let wasm_path = wasm_path.clone();
            async move {
                let _ = rune_registry::delete_function(&pool, &id).await;
                let _ = std::fs::remove_file(&wasm_path);
            }
        });
        (StatusCode::CONFLICT, e.to_string())
    })?;

    Ok(Json(meta.into()))
}

// ── GET /api/functions ────────────────────────────────────────────────────────

async fn list(
    State(state): State<ApiState>,
) -> Result<Json<Vec<FunctionResponse>>, (StatusCode, String)> {
    let functions = rune_registry::list_functions(&state.pool)
        .await
        .map_err(|e| internal(e.to_string()))?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(Json(functions))
}

// ── GET /api/functions/:id ────────────────────────────────────────────────────

async fn get_one(
    State(state): State<ApiState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<FunctionResponse>, StatusCode> {
    let all = rune_registry::list_functions(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    all.into_iter()
        .find(|f| f.id == id)
        .map(|f| Json(f.into()))
        .ok_or(StatusCode::NOT_FOUND)
}

// ── DELETE /api/functions/:id ─────────────────────────────────────────────────

async fn remove(
    State(state): State<ApiState>,
    AxumPath(id): AxumPath<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let deleted = rune_registry::delete_function(&state.pool, &id)
        .await
        .map_err(|e| internal(e.to_string()))?;

    if !deleted {
        return Err((StatusCode::NOT_FOUND, format!("function '{id}' not found")));
    }

    // Remove from in-memory cache (best-effort; ignore NotFound).
    let _ = state.store.remove(&id);

    Ok(StatusCode::NO_CONTENT)
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn bad_request(msg: impl ToString) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, msg.to_string())
}

fn internal(msg: impl ToString) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, msg.to_string())
}
