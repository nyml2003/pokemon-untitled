use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EditorDiagnostic {
    code: String,
    target: Option<String>,
    message: String,
}

impl EditorDiagnostic {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            target: None,
            message: message.into(),
        }
    }

    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn target(&self) -> Option<&str> {
        self.target.as_deref()
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}
