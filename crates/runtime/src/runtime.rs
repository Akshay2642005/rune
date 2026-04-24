use std::sync::Arc;

pub struct Runtime {
    store: Arc<dyn rune_core::FunctionStore>,
    executor: crate::executor::WasmExecutor,
}

impl Runtime {
    pub fn new(store: Arc<dyn rune_core::FunctionStore>) -> Self {
        let executor = crate::executor::WasmExecutor::new().unwrap();
        Self { store, executor }
    }

    pub fn handle_request(
        &self,
        req: rune_core::CoreRequest,
    ) -> Result<rune_core::CoreResponse, rune_core::RuneError> {
        let func = self
            .store
            .get_by_route(&req.path)?
            .ok_or(rune_core::RuneError::NotFound)?;

        let resp_bytes = self
            .executor
            .execute(&func.wasm_path)
            .map_err(|e| rune_core::RuneError::ExecutionError(e.to_string()))?;

        if resp_bytes.is_empty() {
            return Err(rune_core::RuneError::ExecutionError(
                "empty response".into(),
            ));
        }

        let wasm_resp: rune_core::WasmResponse = serde_json::from_slice(&resp_bytes)
            .map_err(|e| rune_core::RuneError::ExecutionError(e.to_string()))?;

        Ok(rune_core::CoreResponse {
            status: wasm_resp.status,
            headers: Default::default(),
            body: wasm_resp.body.into_bytes(),
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

        let runtime = Runtime::new(Arc::new(store));

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
