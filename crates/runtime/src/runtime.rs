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
        let executor = crate::WasmExecutor::new(config.max_fuel)
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

        let input = serde_json::to_vec(&req)
            .map_err(|e| rune_core::RuneError::ExecutionError(e.to_string()))?;

        let resp_bytes = self
            .executor
            .execute(&func.wasm_path, &input)
            .map_err(|e| rune_core::RuneError::ExecutionError(e.to_string()))?;

        if resp_bytes.is_empty() {
            return Err(rune_core::RuneError::ExecutionError(
                "empty response".into(),
            ));
        }

        let wasm_resp: rune_core::WasmResponse = serde_json::from_slice(&resp_bytes).map_err(
            |e| {
                rune_core::RuneError::ExecutionError(format!(
                    "failed to parse JSON response from function '{}' (route '{}'): {}",
                    func.id, req.path, e
                ))
            },
        )?;

        if wasm_resp.status < 100 || wasm_resp.status > 599 {
            return Err(rune_core::RuneError::ExecutionError(format!(
                "invalid HTTP status {} from function '{}' (route '{}')",
                wasm_resp.status, func.id, req.path
            )));
        }

        Ok(rune_core::CoreResponse {
            status: wasm_resp.status,
            headers: wasm_resp.headers.unwrap_or_default().into(),
            body: wasm_resp.body,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use rune_core::{CoreRequest, FunctionMeta, FunctionStore};
    use rune_registry::InMemoryFunctionStore;

    #[test]
    fn runtime_dispatches_function() {
        let store = InMemoryFunctionStore::new();
        let wasm_path = format!("{}/tests/fixtures/hello.wasm", env!("CARGO_MANIFEST_DIR"));
        let func = FunctionMeta {
            id: "hello".into(),
            route: "/hello".into(),
            wasm_path,
        };

        store.register(func).unwrap();

        let config = rune_core::RuntimeConfig {
            max_fuel: 1_000_000,
            max_memory_bytes: 64 * 1024 * 1024,
            request_timeout_ms: 5000,
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
}