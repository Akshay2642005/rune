use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionMeta {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subdomain: Option<String>,
    pub route: String,
    pub wasm_path: String,
}
