use crate::{FunctionMeta, RuneError};

/// Hot-path function registry used by the runtime.
///
/// All implementations must be internally synchronised (`Send + Sync`)
/// because the trait is called from async Tokio tasks via
/// `spawn_blocking`.  Write operations (register / remove) are
/// infrequent; read operations (get_by_*) are on every request.
pub trait FunctionStore: Send + Sync {
    /// Look up a function by its URL path (e.g. `/hello`).
    fn get_by_route(&self, route: &str) -> Result<Option<FunctionMeta>, RuneError>;

    /// Look up a function by its subdomain label (e.g. `hello` for
    /// `hello.yourdomain.com`).
    fn get_by_subdomain(&self, subdomain: &str) -> Result<Option<FunctionMeta>, RuneError>;

    /// Register or replace a function.
    ///
    /// If a function with the same `id` already exists it is replaced.
    /// If a *different* function owns the same `route` or `subdomain`
    /// the implementation must return `RuneError::DuplicateRoute`.
    fn register(&self, func: FunctionMeta) -> Result<(), RuneError>;

    /// Remove a function by id.  Returns `RuneError::NotFound` when
    /// no function with that id exists.
    fn remove(&self, id: &str) -> Result<(), RuneError>;

    /// Return all registered functions (used by the control plane API).
    fn list(&self) -> Result<Vec<FunctionMeta>, RuneError>;
}

