mod bootstrap;
mod error;
mod handler;

use crate::{bootstrap::load_deployments, handler::handler};
use axum::routing::any;
use axum::Router;
use rune_registry::InMemoryFunctionStore;
use rune_runtime::Runtime;
use std::sync::Arc;

#[cfg(test)]
pub(crate) static TEST_CWD_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::main]
async fn main() {
    let store = Arc::new(InMemoryFunctionStore::new());
    let loaded = load_deployments(store.as_ref()).expect("Failed to load deployments");

    let config = rune_core::RuntimeConfig {
        max_fuel: 1_000_000,
        max_memory_bytes: 64 * 1024 * 1024,
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, extract::State, http::Request};
    use http_body_util::BodyExt;
    use runectl::deploy_function;
    use std::{
        env,
        path::{Path, PathBuf},
    };

    struct CurrentDirGuard {
        previous: PathBuf,
    }

    impl CurrentDirGuard {
        fn enter(path: &Path) -> Self {
            let previous = env::current_dir().unwrap();
            env::set_current_dir(path).unwrap();
            Self { previous }
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            env::set_current_dir(&self.previous).unwrap();
        }
    }

    #[tokio::test]
    async fn alpha_smoke_test_deploys_bootstraps_and_serves_hello() {
        let _lock = crate::TEST_CWD_LOCK.lock().await;
        let temp_dir = tempfile::tempdir().unwrap();
        let _guard = CurrentDirGuard::enter(temp_dir.path());

        let hello_fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/runtime/tests/fixtures/hello.wasm");
        deploy_function("hello", "/hello", &hello_fixture).unwrap();

        let store = Arc::new(InMemoryFunctionStore::new());
        let loaded = load_deployments(store.as_ref()).unwrap();
        assert_eq!(loaded, 1);

        let runtime = Arc::new(
            Runtime::new(
                store,
                rune_core::RuntimeConfig {
                    max_fuel: 1_000_000,
                    max_memory_bytes: 64 * 1024 * 1024,
                },
            )
            .unwrap(),
        );

        let request = Request::builder()
            .method("GET")
            .uri("/hello")
            .body(Body::empty())
            .unwrap();

        let response = handler(State(runtime), request).await;
        assert_eq!(response.status().as_u16(), 200);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body.as_ref(), b"hello");
    }
}
