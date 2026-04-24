use serde::Deserialize;

#[derive(Debug)]
pub struct CoreResponse {
    pub status: u16,
    pub headers: crate::Headers,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WasmResponse {
    pub status: u16,
    pub headers: Option<Vec<(String, String)>>,
    #[serde(with = "serde_bytes")]
    pub body: Vec<u8>,
}
