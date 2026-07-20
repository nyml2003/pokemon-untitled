use crate::{EditorDiagnostic, EditorDocumentId, EditorKind};

/// Pure editor state consumed equally by GUI, CLI, and structured callers.
pub trait EditorCore: Sized {
    type Command;
    type Snapshot;
    type Error;

    fn inspect(&self) -> Self::Snapshot;
    fn validate(&self) -> Vec<EditorDiagnostic>;
    fn transition(self, command: Self::Command) -> Result<Self, Self::Error>;
    fn is_dirty(&self) -> bool;
}

/// Runtime-owned persistence boundary for a validated editor document.
pub trait EditorDocumentStore<Document> {
    type Error;

    fn load(&self, kind: EditorKind, document: &EditorDocumentId) -> Result<Document, Self::Error>;
    fn save(
        &self,
        kind: EditorKind,
        document: &EditorDocumentId,
        value: &Document,
    ) -> Result<(), Self::Error>;
}
