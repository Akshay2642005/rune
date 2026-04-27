#[derive(Debug)]
pub enum RuneError {
    NotFound,
    ExecutionError(String),
    Timeout,
    OutOfFuel,
    InvalidRequest(String),
    DuplicateIdentifier { field: String, value: String },
    InternalError(String),
}

impl std::fmt::Display for RuneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => f.write_str("not found"),
            Self::ExecutionError(msg) => f.write_str(&format!("execution error: {}", msg)),
            Self::Timeout => f.write_str("request timed out"),
            Self::OutOfFuel => f.write_str("out of fuel"),
            Self::InvalidRequest(msg) => f.write_str(&format!("invalid request: {}", msg)),
            Self::DuplicateIdentifier { field, value } => {
                f.write_str(&format!("duplicate {field}: {value}"))
            }
            Self::InternalError(msg) => f.write_str(&format!("internal error: {}", msg)),
        }
    }
}

impl std::error::Error for RuneError {}
