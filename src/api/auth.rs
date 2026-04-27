use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use sqlx::SqlitePool;

/// Tower middleware that checks `Authorization: Bearer rune_sk_…` on every
/// request to the control-plane router.
pub async fn require_api_key(
    State(pool): State<SqlitePool>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let raw_key = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let found = rune_registry::verify_api_key(&pool, raw_key)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if found.is_none() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(req).await)
}
