use super::{
    Direction, Position, Tile, TileMap, World, WorldActor, WorldActorCommand, WorldActorId,
    WorldCommand, WorldError, WorldEvent,
};

fn world() -> World {
    let map = TileMap::new(
        4,
        3,
        vec![
            Tile::Wall,
            Tile::Wall,
            Tile::Wall,
            Tile::Wall,
            Tile::Wall,
            Tile::Ground,
            Tile::Grass,
            Tile::Wall,
            Tile::Wall,
            Tile::Wall,
            Tile::Wall,
            Tile::Wall,
        ],
    )
    .unwrap();
    World::new(map, Position::new(1, 1), Direction::Down).unwrap()
}

#[test]
fn blocked_move_only_changes_facing() {
    let (world, outcome) = world().transition(WorldCommand::Move(Direction::Up));

    assert_eq!(world.player(), Position::new(1, 1));
    assert_eq!(world.facing(), Direction::Up);
    assert_eq!(
        outcome.event(),
        WorldEvent::Blocked {
            at: Position::new(1, 0)
        }
    );
}

#[test]
fn face_changes_direction_without_changing_position() {
    let (world, outcome) = world().transition(WorldCommand::Face(Direction::Left));

    assert_eq!(world.player(), Position::new(1, 1));
    assert_eq!(world.facing(), Direction::Left);
    assert_eq!(
        outcome.event(),
        WorldEvent::Turned {
            from: Direction::Down,
            to: Direction::Left,
        }
    );
}

#[test]
fn entering_grass_moves_the_player_and_triggers_an_encounter() {
    let (world, outcome) = world().transition(WorldCommand::Move(Direction::Right));

    assert_eq!(world.player(), Position::new(2, 1));
    assert!(outcome.starts_battle());
}

#[test]
fn actors_block_movement_and_keep_their_identity() {
    let map = TileMap::new(3, 2, vec![Tile::Ground; 6]).unwrap();
    let npc = WorldActor::new(
        WorldActorId::new("guide").unwrap(),
        Position::new(1, 0),
        Direction::Down,
        true,
    );
    let world = World::with_actors(map, Position::new(0, 0), Direction::Right, vec![npc]).unwrap();
    let (_, outcome) = world.transition(WorldCommand::Move(Direction::Right));
    assert_eq!(
        outcome.event(),
        WorldEvent::BlockedByActor {
            actor: WorldActorId::new("guide").unwrap(),
            at: Position::new(1, 0),
        }
    );
}

#[test]
fn actor_commands_move_npcs_without_triggering_encounters() {
    let map = TileMap::new(3, 1, vec![Tile::Ground, Tile::Grass, Tile::Ground]).unwrap();
    let npc = WorldActor::new(
        WorldActorId::new("guide").unwrap(),
        Position::new(1, 0),
        Direction::Left,
        true,
    );
    let world = World::with_actors(map, Position::new(0, 0), Direction::Right, vec![npc]).unwrap();
    let (world, outcome) = world
        .transition_actor(
            &WorldActorId::new("guide").unwrap(),
            WorldActorCommand::Move(Direction::Right),
        )
        .unwrap();
    assert_eq!(
        outcome.event(),
        WorldEvent::Moved {
            from: Position::new(1, 0),
            to: Position::new(2, 0),
        }
    );
    assert!(
        world
            .actors()
            .any(|actor| actor.id().as_str() == "guide" && actor.position() == Position::new(2, 0))
    );
    assert!(matches!(
        world.transition_actor(
            &WorldActorId::player(),
            WorldActorCommand::Face(Direction::Up)
        ),
        Err(WorldError::PlayerActorCommand)
    ));
}

#[test]
fn actor_construction_rejects_invalid_occupancy() {
    let map = TileMap::new(2, 1, vec![Tile::Ground, Tile::Wall]).unwrap();
    let actor = |id, position| {
        WorldActor::new(
            WorldActorId::new(id).unwrap(),
            position,
            Direction::Down,
            true,
        )
    };
    assert!(matches!(
        World::with_actors(
            map.clone(),
            Position::new(0, 0),
            Direction::Down,
            vec![actor("a", Position::new(0, 0))]
        ),
        Err(WorldError::ActorOverlap { .. })
    ));
    assert!(matches!(
        World::with_actors(
            map,
            Position::new(0, 0),
            Direction::Down,
            vec![actor("a", Position::new(1, 0))]
        ),
        Err(WorldError::ActorOnBlockedTile { .. })
    ));
}

#[test]
fn map_and_spawn_boundaries_are_explicit() {
    assert_eq!(TileMap::new(0, 1, vec![]), Err(WorldError::EmptyMap));
    let map = TileMap::new(1, 1, vec![Tile::Ground]).unwrap();
    assert_eq!(map.tile(Position::new(1, 0)), None);
    assert_eq!(
        World::new(map.clone(), Position::new(1, 0), Direction::Down),
        Err(WorldError::PlayerOutOfBounds(Position::new(1, 0)))
    );
    let blocked = TileMap::new(1, 1, vec![Tile::Wall]).unwrap();
    assert_eq!(
        World::new(blocked, Position::new(0, 0), Direction::Down),
        Err(WorldError::PlayerOnBlockedTile(Position::new(0, 0)))
    );

    let world = World::new(map, Position::new(0, 0), Direction::Down).unwrap();
    let (world, underflow) = world.transition(WorldCommand::Move(Direction::Left));
    assert_eq!(
        underflow.event(),
        WorldEvent::Blocked {
            at: Position::new(0, 0)
        }
    );
    let (_, outside) = world.transition(WorldCommand::Move(Direction::Right));
    assert_eq!(
        outside.event(),
        WorldEvent::Blocked {
            at: Position::new(1, 0)
        }
    );
}

#[test]
fn ordinary_moves_cover_both_remaining_directions() {
    let map = TileMap::new(2, 2, vec![Tile::Ground; 4]).unwrap();
    let world = World::new(map, Position::new(0, 0), Direction::Down).unwrap();
    let (world, down) = world.transition(WorldCommand::Move(Direction::Down));
    assert_eq!(
        down.event(),
        WorldEvent::Moved {
            from: Position::new(0, 0),
            to: Position::new(0, 1)
        }
    );
    let (_, right) = world.transition(WorldCommand::Move(Direction::Right));
    assert!(matches!(right.event(), WorldEvent::Moved { .. }));
}
