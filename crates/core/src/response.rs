pub struct CoreResponse {
    pub status: u16,
    pub headers: crate::headers::Headers,
    pub body: Vec<u8>,
}
