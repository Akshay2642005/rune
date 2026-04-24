use serde::Serialize;

#[derive(Serialize)]
pub struct CoreRequest {
    pub method: String,
    pub path: String,
    pub headers: crate::Headers,
    #[serde(with = "crate::base64_serde")]
    pub body: Vec<u8>,
}
