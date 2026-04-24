use rune_core::FunctionStore;

pub struct InMemoryFunctionStore {
    functions: std::sync::RwLock<std::collections::HashMap<String, rune_core::FunctionMeta>>,
}

impl InMemoryFunctionStore {
    pub fn new() -> Self {
        Self {
            functions: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for InMemoryFunctionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl FunctionStore for InMemoryFunctionStore {
    fn get_by_route(
        &self,
        route: &str,
    ) -> Result<Option<rune_core::FunctionMeta>, rune_core::RuneError> {
        let map = self.functions.read().map_err(|_| {
            rune_core::RuneError::InternalError(
                "function store lock poisoned while reading".to_string(),
            )
        })?;
        Ok(map.get(route).cloned())
    }

    fn register(&self, meta: rune_core::FunctionMeta) -> Result<(), rune_core::RuneError> {
        let mut map = self.functions.write().map_err(|_| {
            rune_core::RuneError::InternalError(
                "function store lock poisoned while writing".to_string(),
            )
        })?;
        map.insert(meta.route.clone(), meta);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_get_function() {
        let store = InMemoryFunctionStore::new();

        let func = rune_core::FunctionMeta {
            id: "hello".to_string(),
            route: "/hello".to_string(),
            wasm_path: "hello.wasm".to_string(),
        };
        store.register(func).unwrap();

        let found = store.get_by_route("/hello").unwrap();
        assert!(found.is_some());
    }
}
