#[derive(Debug)]
pub enum RuneError {
    NotFound,
    ExecutionError(String),
    Timeout,
    OutOfFuel,
    InvalidRequest(String),
    InternalError(String),
}
