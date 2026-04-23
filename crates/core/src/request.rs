pub struct CoreRequest {
    pub method: String,
    pub path: String,
    pub headers: crate::headers::Headers,
    pub body: Vec<u8>,
}
