use crate::{FunctionMeta, RuneError};

pub trait FunctionStore: Send + Sync {
    fn get_by_route(&self, route: &str) -> Result<Option<FunctionMeta>, RuneError>;
    /// Register a function.
    ///
    /// Implementations must be internally synchronized (e.g., Mutex/RwLock),
    /// as this method takes `&self` and may be called concurrently.
    fn register(&self, func: FunctionMeta) -> Result<(), RuneError>;
}
