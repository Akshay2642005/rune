mod bootstrap;
mod error;
mod handler;

use crate::{bootstrap::load_deployments, handler::handler};
use axum::{routing::any, Router};
use rune_registry::InMemoryFunctionStore;
use rune_runtime::Runtime;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let store = Arc::new(InMemoryFunctionStore::new());
    let loaded = load_deployments(store.as_ref()).expect("Failed to load deployments");

    let config = rune_core::RuntimeConfig {
        max_fuel: 1_000_000,
        max_memory_bytes: 64 * 1024 * 1024,
        request_timeout_ms: 5000,
    };

    let runtime = Arc::new(Runtime::new(store.clone(), config).expect("Failed to create runtime"));

    let app = Router::new()
        .route("/{*path}", any(handler))
        .with_state(runtime);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("server running on http://localhost:3000 ({loaded} deployed functions loaded)");

    axum::serve(listener, app).await.unwrap();
}
