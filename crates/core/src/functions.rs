use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionMeta {
    pub id: String,
    pub subdomain: Option<String>,
    pub route: String,
    pub wasm_path: String,
}
