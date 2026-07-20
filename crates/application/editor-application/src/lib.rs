//! Shared pure protocol for editor applications and structured callers.

#![forbid(unsafe_code)]

mod call;
mod diagnostic;
mod document;
mod port;

pub use call::{
    EDITOR_PROTOCOL_VERSION, EditorCall, EditorOperation, EditorProtocolError, EditorResponse,
};
pub use diagnostic::EditorDiagnostic;
pub use document::{EditorDocumentId, EditorKind};
pub use port::{EditorCore, EditorDocumentStore};

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
