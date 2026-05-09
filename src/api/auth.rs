use axum::{
    extract::{Request, State},
    http::{Method, StatusCode},
    middleware::Next,
    response::Response,
};
use sqlx::SqlitePool;

use super::session::extract_session_key;

/// Tower middleware that checks either:
/// - `Authorization: Bearer rune_sk_…` header, or
/// - `rune_session=…` httpOnly cookie
pub async fn require_api_key(
    State(pool): State<SqlitePool>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let is_bootstrap = req.method() == Method::POST && req.uri().path() == "/api/keys";
    if is_bootstrap {
        let active_keys: i64 =
            sqlx::query_scalar("SELECT COUNT(1) FROM api_keys WHERE revoked_at IS NULL")
                .fetch_one(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if active_keys == 0 {
            return Ok(next.run(req).await);
        }
    }

    // Accept Bearer token OR session cookie.
    let raw_key = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .or_else(|| extract_session_key(req.headers()))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let found = rune_registry::verify_api_key(&pool, &raw_key)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if found.is_none() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(req).await)
}
