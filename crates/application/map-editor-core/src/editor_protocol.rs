use std::{error::Error, fmt};

use editor_application::{EditorCall, EditorKind, EditorOperation};

use crate::EditorVirtualCommand;

/// Converts a routed logical editor call into the map editor's typed reducer command.
pub fn virtual_command_from_editor_call(
    call: &EditorCall,
) -> Result<EditorVirtualCommand, MapEditorProtocolError> {
    call.validate()
        .map_err(|error| MapEditorProtocolError::Protocol(error.to_string()))?;
    if call.kind() != EditorKind::Map {
        return Err(MapEditorProtocolError::WrongEditorKind);
    }
    match call.operation() {
        EditorOperation::Inspect => Ok(EditorVirtualCommand::Inspect),
        EditorOperation::Validate => Ok(EditorVirtualCommand::ValidateSemantics),
        EditorOperation::Command => serde_json::from_value(call.payload().clone())
            .map_err(|error| MapEditorProtocolError::Command(error.to_string())),
        EditorOperation::Save => Ok(EditorVirtualCommand::Save),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MapEditorProtocolError {
    Protocol(String),
    WrongEditorKind,
    Command(String),
}

impl fmt::Display for MapEditorProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(error) => write!(formatter, "invalid editor protocol call: {error}"),
            Self::WrongEditorKind => formatter.write_str("map editor requires a map editor call"),
            Self::Command(error) => {
                write!(formatter, "invalid map editor command payload: {error}")
            }
        }
    }
}

impl Error for MapEditorProtocolError {}

#[cfg(test)]
mod tests {
    use editor_application::{EditorCall, EditorDocumentId};
    use serde_json::json;

    use super::*;

    #[test]
    fn structured_map_call_decodes_into_the_existing_virtual_command()
    -> Result<(), MapEditorProtocolError> {
        let call = EditorCall::new(
            EditorKind::Map,
            EditorDocumentId::new("verdant-route")
                .map_err(|error| MapEditorProtocolError::Protocol(error.to_string()))?,
            EditorOperation::Command,
            json!("Undo"),
        )
        .map_err(|error| MapEditorProtocolError::Protocol(error.to_string()))?;
        assert_eq!(
            virtual_command_from_editor_call(&call)?,
            EditorVirtualCommand::Undo
        );
        Ok(())
    }
}
