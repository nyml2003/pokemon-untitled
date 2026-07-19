use std::collections::BTreeMap;

use map_project::{
    AtomicTileId, CharacterAppearanceId, Collision, CompositeTile, CompositeTileId, MapActor,
    MapActorId, MapDirection, MapEventKind, MapProject, MapProjectId, TilePosition,
};
use narrative_cps::{
    ActorId, ContinuationId, CpsNode, ScriptDirection, ScriptId, ScriptProgram, TextId,
};

use super::{Direction, Position, Tile, WorldApplication, WorldCommand};

#[test]
fn demo_map_exposes_a_walkable_spawn_and_nearby_grass() {
    let mut application = WorldApplication::demo().unwrap();
    let opening = application.observe().unwrap();

    assert_eq!(opening.player(), Position::new(3, 6));
    assert_eq!(opening.width(), 16);
    assert_eq!(opening.height(), 10);
    assert_eq!(opening.tile(opening.player()), Some(Tile::Ground));
    assert_eq!(opening.tile(Position::new(99, 99)), None);
    assert_eq!(opening.facing(), Direction::Down);
    assert_eq!(application.player(), opening.player());
    let (next, outcome) = application.transition(WorldCommand::Move(Direction::Right));
    application = next;
    assert!(!outcome.starts_battle());
    let (next, outcome) = application.transition(WorldCommand::Move(Direction::Right));
    application = next;
    assert!(!outcome.starts_battle());
    let (_, outcome) = application.transition(WorldCommand::Move(Direction::Right));
    assert!(outcome.starts_battle());
}

#[test]
fn independent_collision_and_event_layers_drive_world_rules() {
    let mut project = MapProject::blank(
        MapProjectId::new("world").unwrap(),
        3,
        2,
        Some(CompositeTile::new(
            CompositeTileId::new("base").unwrap(),
            vec![AtomicTileId::new("tile").unwrap()],
        )),
    );
    project.player_spawn = TilePosition::new(0, 0);
    project.event_cells[1] = Some(MapEventKind::Encounter);
    project.collision_cells[2] = Collision::Blocked;
    let world = WorldApplication::from_map_project(&project).unwrap();
    assert!(
        world
            .transition(WorldCommand::Move(Direction::Right))
            .1
            .starts_battle()
    );
}

#[test]
fn map_actors_become_visible_world_observations() {
    let mut project = MapProject::blank(
        MapProjectId::new("world").unwrap(),
        3,
        2,
        Some(CompositeTile::new(
            CompositeTileId::new("base").unwrap(),
            vec![AtomicTileId::new("tile").unwrap()],
        )),
    );
    project.player_spawn = TilePosition::new(0, 0);
    project.actors.push(MapActor::new(
        MapActorId::new("guide").unwrap(),
        TilePosition::new(1, 0),
        MapDirection::Left,
        CharacterAppearanceId::new("dppt/000").unwrap(),
    ));
    let observation = WorldApplication::from_map_project(&project)
        .unwrap()
        .observe()
        .unwrap();
    assert_eq!(observation.actors().len(), 2);
    let npc = observation
        .actors()
        .iter()
        .find(|actor| actor.role() == super::WorldActorRole::Npc)
        .unwrap();
    assert_eq!(npc.id().as_str(), "guide");
    assert_eq!(npc.position(), Position::new(1, 0));
    assert_eq!(npc.facing(), Direction::Left);
    assert_eq!(npc.appearance().as_str(), "dppt/000");
}

#[test]
fn map_actor_appearances_survive_player_transitions() {
    let material = CompositeTile::new(
        CompositeTileId::new("ground").unwrap(),
        vec![AtomicTileId::new("tile-0001").unwrap()],
    );
    let mut project = MapProject::blank(MapProjectId::new("demo").unwrap(), 3, 1, Some(material));
    project.player_spawn = TilePosition::new(0, 0);
    project.actors.push(MapActor::new(
        MapActorId::new("guide").unwrap(),
        TilePosition::new(2, 0),
        MapDirection::Left,
        CharacterAppearanceId::new("dppt/000").unwrap(),
    ));
    let world = WorldApplication::from_map_project(&project).unwrap();
    let (world, _) = world.transition(WorldCommand::Move(Direction::Right));
    let observation = world.observe().unwrap();
    let npc = observation
        .actors()
        .iter()
        .find(|actor| actor.role() == super::WorldActorRole::Npc)
        .unwrap();
    assert_eq!(npc.appearance().as_str(), "dppt/000");
}

#[test]
fn actor_bound_scripts_drive_npc_movement_and_speech() {
    let material = CompositeTile::new(
        CompositeTileId::new("ground").unwrap(),
        vec![AtomicTileId::new("tile-0001").unwrap()],
    );
    let mut project = MapProject::blank(MapProjectId::new("demo").unwrap(), 6, 4, Some(material));
    project.player_spawn = TilePosition::new(0, 0);
    project.actors.push(MapActor::new(
        MapActorId::new("forest-guide").unwrap(),
        TilePosition::new(3, 1),
        MapDirection::Left,
        CharacterAppearanceId::new("dppt/000").unwrap(),
    ));
    let mut continuations = BTreeMap::new();
    continuations.insert(
        ContinuationId::new(0),
        CpsNode::Say {
            text: TextId::new("text:hello").unwrap(),
            next: ContinuationId::new(1),
        },
    );
    for (index, direction) in [
        ScriptDirection::Right,
        ScriptDirection::Down,
        ScriptDirection::Left,
        ScriptDirection::Up,
    ]
    .into_iter()
    .enumerate()
    {
        continuations.insert(
            ContinuationId::new(index as u32 + 1),
            CpsNode::Move {
                direction,
                next: ContinuationId::new(index as u32 + 2),
            },
        );
    }
    continuations.insert(ContinuationId::new(5), CpsNode::End);
    let script = ScriptProgram::with_actor(
        ScriptId::new("script:guide").unwrap(),
        Some(ActorId::new("actor:forest-guide").unwrap()),
        ContinuationId::new(0),
        continuations,
    )
    .unwrap();
    let world = WorldApplication::from_map_project_with_scripts(&project, [script]).unwrap();
    let (after_player_move, outcome) = world.transition(WorldCommand::Move(Direction::Right));
    assert!(matches!(outcome.event(), super::WorldEvent::Moved { .. }));
    let after_player_move = after_player_move.observe().unwrap();
    let npc_after_player_move = after_player_move
        .actors()
        .iter()
        .find(|actor| actor.id().as_str() == "forest-guide")
        .unwrap();
    assert_eq!(npc_after_player_move.position(), Position::new(3, 1));
    assert_eq!(npc_after_player_move.facing(), Direction::Left);
    assert_eq!(npc_after_player_move.speech(), None);

    let world = world.advance_npcs().unwrap();
    let observation = world.observe().unwrap();
    let npc = observation
        .actors()
        .iter()
        .find(|actor| actor.id().as_str() == "forest-guide")
        .unwrap();
    assert_eq!(npc.position(), Position::new(3, 1));
    assert_eq!(npc.speech().unwrap().as_str(), "text:hello");

    let world = world.advance_npcs().unwrap();
    let npc = world
        .observe()
        .unwrap()
        .actors()
        .iter()
        .find(|actor| actor.id().as_str() == "forest-guide")
        .unwrap()
        .clone();
    assert_eq!(npc.position(), Position::new(4, 1));
    assert_eq!(npc.facing(), Direction::Right);
    assert_eq!(npc.speech(), None);

    let world = world
        .advance_npcs()
        .unwrap()
        .advance_npcs()
        .unwrap()
        .advance_npcs()
        .unwrap();
    let observation = world.observe().unwrap();
    let npc = observation
        .actors()
        .iter()
        .find(|actor| actor.id().as_str() == "forest-guide")
        .unwrap();
    assert_eq!(npc.position(), Position::new(3, 1));
    assert_eq!(npc.facing(), Direction::Up);
}
