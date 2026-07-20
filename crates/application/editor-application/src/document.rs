use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EditorKind {
    Map,
    Trainer,
    Pokemon,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct EditorDocumentId(String);

impl EditorDocumentId {
    pub fn new(value: impl Into<String>) -> Result<Self, EditorDocumentIdError> {
        let value = value.into();
        if value.is_empty() || value.len() > 96 {
            return Err(EditorDocumentIdError::InvalidLength(value.len()));
        }
        if !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        }) {
            return Err(EditorDocumentIdError::InvalidCharacter);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditorDocumentIdError {
    InvalidLength(usize),
    InvalidCharacter,
}

impl fmt::Display for EditorDocumentIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength(length) => write!(formatter, "invalid editor document ID length: {length}"),
            Self::InvalidCharacter => formatter.write_str("editor document ID must contain only lowercase ASCII letters, digits, hyphens, or underscores"),
        }
    }
}

impl std::error::Error for EditorDocumentIdError {}

impl<'de> Deserialize<'de> for EditorDocumentId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}
