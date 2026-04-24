use serde::{Deserialize, Serialize};

pub type HeaderName = String;
pub type HeaderValue = String;

#[derive(Clone, Debug, Default, Serialize)]
#[serde(transparent)]
pub struct Headers {
    inner: Vec<(HeaderName, HeaderValue)>,
}

impl<'de> Deserialize<'de> for Headers {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw: Vec<(String, String)> = Vec::deserialize(deserializer)?;
        Ok(Headers::from(raw))
    }
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

impl From<Vec<(String, String)>> for Headers {
    fn from(headers: Vec<(String, String)>) -> Self {
        let mut h = Headers::new();

        for (k, v) in headers {
            h.insert(k, v);
        }
        h
    }
}

fn normalize(name: String) -> String {
    name.to_ascii_lowercase()
}
