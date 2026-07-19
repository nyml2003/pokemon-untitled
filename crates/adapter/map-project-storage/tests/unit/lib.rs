use super::*;

fn tile(value: &str) -> AtomicTileId {
    AtomicTileId::new(value).unwrap()
}

fn fixture() -> (MapProject, BTreeSet<AtomicTileId>) {
    let ground = CompositeTile::new(
        CompositeTileId::new("ground").unwrap(),
        vec![tile("tile-ground")],
    );
    let wall = CompositeTile::new(
        CompositeTileId::new("wall").unwrap(),
        vec![tile("tile-wall")],
    );
    let mut project = MapProject::blank(
        MapProjectId::new("fixture").unwrap(),
        4,
        2,
        Some(ground.clone()),
    );
    project.materials.push(wall.clone());
    project.visual_cells[3] = VisualCell::new(Some(wall.id.clone()));
    project.collision_cells[3] = Collision::Blocked;
    project.event_cells[1] = Some(MapEventKind::Encounter);
    project.player_spawn = TilePosition::new(1, 1);
    project.actors.push(MapActor::new(
        MapActorId::new("guide").unwrap(),
        TilePosition::new(2, 1),
        MapDirection::Left,
        CharacterAppearanceId::new("hero").unwrap(),
    ));
    let known = [tile("tile-ground"), tile("tile-wall")]
        .into_iter()
        .collect();
    (project, known)
}

#[test]
fn round_trips_and_inspects_a_valid_project() {
    let (project, known) = fixture();
    let bytes = MapProjectWriter::default().write(&project, &known).unwrap();
    let metadata = MapProjectReader::inspect(&bytes).unwrap();
    assert_eq!(metadata.map_id, "fixture");
    assert_eq!((metadata.width, metadata.height), (4, 2));
    assert_eq!(metadata.material_count, 2);
    assert_eq!(metadata.event_count, 1);
    assert_eq!(MapProjectReader::read(&bytes, &known).unwrap(), project);
}

#[test]
fn output_is_deterministic() {
    let (project, known) = fixture();
    let writer = MapProjectWriter::default();
    assert_eq!(
        writer.write(&project, &known).unwrap(),
        writer.write(&project, &known).unwrap()
    );
}

#[test]
fn detects_truncation_and_corruption() {
    let (project, known) = fixture();
    let bytes = MapProjectWriter::default().write(&project, &known).unwrap();
    assert!(matches!(
        MapProjectReader::read(&bytes[..20], &known),
        Err(MapStorageError::Truncated)
    ));
    let mut corrupt = bytes;
    let last = corrupt.len() - 1;
    corrupt[last] ^= 0xff;
    assert!(MapProjectReader::read(&corrupt, &known).is_err());
}

#[test]
fn uses_rle_for_uniform_wide_rows() {
    let ground = CompositeTile::new(
        CompositeTileId::new("ground").unwrap(),
        vec![tile("tile-ground")],
    );
    let mut project =
        MapProject::blank(MapProjectId::new("uniform").unwrap(), 128, 1, Some(ground));
    project.player_spawn = TilePosition::new(0, 0);
    project.collision_cells.fill(Collision::Blocked);
    project.collision_cells[0] = Collision::Walkable;
    let known = [tile("tile-ground")].into_iter().collect();

    let raw = encode_project(&project).unwrap();
    let sections = parse_sections(&raw).unwrap();
    assert_eq!(required_section(&sections, SECTION_VISUAL).unwrap()[1], 1);
    assert_eq!(
        required_section(&sections, SECTION_COLLISION).unwrap()[0],
        1
    );

    let bytes = MapProjectWriter::default().write(&project, &known).unwrap();
    assert_eq!(MapProjectReader::read(&bytes, &known).unwrap(), project);
}
