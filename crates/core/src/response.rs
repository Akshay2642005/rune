use serde::Deserialize;

pub struct CoreResponse {
    pub status: u16,
    pub headers: crate::headers::Headers,
    pub body: Vec<u8>,
}

#[derive(Deserialize)]
pub struct WasmResponse {
    pub status: u16,
    pub body: String,
}
