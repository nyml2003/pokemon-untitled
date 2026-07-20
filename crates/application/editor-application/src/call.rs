use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{EditorDiagnostic, EditorDocumentId, EditorKind};

pub const EDITOR_PROTOCOL_VERSION: &str = "editor-v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EditorOperation {
    Inspect,
    Validate,
    Command,
    Save,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EditorCall {
    protocol_version: String,
    kind: EditorKind,
    document: EditorDocumentId,
    operation: EditorOperation,
    payload: Value,
}

impl EditorCall {
    pub fn new(
        kind: EditorKind,
        document: EditorDocumentId,
        operation: EditorOperation,
        payload: Value,
    ) -> Result<Self, EditorProtocolError> {
        if !matches!(operation, EditorOperation::Command) && !payload.is_null() {
            return Err(EditorProtocolError::UnexpectedPayload(operation));
        }
        Ok(Self {
            protocol_version: EDITOR_PROTOCOL_VERSION.to_owned(),
            kind,
            document,
            operation,
            payload,
        })
    }

    pub fn from_json(json: &str) -> Result<Self, EditorProtocolError> {
        let call = serde_json::from_str::<Self>(json)
            .map_err(|error| EditorProtocolError::Json(error.to_string()))?;
        call.validate()?;
        Ok(call)
    }

    pub fn validate(&self) -> Result<(), EditorProtocolError> {
        if self.protocol_version != EDITOR_PROTOCOL_VERSION {
            return Err(EditorProtocolError::UnsupportedVersion(
                self.protocol_version.clone(),
            ));
        }
        if !matches!(self.operation, EditorOperation::Command) && !self.payload.is_null() {
            return Err(EditorProtocolError::UnexpectedPayload(self.operation));
        }
        Ok(())
    }

    pub fn kind(&self) -> EditorKind {
        self.kind
    }

    pub fn document(&self) -> &EditorDocumentId {
        &self.document
    }

    pub fn operation(&self) -> EditorOperation {
        self.operation
    }

    pub fn payload(&self) -> &Value {
        &self.payload
    }

    pub fn to_json(&self) -> Result<String, EditorProtocolError> {
        serde_json::to_string(self).map_err(|error| EditorProtocolError::Json(error.to_string()))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "data")]
pub enum EditorResponse {
    Snapshot(Value),
    Diagnostics(Vec<EditorDiagnostic>),
    CommandApplied { dirty: bool },
    SaveRequested,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorProtocolError {
    Json(String),
    UnsupportedVersion(String),
    UnexpectedPayload(EditorOperation),
}

impl fmt::Display for EditorProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "editor protocol JSON error: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported editor protocol version: {version}")
            }
            Self::UnexpectedPayload(operation) => {
                write!(formatter, "{operation:?} does not accept a payload")
            }
        }
    }
}

impl std::error::Error for EditorProtocolError {}
