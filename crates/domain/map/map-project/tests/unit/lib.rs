use std::collections::BTreeSet;

use super::*;

fn id(value: &str) -> AtomicTileId {
    AtomicTileId::new(value).unwrap()
}

fn fixture() -> (MapProject, BTreeSet<AtomicTileId>) {
    let known = [id("tile-0000"), id("tile-0001")].into_iter().collect();
    let material = CompositeTile::new(
        CompositeTileId::new("ground").unwrap(),
        vec![id("tile-0000"), id("tile-0001")],
    );
    (
        MapProject::blank(MapProjectId::new("demo").unwrap(), 3, 2, Some(material)),
        known,
    )
}

#[test]
fn round_trips_the_versioned_json_document() {
    let (project, known) = fixture();
    let json = project.to_json_pretty(&known).unwrap();
    let decoded = MapProject::from_json(&json, &known).unwrap();
    assert_eq!(decoded, project);
    assert!(json.contains("gen3-map-v2"));
    assert!(json.contains("\"visual_cells\""));
    assert!(json.contains("\"collision_cells\""));
    assert!(json.contains("\"event_cells\""));
}

#[test]
fn accepts_an_unbounded_number_of_flat_layers() {
    let (mut project, mut known) = fixture();
    let layers = (0..4096)
        .map(|index| id(&format!("layer-{index}")))
        .inspect(|tile| {
            known.insert(tile.clone());
        })
        .collect();
    project.materials[0].layers = layers;
    project.validate(&known).unwrap();
}

#[test]
fn rejects_unknown_atomic_and_composite_references() {
    let (mut project, known) = fixture();
    project.materials[0].layers.push(id("missing"));
    assert!(matches!(
        project.validate(&known),
        Err(MapError::UnknownAtomicTile(_))
    ));

    project.materials[0].layers.pop();
    project.visual_cells[0].material = Some(CompositeTileId::new("missing").unwrap());
    assert!(matches!(
        project.validate(&known),
        Err(MapError::UnknownMaterial(_))
    ));
}

#[test]
fn undo_and_redo_restore_cell_edits() {
    let (project, _) = fixture();
    let position = TilePosition::new(1, 1);
    let before = project.cell(position).unwrap();
    let after = MapCell::new(None, Collision::Blocked, Some(MapEventKind::Encounter));
    let history = EditHistory::default();
    let (project, history) = history
        .execute(
            project,
            MapEditCommand::ReplaceCells(vec![CellChange {
                position,
                before: before.clone(),
                after: after.clone(),
            }]),
        )
        .unwrap();
    assert_eq!(project.cell(position), Some(after.clone()));
    let (project, history, changed) = history.undo(project).unwrap();
    assert!(changed);
    assert_eq!(project.cell(position), Some(before.clone()));
    let (project, _, changed) = history.redo(project).unwrap();
    assert!(changed);
    assert_eq!(project.cell(position), Some(after));
}

#[test]
fn validation_reports_every_schema_boundary() {
    assert!(AtomicTileId::new(" ").is_err());
    let (_, known) = fixture();
    let invalid = [
        {
            let (mut project, _) = fixture();
            project.format_version = "old".into();
            project
        },
        {
            let (mut project, _) = fixture();
            project.id = MapProjectId(" ".into());
            project
        },
        {
            let (mut project, _) = fixture();
            project.tile_size = TilePixelSize::new(0, 16);
            project
        },
        MapProject::blank(MapProjectId::new("empty").unwrap(), 0, 1, None),
        {
            let (mut project, _) = fixture();
            project.visual_cells.pop();
            project
        },
        {
            let (mut project, _) = fixture();
            project.collision_cells.pop();
            project
        },
        {
            let (mut project, _) = fixture();
            project.event_cells.pop();
            project
        },
        {
            let (mut project, _) = fixture();
            project.player_spawn = TilePosition::new(99, 99);
            project
        },
        {
            let (mut project, _) = fixture();
            project.materials.push(project.materials[0].clone());
            project
        },
        {
            let (mut project, _) = fixture();
            project.materials[0].id = CompositeTileId(" ".into());
            project
        },
        {
            let (mut project, _) = fixture();
            project.materials[0].layers.clear();
            project
        },
        {
            let (mut project, _) = fixture();
            project.materials[0].layers[0] = AtomicTileId(" ".into());
            project
        },
        {
            let (mut project, _) = fixture();
            let spawn = project.cell_index(project.player_spawn).unwrap();
            project.collision_cells[spawn] = Collision::Blocked;
            project
        },
    ];
    let expected = [
        "unsupported map format",
        "MapProjectId must not be empty",
        "tile size must be non-zero",
        "map width and height must be non-zero",
        "map layer visual_cells",
        "map layer collision_cells",
        "map layer event_cells",
        "spawn",
        "defined more than once",
        "CompositeTileId must not be empty",
        "has no layers",
        "AtomicTileId must not be empty",
        "is blocked",
    ];
    for (project, message) in invalid.into_iter().zip(expected) {
        assert!(
            project
                .validate(&known)
                .unwrap_err()
                .to_string()
                .contains(message)
        );
    }
    assert!(MapProject::from_json("not json", &known).is_err());
}

#[test]
fn material_commands_are_reversible_values() {
    let (project, _) = fixture();
    let history = EditHistory::default();
    let extra = CompositeTile::new(
        CompositeTileId::new("extra").unwrap(),
        vec![id("tile-0000")],
    );
    let (project, history) = history
        .execute(project, MapEditCommand::CreateMaterial(extra.clone()))
        .unwrap();
    assert!(history.is_dirty());
    assert_eq!(project.material(&extra.id), Some(&extra));
    let (project, history, changed) = history.undo(project).unwrap();
    assert!(changed);
    assert!(project.material(&extra.id).is_none());
    let (project, history, changed) = history.redo(project).unwrap();
    assert!(changed);
    assert_eq!(project.material(&extra.id), Some(&extra));

    let replacement = CompositeTile::new(extra.id.clone(), vec![id("tile-0001")]);
    let (project, history) = history
        .execute(
            project,
            MapEditCommand::ReplaceMaterial {
                before: extra.clone(),
                after: replacement.clone(),
            },
        )
        .unwrap();
    assert_eq!(project.material(&extra.id), Some(&replacement));
    let (project, history, _) = history.undo(project).unwrap();
    assert_eq!(project.material(&extra.id), Some(&extra));

    let (project, history) = history
        .execute(project, MapEditCommand::RemoveMaterial(extra.clone()))
        .unwrap();
    assert!(project.material(&extra.id).is_none());
    let (project, _, _) = history.undo(project).unwrap();
    assert_eq!(project.material(&extra.id), Some(&extra));
}

#[test]
fn failed_and_empty_history_transitions_are_explicit() {
    let (project, _) = fixture();
    let (project, history, changed) = EditHistory::default().undo(project).unwrap();
    assert!(!changed);
    assert!(!history.is_dirty());
    let (project, _, changed) = history.redo(project).unwrap();
    assert!(!changed);

    let outside = MapEditCommand::ReplaceCells(vec![CellChange {
        position: TilePosition::new(99, 99),
        before: MapCell::new(None, Collision::Walkable, None),
        after: MapCell::new(None, Collision::Blocked, None),
    }]);
    assert!(EditHistory::default().execute(project, outside).is_err());

    let (project, _) = fixture();
    let used = project.materials[0].clone();
    assert!(
        EditHistory::default()
            .execute(
                project.clone(),
                MapEditCommand::CreateMaterial(used.clone())
            )
            .is_err()
    );
    assert!(
        EditHistory::default()
            .execute(project, MapEditCommand::RemoveMaterial(used))
            .is_err()
    );

    let (project, _) = fixture();
    let missing = CompositeTile::new(
        CompositeTileId::new("missing").unwrap(),
        vec![id("tile-0000")],
    );
    assert!(
        EditHistory::default()
            .execute(
                project,
                MapEditCommand::ReplaceMaterial {
                    before: missing.clone(),
                    after: missing,
                },
            )
            .is_err()
    );
}

#[test]
fn every_map_error_has_actionable_text() {
    let material = CompositeTileId::new("material").unwrap();
    let tile = id("tile");
    let errors = [
        MapError::CellOutOfBounds(TilePosition::new(1, 2)),
        MapError::SpawnOutOfBounds(TilePosition::new(1, 2)),
        MapError::SpawnBlocked(TilePosition::new(1, 2)),
        MapError::DuplicateActor(MapActorId::new("actor").unwrap()),
        MapError::ActorOutOfBounds {
            actor: MapActorId::new("actor").unwrap(),
            position: TilePosition::new(1, 2),
        },
        MapError::ActorBlocked {
            actor: MapActorId::new("actor").unwrap(),
            position: TilePosition::new(1, 2),
        },
        MapError::ActorOnPlayerSpawn(MapActorId::new("actor").unwrap()),
        MapError::ActorOverlap(TilePosition::new(1, 2)),
        MapError::DuplicateMaterial(material.clone()),
        MapError::UnknownMaterial(material.clone()),
        MapError::EmptyMaterial(material.clone()),
        MapError::UnknownAtomicTile(tile),
        MapError::MaterialInUse(material),
        MapError::Json("bad".into()),
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
    }
}

#[test]
fn validates_static_actors_as_blocking_world_content() {
    let (mut project, known) = fixture();
    project.actors.push(MapActor::new(
        MapActorId::new("guide").unwrap(),
        TilePosition::new(2, 1),
        MapDirection::Left,
        CharacterAppearanceId::new("dppt/000").unwrap(),
    ));
    project.validate(&known).unwrap();

    let actor = project.actors[0].clone();
    project.actors.push(actor.clone());
    assert!(matches!(
        project.validate(&known),
        Err(MapError::DuplicateActor(_))
    ));
    project.actors.pop();
    project.actors[0].position = project.player_spawn;
    assert!(matches!(
        project.validate(&known),
        Err(MapError::ActorOnPlayerSpawn(_))
    ));
    project.actors[0].position = TilePosition::new(99, 99);
    assert!(matches!(
        project.validate(&known),
        Err(MapError::ActorOutOfBounds { .. })
    ));
}
