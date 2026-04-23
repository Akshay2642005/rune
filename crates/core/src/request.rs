pub struct CoreRequest {
    pub method: String,
    pub path: String,
    pub headers: crate::Headers
    pub body: Vec<u8>,
}

