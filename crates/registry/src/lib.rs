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
        if map.contains_key(&meta.route) {
            return Err(rune_core::RuneError::DuplicateRoute {
                route: meta.route.clone(),
            });
        }
        map.insert(meta.route.clone(), meta);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

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

    #[test]
    fn get_unknown_route_returns_none() {
        let store = InMemoryFunctionStore::new();
        let result = store.get_by_route("/unknown").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn duplicate_registration_returns_error() {
        let store = InMemoryFunctionStore::new();

        let func1 = rune_core::FunctionMeta {
            id: "hello1".to_string(),
            route: "/hello".to_string(),
            wasm_path: "hello1.wasm".to_string(),
        };
        store.register(func1).unwrap();

        let func2 = rune_core::FunctionMeta {
            id: "hello2".to_string(),
            route: "/hello".to_string(),
            wasm_path: "hello2.wasm".to_string(),
        };
        let result = store.register(func2);
        assert!(result.is_err());
        match result {
            Err(rune_core::RuneError::DuplicateRoute { route }) => {
                assert_eq!(route, "/hello");
            }
            _ => panic!("Expected DuplicateRoute error"),
        }
    }

    #[test]
    fn concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let store = Arc::new(InMemoryFunctionStore::new());
        let mut handles = vec![];

        for i in 0..10 {
            let store_clone = Arc::clone(&store);
            let handle = thread::spawn(move || {
                let func = rune_core::FunctionMeta {
                    id: format!("func{}", i),
                    route: format!("/route{}", i),
                    wasm_path: format!("func{}.wasm", i),
                };
                store_clone.register(func).unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        for i in 0..10 {
            let result = store.get_by_route(&format!("/route{}", i)).unwrap();
            assert!(result.is_some());
            assert_eq!(result.unwrap().id, format!("func{}", i));
        }
    }
}
