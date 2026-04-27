use std::collections::HashMap;
use std::sync::RwLock;

use rune_core::{FunctionMeta, FunctionStore, RuneError};

/// Thread-safe in-memory function registry.
///
/// Used as the hot-path read cache in the server and as the sole
/// store in tests.  `id` is the primary key; `route` and `subdomain`
/// are maintained as secondary indexes for O(1) lookup.
pub struct InMemoryFunctionStore {
    inner: RwLock<StoreInner>,
}

#[derive(Default)]
struct StoreInner {
    /// Primary storage keyed by function id.
    by_id: HashMap<String, FunctionMeta>,
    /// Secondary index: route → id.
    route_index: HashMap<String, String>,
    /// Secondary index: subdomain → id.
    subdomain_index: HashMap<String, String>,
}

impl InMemoryFunctionStore {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(StoreInner::default()),
        }
    }

    fn write_lock(&self) -> Result<std::sync::RwLockWriteGuard<'_, StoreInner>, RuneError> {
        self.inner
            .write()
            .map_err(|_| RuneError::InternalError("store lock poisoned (write)".into()))
    }

    fn read_lock(&self) -> Result<std::sync::RwLockReadGuard<'_, StoreInner>, RuneError> {
        self.inner
            .read()
            .map_err(|_| RuneError::InternalError("store lock poisoned (read)".into()))
    }
}

impl Default for InMemoryFunctionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl FunctionStore for InMemoryFunctionStore {
    fn get_by_route(&self, route: &str) -> Result<Option<FunctionMeta>, RuneError> {
        let g = self.read_lock()?;
        let meta = g
            .route_index
            .get(route)
            .and_then(|id| g.by_id.get(id))
            .cloned();
        Ok(meta)
    }

    fn get_by_subdomain(&self, subdomain: &str) -> Result<Option<FunctionMeta>, RuneError> {
        let g = self.read_lock()?;
        let meta = g
            .subdomain_index
            .get(subdomain)
            .and_then(|id| g.by_id.get(id))
            .cloned();
        Ok(meta)
    }

    fn register(&self, meta: FunctionMeta) -> Result<(), RuneError> {
        let mut g = self.write_lock()?;

        // Check route collision (different function owns this route).
        if let Some(owner_id) = g.route_index.get(&meta.route) {
            if owner_id != &meta.id {
                return Err(RuneError::DuplicateIdentifier {
                    field: "route".to_string(),
                    value: meta.route.clone(),
                });
            }
        }

        // Check subdomain collision.
        if let Some(sub) = &meta.subdomain {
            if let Some(owner_id) = g.subdomain_index.get(sub) {
                if owner_id != &meta.id {
                    return Err(RuneError::DuplicateIdentifier {
                        field: "subdomain".to_string(),
                        value: sub.clone(),
                    });
                }
            }
        }

        // Remove stale indexes if this id already exists.
        if let Some(old) = g.by_id.get(&meta.id) {
            let old_route = old.route.clone();
            let old_sub = old.subdomain.clone();
            g.route_index.remove(&old_route);
            if let Some(old_sub) = old_sub {
                g.subdomain_index.remove(&old_sub);
            }
        }

        // Insert new indexes.
        g.route_index.insert(meta.route.clone(), meta.id.clone());
        if let Some(sub) = &meta.subdomain {
            g.subdomain_index.insert(sub.clone(), meta.id.clone());
        }
        g.by_id.insert(meta.id.clone(), meta);

        Ok(())
    }

    fn remove(&self, id: &str) -> Result<(), RuneError> {
        let mut g = self.write_lock()?;
        let meta = g.by_id.remove(id).ok_or(RuneError::NotFound)?;
        g.route_index.remove(&meta.route);
        if let Some(sub) = &meta.subdomain {
            g.subdomain_index.remove(sub);
        }
        Ok(())
    }

    fn list(&self) -> Result<Vec<FunctionMeta>, RuneError> {
        let g = self.read_lock()?;
        Ok(g.by_id.values().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(id: &str, route: &str, subdomain: Option<&str>) -> FunctionMeta {
        FunctionMeta {
            id: id.into(),
            subdomain: subdomain.map(Into::into),
            route: route.into(),
            wasm_path: format!("{id}.wasm"),
        }
    }

    #[test]
    fn register_and_get_by_route() {
        let s = InMemoryFunctionStore::new();
        s.register(meta("hello", "/hello", None)).unwrap();
        assert!(s.get_by_route("/hello").unwrap().is_some());
    }

    #[test]
    fn register_and_get_by_subdomain() {
        let s = InMemoryFunctionStore::new();
        s.register(meta("hello", "/hello", Some("hello"))).unwrap();
        let found = s.get_by_subdomain("hello").unwrap().unwrap();
        assert_eq!(found.id, "hello");
    }

    #[test]
    fn register_replaces_same_id() {
        let s = InMemoryFunctionStore::new();
        s.register(meta("hello", "/hello", None)).unwrap();
        s.register(meta("hello", "/v2/hello", Some("hello-v2")))
            .unwrap();

        assert!(s.get_by_route("/hello").unwrap().is_none());
        assert!(s.get_by_route("/v2/hello").unwrap().is_some());
        assert_eq!(s.list().unwrap().len(), 1);
    }

    #[test]
    fn duplicate_route_different_id_rejected() {
        let s = InMemoryFunctionStore::new();
        s.register(meta("a", "/hello", None)).unwrap();
        let err = s.register(meta("b", "/hello", None)).unwrap_err();
        assert!(matches!(err, RuneError::DuplicateIdentifier { .. }));
    }

    #[test]
    fn remove_cleans_indexes() {
        let s = InMemoryFunctionStore::new();
        s.register(meta("hello", "/hello", Some("hello"))).unwrap();
        s.remove("hello").unwrap();
        assert!(s.get_by_route("/hello").unwrap().is_none());
        assert!(s.get_by_subdomain("hello").unwrap().is_none());
        assert_eq!(s.list().unwrap().len(), 0);
    }

    #[test]
    fn remove_unknown_returns_not_found() {
        let s = InMemoryFunctionStore::new();
        assert!(matches!(s.remove("ghost"), Err(RuneError::NotFound)));
    }

    #[test]
    fn concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let s = Arc::new(InMemoryFunctionStore::new());
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let s = Arc::clone(&s);
                thread::spawn(move || {
                    s.register(meta(
                        &format!("fn{i}"),
                        &format!("/route{i}"),
                        Some(&format!("fn{i}")),
                    ))
                    .unwrap();
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(s.list().unwrap().len(), 10);
    }
}
