#[derive(serde::Serialize)]
pub struct LoginBody<'a> {
    pub key: &'a str,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct FunctionRecord {
    pub id: String,
    pub route: String,
    pub subdomain: Option<String>,
    pub wasm_path: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreatedKey {
    pub id: String,
    pub raw: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct FunctionDeployRequest {
    pub id: String,
    pub route: String,
    pub subdomain: Option<String>,
    pub wasm_bytes: Vec<u8>,
}
