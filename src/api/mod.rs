pub mod auth;
pub mod functions;
pub mod keys;

use axum::{middleware, Router};
use sqlx::SqlitePool;
use std::sync::Arc;

use rune_core::FunctionStore;

/// Shared state injected into every control-plane handler.
#[derive(Clone)]
pub struct ApiState {
    pub pool: SqlitePool,
    pub store: Arc<dyn FunctionStore>,
    pub wasm_dir: String,
}

/// Build the control-plane router.
///
/// All routes under `/api/` require a valid `Authorization: Bearer rune_sk_…`
/// header, enforced by the `auth::require_api_key` middleware.
pub fn router(state: ApiState) -> Router {
    let protected = Router::new()
        .merge(functions::router())
        .merge(keys::router())
        .layer(middleware::from_fn_with_state(
            state.pool.clone(),
            auth::require_api_key,
        ))
        .with_state(state);

    Router::new().nest("/api", protected)
}
