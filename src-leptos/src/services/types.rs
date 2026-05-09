#[derive(serde::Serialize)]
pub struct LoginBody<'a> {
    pub key: &'a str,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct FunctionRecord {
    pub id: String,
    pub route: String,
    pub subdomain: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub created_at: String,
    pub revoked_at: Option<String>,
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
