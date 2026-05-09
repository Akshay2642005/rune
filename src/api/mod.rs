pub mod auth;
pub mod functions;
pub mod keys;
pub mod session;

use axum::{middleware, routing::post, Router};
use sqlx::SqlitePool;
use std::sync::Arc;

use rune_core::FunctionStore;

#[derive(Clone)]
pub struct ApiState {
    pub pool: SqlitePool,
    pub store: Arc<dyn FunctionStore>,
    pub wasm_dir: String,
}

pub fn router(state: ApiState) -> Router {
    // Unauthenticated: login sets cookie, logout clears it
    let session_routes = Router::new()
        .route("/ui/session", post(session::login).delete(session::logout))
        .with_state(state.pool.clone());

    // Protected API routes
    let protected = Router::new()
        .merge(functions::router())
        .merge(keys::router())
        .layer(middleware::from_fn_with_state(
            state.pool.clone(),
            auth::require_api_key,
        ))
        .with_state(state);

    Router::new()
        .merge(session_routes)
        .nest("/api", protected)
}
