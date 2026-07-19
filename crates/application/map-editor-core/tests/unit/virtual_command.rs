use map_project::{CompositeTile, MapProject, MapProjectId};
use map_tile_semantics::{
    FORMAT_VERSION, Neighbours8, TileDefinition, TileHardRules, TileSemanticsCatalog, TileStatus,
};

use super::*;

fn model() -> EditorModel {
    let grass = AtomicTileId::new("grass").unwrap();
    let rock = AtomicTileId::new("rock").unwrap();
    let base = CompositeTile::new(CompositeTileId::new("base").unwrap(), vec![grass.clone()]);
    EditorModel::new(
        MapProject::blank(MapProjectId::new("test").unwrap(), 3, 2, Some(base)),
        vec![grass, rock],
    )
}

fn model_with_semantics() -> EditorModel {
    let model = model();
    let semantics = TileSemanticsCatalog {
        format_version: FORMAT_VERSION.into(),
        tiles: vec![
            TileDefinition {
                id: AtomicTileId::new("grass").unwrap(),
                status: TileStatus::Approved {
                    tags: Default::default(),
                    rules: Box::new(TileHardRules {
                        stack: Vec::new(),
                        neighbours: Neighbours8::filled(map_tile_semantics::NeighbourRule::Any),
                    }),
                },
            },
            TileDefinition {
                id: AtomicTileId::new("rock").unwrap(),
                status: TileStatus::Blocked {
                    reason: "fixture".into(),
                },
            },
        ],
        patterns: Vec::new(),
    };
    EditorModel::with_semantics(model.project, model.atomic_ids, semantics)
}

#[test]
fn paints_batches_without_window_or_grid_coordinates() {
    let model = model();
    let (model, effect) = model
        .apply_virtual_command(EditorVirtualCommand::PaintCollision {
            cells: vec![TilePosition::new(0, 0), TilePosition::new(2, 1)],
            collision: Collision::Blocked,
        })
        .unwrap();
    assert_eq!(effect, EditorEffect::None);
    assert_eq!(
        model
            .project
            .cell(TilePosition::new(0, 0))
            .unwrap()
            .collision,
        Collision::Blocked
    );
    assert_eq!(
        model
            .project
            .cell(TilePosition::new(2, 1))
            .unwrap()
            .collision,
        Collision::Blocked
    );

    let (model, _) = model
        .apply_virtual_command(EditorVirtualCommand::PaintEvent {
            cells: vec![TilePosition::new(1, 0)],
            event: Some(MapEventKind::Encounter),
        })
        .unwrap();
    assert_eq!(
        model.project.cell(TilePosition::new(1, 0)).unwrap().event,
        Some(MapEventKind::Encounter)
    );
}

#[test]
fn resolves_resources_by_id_and_keeps_save_as_an_effect() {
    let model = model();
    let (model, _) = model
        .apply_virtual_command(EditorVirtualCommand::AppendAtomicLayer {
            material: CompositeTileId::new("base").unwrap(),
            tile: AtomicTileId::new("rock").unwrap(),
        })
        .unwrap();
    assert_eq!(model.project.materials.len(), 2);
    assert_eq!(model.project.materials[1].layers.len(), 2);
    let (_, effect) = model
        .apply_virtual_command(EditorVirtualCommand::Save)
        .unwrap();
    assert_eq!(effect, EditorEffect::SaveRequested);
}

#[test]
fn creates_a_named_composition_for_non_window_clients() {
    let model = model();
    let material = CompositeTile::new(
        CompositeTileId::new("tall-grass").unwrap(),
        vec![
            AtomicTileId::new("grass").unwrap(),
            AtomicTileId::new("rock").unwrap(),
        ],
    );
    let (model, _) = model
        .apply_virtual_command(EditorVirtualCommand::CreateMaterial {
            material: material.clone(),
        })
        .unwrap();
    assert_eq!(model.project.material(&material.id), Some(&material));
    assert!(matches!(
        model.apply_virtual_command(EditorVirtualCommand::CreateMaterial { material }),
        Err(EditorVirtualCommandError::Map(MapError::DuplicateMaterial(
            _
        )))
    ));
}

#[test]
fn returns_semantic_diagnostics_without_mutating_the_editor() {
    let semantic_model = model_with_semantics();
    let (next, result) = semantic_model
        .execute_virtual_command(EditorVirtualCommand::ValidateSemantics)
        .unwrap();
    assert_eq!(next.project, semantic_model.project);
    assert!(
        matches!(result, EditorVirtualCommandResult::Diagnostics(ref diagnostics) if diagnostics.is_empty())
    );

    let (_, result) = semantic_model
        .execute_virtual_command(EditorVirtualCommand::PaintVisual {
            cells: vec![TilePosition::new(0, 0)],
            material: Some(CompositeTileId::new("base").unwrap()),
        })
        .unwrap();
    assert!(matches!(
        result,
        EditorVirtualCommandResult::Effect(EditorEffect::None)
    ));

    assert!(matches!(
        model().execute_virtual_command(EditorVirtualCommand::ValidateSemantics),
        Err(EditorVirtualCommandError::SemanticCatalogUnavailable)
    ));
}

#[test]
fn rejects_invalid_agent_commands_before_reducing() {
    let model = model();
    assert!(matches!(
        model.apply_virtual_command(EditorVirtualCommand::PaintVisual {
            cells: Vec::new(),
            material: Some(CompositeTileId::new("base").unwrap()),
        }),
        Err(EditorVirtualCommandError::EmptyCells)
    ));
    assert!(matches!(
        model.apply_virtual_command(EditorVirtualCommand::PaintCollision {
            cells: vec![TilePosition::new(9, 9)],
            collision: Collision::Blocked,
        }),
        Err(EditorVirtualCommandError::CellOutOfBounds(_))
    ));
    assert!(matches!(
        model.apply_virtual_command(EditorVirtualCommand::SelectAtomic {
            tile: AtomicTileId::new("missing").unwrap(),
        }),
        Err(EditorVirtualCommandError::UnknownAtomicTile(_))
    ));
}

#[test]
fn inspect_returns_a_read_only_agent_snapshot() {
    let model = model();
    let (unchanged, result) = model
        .execute_virtual_command(EditorVirtualCommand::Inspect)
        .unwrap();
    let EditorVirtualCommandResult::State(state) = result else {
        panic!("inspect must return state");
    };
    assert_eq!(unchanged.project, model.project);
    assert_eq!(state.project.id.as_str(), "test");
    assert_eq!(state.project.width, 3);
    assert_eq!(state.project.height, 2);
    assert_eq!(state.atomic_tiles.len(), 2);
    assert_eq!(state.selected_atomic.unwrap().as_str(), "grass");
    assert_eq!(state.selected_material.unwrap().as_str(), "base");
    assert!(!state.dirty);
}
