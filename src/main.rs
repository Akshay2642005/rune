mod error;
mod handler;

use crate::handler::handler;
use axum::{routing::any, Router};
use std::sync::Arc;

use rune_core::{FunctionMeta, FunctionStore};
use rune_registry::InMemoryFunctionStore;
use rune_runtime::Runtime;

#[tokio::main]
async fn main() {
    let store = Arc::new(InMemoryFunctionStore::new());

    let func: FunctionMeta = FunctionMeta {
        id: "hello".to_string(),
        route: "/hello".to_string(),
        wasm_path: "crates/runtime/tests/fixtures/hello.wasm".to_string(),
    };

    let config = rune_core::RuntimeConfig {
        max_fuel: 1_000_000,
        max_memory_bytes: 64 * 1024 * 1024,
        request_timeout_ms: 5000,
    };
    store.register(func).expect("Failed to register function");

    let runtime = Arc::new(Runtime::new(store.clone(), config).expect("Failed to create runtime"));

    let app = Router::new()
        .route("/{*path}", any(handler))
        .with_state(runtime);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("server running on http://localhost:3000");

    axum::serve(listener, app).await.unwrap();
}
