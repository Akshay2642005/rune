use std::sync::Arc;

pub struct Runtime {
    store: Arc<dyn rune_core::FunctionStore>,
    executor: crate::executor::WasmExecutor,
}

impl Runtime {
    pub fn new(
        store: Arc<dyn rune_core::FunctionStore>,
        config: rune_core::RuntimeConfig,
    ) -> Result<Self, rune_core::RuneError> {
        let executor = crate::WasmExecutor::new(config.max_fuel, config.max_memory_bytes)
            .map_err(|e| rune_core::RuneError::ExecutionError(e.to_string()))?;

        Ok(Self { store, executor })
    }

    pub fn handle_request(
        &self,
        req: rune_core::CoreRequest,
    ) -> Result<rune_core::CoreResponse, rune_core::RuneError> {
        let func = self
            .store
            .get_by_route(&req.path)?
            .ok_or(rune_core::RuneError::NotFound)?;

        let input = serde_json::to_vec(&req).map_err(|e| {
            execution_error(
                &func.id,
                &req.path,
                format!("failed to serialize request: {e}"),
            )
        })?;

        let resp_bytes = self
            .executor
            .execute(&func.wasm_path, &input)
            .map_err(|e| map_executor_error(&func.id, &req.path, e))?;

        if resp_bytes.is_empty() {
            return Err(execution_error(&func.id, &req.path, "empty response"));
        }

        let wasm_resp: rune_core::WasmResponse =
            serde_json::from_slice(&resp_bytes).map_err(|e| {
                execution_error(
                    &func.id,
                    &req.path,
                    format!("failed to parse JSON response: {e}"),
                )
            })?;

        if wasm_resp.status < 100 || wasm_resp.status > 599 {
            return Err(execution_error(
                &func.id,
                &req.path,
                format!("invalid HTTP status {}", wasm_resp.status),
            ));
        }

        Ok(rune_core::CoreResponse {
            status: wasm_resp.status,
            headers: wasm_resp.headers.unwrap_or_default().into(),
            body: wasm_resp.body,
        })
    }
}

fn map_executor_error(
    function_id: &str,
    route: &str,
    error: anyhow::Error,
) -> rune_core::RuneError {
    if crate::executor::is_out_of_fuel(&error) {
        rune_core::RuneError::OutOfFuel
    } else {
        execution_error(function_id, route, error)
    }
}

fn execution_error(
    function_id: &str,
    route: &str,
    detail: impl std::fmt::Display,
) -> rune_core::RuneError {
    rune_core::RuneError::ExecutionError(format!(
        "function '{function_id}' (route '{route}') failed: {detail}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, sync::Arc};

    use rune_core::{CoreRequest, FunctionMeta, FunctionStore, RuneError, RuntimeConfig};
    use rune_registry::InMemoryFunctionStore;
    use tempfile::TempDir;

    const TEST_ROUTE: &str = "/fixture";

    fn default_config() -> RuntimeConfig {
        RuntimeConfig {
            max_fuel: 1_000_000,
            max_memory_bytes: 64 * 1024 * 1024,
        }
    }

    fn request() -> CoreRequest {
        CoreRequest {
            method: "GET".into(),
            path: TEST_ROUTE.into(),
            headers: Default::default(),
            body: vec![],
        }
    }

    fn runtime_with_module(wat_source: &str, config: RuntimeConfig) -> (TempDir, Runtime) {
        let temp_dir = tempfile::tempdir().unwrap();
        let wasm_path = temp_dir.path().join("fixture.wasm");
        fs::write(&wasm_path, wat::parse_str(wat_source).unwrap()).unwrap();

        let store = InMemoryFunctionStore::new();
        store
            .register(FunctionMeta {
                subdomain: None,

                id: "fixture".into(),
                route: TEST_ROUTE.into(),
                wasm_path: wasm_path.to_string_lossy().into_owned(),
            })
            .unwrap();

        let runtime = Runtime::new(Arc::new(store), config).unwrap();

        (temp_dir, runtime)
    }

    fn static_response_module(response_body: &[u8], memory_pages: u32) -> String {
        const RESPONSE_PTR: u32 = 4096;

        let mut payload = Vec::with_capacity(4 + response_body.len());
        payload.extend_from_slice(&(response_body.len() as u32).to_le_bytes());
        payload.extend_from_slice(response_body);

        let encoded_payload = payload
            .iter()
            .map(|byte| format!("\\{:02x}", byte))
            .collect::<String>();

        format!(
            r#"(module
                (memory (export "memory") {memory_pages})
                (data (i32.const {RESPONSE_PTR}) "{encoded_payload}")
                (func (export "handler") (param i32 i32) (result i32)
                    i32.const {RESPONSE_PTR}
                )
            )"#
        )
    }

    fn fuel_exhaustion_module() -> &'static str {
        r#"(module
            (memory (export "memory") 1)
            (func (export "handler") (param i32 i32) (result i32)
                (loop $forever
                    br $forever
                )
                unreachable
            )
        )"#
    }

    #[test]
    fn runtime_dispatches_function() {
        let store = InMemoryFunctionStore::new();
        let wasm_path = format!("{}/tests/fixtures/hello.wasm", env!("CARGO_MANIFEST_DIR"));
        let func = FunctionMeta {
            subdomain: None,
            id: "hello".into(),
            route: "/hello".into(),
            wasm_path,
        };

        store.register(func).unwrap();

        let config = rune_core::RuntimeConfig {
            max_fuel: 1_000_000,
            max_memory_bytes: 64 * 1024 * 1024,
        };
        let runtime = Runtime::new(Arc::new(store), config).unwrap();

        let req = CoreRequest {
            method: "GET".into(),
            path: "/hello".into(),
            headers: Default::default(),
            body: vec![],
        };

        let res = runtime.handle_request(req).unwrap();
        assert_eq!(res.status, 200);
        assert_eq!(res.body, b"hello");
    }

    #[test]
    fn runtime_maps_fuel_exhaustion_to_out_of_fuel() {
        let (_temp_dir, runtime) = runtime_with_module(
            fuel_exhaustion_module(),
            RuntimeConfig {
                max_fuel: 10_000,
                max_memory_bytes: 64 * 1024 * 1024,
            },
        );

        let err = runtime.handle_request(request()).unwrap_err();
        assert!(matches!(err, RuneError::OutOfFuel));
    }

    #[test]
    fn runtime_rejects_invalid_json_response() {
        let (_temp_dir, runtime) =
            runtime_with_module(&static_response_module(b"not-json", 1), default_config());

        let err = runtime.handle_request(request()).unwrap_err();
        match err {
            RuneError::ExecutionError(message) => {
                assert!(message.contains("function 'fixture'"));
                assert!(message.contains("failed to parse JSON response"));
            }
            other => panic!("expected execution error, got {other:?}"),
        }
    }

    #[test]
    fn runtime_rejects_invalid_http_status() {
        let response = br#"{"status":999,"body":[]}"#;
        let (_temp_dir, runtime) =
            runtime_with_module(&static_response_module(response, 1), default_config());

        let err = runtime.handle_request(request()).unwrap_err();
        match err {
            RuneError::ExecutionError(message) => {
                assert!(message.contains("function 'fixture'"));
                assert!(message.contains("invalid HTTP status 999"));
            }
            other => panic!("expected execution error, got {other:?}"),
        }
    }

    #[test]
    fn runtime_enforces_memory_limit_from_config() {
        let response = br#"{"status":200,"body":[111,107]}"#;
        let module = static_response_module(response, 2);

        let (_temp_dir, runtime) = runtime_with_module(
            &module,
            RuntimeConfig {
                max_fuel: 1_000_000,
                max_memory_bytes: 2 * 65536,
            },
        );
        let ok = runtime.handle_request(request()).unwrap();
        assert_eq!(ok.status, 200);
        assert_eq!(ok.body, b"ok");

        let (_temp_dir, runtime) = runtime_with_module(
            &module,
            RuntimeConfig {
                max_fuel: 1_000_000,
                max_memory_bytes: 65536,
            },
        );
        let err = runtime.handle_request(request()).unwrap_err();
        match err {
            RuneError::ExecutionError(message) => {
                assert!(message.contains("function 'fixture'"));
            }
            other => panic!("expected execution error, got {other:?}"),
        }
    }
}
