use crate::{FunctionMeta, RuneError};

pub trait FunctionStore: Send + Sync {
    fn get_by_route(&self, route: &str) -> Result<Option<FunctionMeta>, RuneError>;
    fn register(&mut self, func: FunctionMeta) -> Result<(), RuneError>;
}
