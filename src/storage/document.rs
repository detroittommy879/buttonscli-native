use serde_json::{Map, Value};

#[derive(Debug)]
pub enum DocumentError {
    InvalidJson,
    NotObject,
}

pub(crate) struct LegacyDocument {
    pub fields: Map<String, Value>,
}

impl LegacyDocument {
    pub fn parse(raw: &[u8]) -> Result<Self, DocumentError> {
        let value: Value = serde_json::from_slice(raw).map_err(|_| DocumentError::InvalidJson)?;
        let fields = value.as_object().ok_or(DocumentError::NotObject)?.clone();
        Ok(Self { fields })
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.fields.get(key)
    }
}
