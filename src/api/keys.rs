use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, post},
    Router,
};
use serde::{Deserialize, Serialize};

use crate::api::ApiState;

pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/keys", post(create).get(list))
        .route("/keys/{id}", delete(revoke))
}

// ── POST /api/keys ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CreateKeyBody {
    name: String,
}

#[derive(Serialize)]
struct CreatedKeyResponse {
    id: String,
    name: String,
    /// Shown exactly once — store it safely, it cannot be recovered.
    key: String,
}

async fn create(
    State(state): State<ApiState>,
    Json(body): Json<CreateKeyBody>,
) -> Result<Json<CreatedKeyResponse>, (StatusCode, String)> {
    if body.name.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "name cannot be empty".into()));
    }

    let new_key = rune_registry::create_api_key(&state.pool, &body.name)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(CreatedKeyResponse {
        id: new_key.id,
        name: new_key.name,
        key: new_key.raw,
    }))
}

// ── GET /api/keys ─────────────────────────────────────────────────────────────

async fn list(
    State(state): State<ApiState>,
) -> Result<Json<Vec<rune_registry::ApiKeyRecord>>, (StatusCode, String)> {
    let keys = rune_registry::list_api_keys(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(keys))
}

// ── DELETE /api/keys/:id ──────────────────────────────────────────────────────

async fn revoke(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let deleted = rune_registry::revoke_api_key(&state.pool, &id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, format!("key '{id}' not found")))
    }
}
