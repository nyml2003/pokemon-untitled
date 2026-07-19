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
mod tests {
    use map_editor_core::EditorVirtualCommandResult;
    use map_project::{AtomicTileId, CompositeTile, CompositeTileId, MapProject, MapProjectId};

    use super::*;

    #[test]
    fn inspect_is_a_serializable_cli_result() {
        let tile = AtomicTileId::new("tile").unwrap();
        let model = EditorModel::new(
            MapProject::blank(
                MapProjectId::new("map").unwrap(),
                1,
                1,
                Some(CompositeTile::new(
                    CompositeTileId::new("base").unwrap(),
                    vec![tile.clone()],
                )),
            ),
            vec![tile],
        );
        let (_, response) = execute(&model, EditorVirtualCommand::Inspect).unwrap();
        assert!(matches!(
            &response,
            CliResponse::Result(EditorVirtualCommandResult::State(_))
        ));
        assert!(
            serde_json::to_string(&response)
                .unwrap()
                .contains("project")
        );
    }
}
