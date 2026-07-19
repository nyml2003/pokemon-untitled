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
