pub struct CoreResponse {
    pub status: u16,
    pub headers: crate::Headers,
    pub body: Vec<u8>,
}
