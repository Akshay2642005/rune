use axum::http::StatusCode;
use rune_core::RuneError;

pub fn map_error(err: RuneError) -> (StatusCode, String) {
    match err {
        RuneError::NotFound => (StatusCode::NOT_FOUND, "not found".into()),
        RuneError::ExecutionError(e) => (StatusCode::BAD_GATEWAY, e),
        RuneError::InternalError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
        RuneError::InvalidRequest(e) => (StatusCode::BAD_REQUEST, e),
        RuneError::Timeout => (StatusCode::GATEWAY_TIMEOUT, "request timed out".into()),
        RuneError::OutOfFuel => (StatusCode::TOO_MANY_REQUESTS, "out of fuel".into()),
        RuneError::DuplicateIdentifier { field, value } => {
            (StatusCode::CONFLICT, format!("{field}:{value}"))
        }
    }
}
