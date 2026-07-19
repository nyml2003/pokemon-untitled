//! JSON Lines command boundary for non-window map editor clients.

#![forbid(unsafe_code)]

use map_editor_core::{
    EditorEffect, EditorModel, EditorVirtualCommand, EditorVirtualCommandError,
    EditorVirtualCommandResult,
};
use serde::Serialize;

#[derive(Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum CliResponse {
    Result(EditorVirtualCommandResult),
    Error { message: String },
}

pub fn execute(
    model: &EditorModel,
    command: EditorVirtualCommand,
) -> Result<(EditorModel, CliResponse), EditorVirtualCommandError> {
    let (model, result) = model.execute_virtual_command(command)?;
    Ok((model, CliResponse::Result(result)))
}

pub fn save_requested(response: &CliResponse) -> bool {
    matches!(
        response,
        CliResponse::Result(EditorVirtualCommandResult::Effect(
            EditorEffect::SaveRequested
        ))
    )
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
