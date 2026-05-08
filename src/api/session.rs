use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use sqlx::SqlitePool;

const COOKIE_NAME: &str = "rune_session";

#[derive(Deserialize)]
pub struct LoginRequest {
    pub key: String,
}

/// POST /ui/session — validate key, set httpOnly session cookie.
pub async fn login(
    State(pool): State<SqlitePool>,
    Json(body): Json<LoginRequest>,
) -> Response {
    let found = rune_registry::verify_api_key(&pool, &body.key)
        .await
        .unwrap_or(None);

    if found.is_none() {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let cookie = format!(
        "{COOKIE_NAME}={}; HttpOnly; SameSite=Strict; Path=/; Max-Age=86400",
        body.key
    );

    (StatusCode::OK, [(header::SET_COOKIE, cookie)]).into_response()
}

/// DELETE /ui/session — clear the session cookie.
pub async fn logout() -> Response {
    let cookie = format!(
        "{COOKIE_NAME}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0"
    );
    (StatusCode::OK, [(header::SET_COOKIE, cookie)]).into_response()
}

/// Extract the session key from the cookie header.
pub fn extract_session_key(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|part| {
                let part = part.trim();
                part.strip_prefix(&format!("{COOKIE_NAME}="))
                    .map(|v| v.to_string())
            })
        })
}
