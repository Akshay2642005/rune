pub type HeaderName = String;
pub type HeaderValue = String;

#[derive(Clone, Debug, Default)]
pub struct Headers {
    inner: Vec<(HeaderName, HeaderValue)>,
}

impl Headers {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn insert(&mut self, name: HeaderName, value: HeaderValue) {
        self.inner.push((normalize(name), value));
    }

    pub fn get_all(&self, name: &str) -> Vec<&str> {
        let name = normalize(name.to_string());
        self.inner
            .iter()
            .filter(|(k, _)| *k == name)
            .map(|(_, v)| v.as_str())
            .collect()
    }
    pub fn get(&self, name: &str) -> Option<&str> {
        self.get_all(name).into_iter().next()
    }
}

fn normalize(name: String) -> String {
    name.to_ascii_lowercase()
}
