//! Pure integer-grid world rules.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Ground,
    Wall,
    Grass,
}

impl Tile {
    pub const fn is_walkable(self) -> bool {
        !matches!(self, Self::Wall)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    x: u16,
    y: u16,
}

impl Position {
    pub const fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }

    pub const fn x(self) -> u16 {
        self.x
    }

    pub const fn y(self) -> u16 {
        self.y
    }

    fn neighbor(self, direction: Direction) -> Option<Self> {
        match direction {
            Direction::Up => self.y.checked_sub(1).map(|y| Self::new(self.x, y)),
            Direction::Down => self.y.checked_add(1).map(|y| Self::new(self.x, y)),
            Direction::Left => self.x.checked_sub(1).map(|x| Self::new(x, self.y)),
            Direction::Right => self.x.checked_add(1).map(|x| Self::new(x, self.y)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldActorId(String);

impl WorldActorId {
    pub fn new(value: impl Into<String>) -> Result<Self, WorldError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(WorldError::EmptyActorId);
        }
        Ok(Self(value))
    }

    pub fn player() -> Self {
        Self("player".into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldActor {
    id: WorldActorId,
    position: Position,
    facing: Direction,
    blocks_movement: bool,
}

impl WorldActor {
    pub const fn new(
        id: WorldActorId,
        position: Position,
        facing: Direction,
        blocks_movement: bool,
    ) -> Self {
        Self {
            id,
            position,
            facing,
            blocks_movement,
        }
    }

    pub const fn id(&self) -> &WorldActorId {
        &self.id
    }

    pub const fn position(&self) -> Position {
        self.position
    }

    pub const fn facing(&self) -> Direction {
        self.facing
    }

    pub const fn blocks_movement(&self) -> bool {
        self.blocks_movement
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileMap {
    width: u16,
    height: u16,
    tiles: Vec<Tile>,
}

impl TileMap {
    pub fn new(width: u16, height: u16, tiles: Vec<Tile>) -> Result<Self, WorldError> {
        if width == 0 || height == 0 {
            return Err(WorldError::EmptyMap);
        }
        let expected = usize::from(width) * usize::from(height);
        if tiles.len() != expected {
            return Err(WorldError::TileCount {
                expected,
                actual: tiles.len(),
            });
        }
        Ok(Self {
            width,
            height,
            tiles,
        })
    }

    pub const fn width(&self) -> u16 {
        self.width
    }

    pub const fn height(&self) -> u16 {
        self.height
    }

    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }

    pub fn tile(&self, position: Position) -> Option<Tile> {
        if position.x >= self.width || position.y >= self.height {
            return None;
        }
        Some(
            self.tiles[usize::from(position.y) * usize::from(self.width) + usize::from(position.x)],
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldCommand {
    Face(Direction),
    Move(Direction),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldActorCommand {
    Face(Direction),
    Move(Direction),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldEvent {
    Turned { from: Direction, to: Direction },
    Moved { from: Position, to: Position },
    Blocked { at: Position },
    BlockedByActor { actor: WorldActorId, at: Position },
    EncounterTriggered { at: Position },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldOutcome {
    event: WorldEvent,
}

impl WorldOutcome {
    pub fn event(&self) -> WorldEvent {
        self.event.clone()
    }

    pub const fn starts_battle(&self) -> bool {
        matches!(self.event, WorldEvent::EncounterTriggered { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    map: TileMap,
    player: WorldActor,
    actors: Vec<WorldActor>,
}

impl World {
    pub fn new(map: TileMap, player: Position, facing: Direction) -> Result<Self, WorldError> {
        Self::with_actors(map, player, facing, Vec::new())
    }

    pub fn with_actors(
        map: TileMap,
        player: Position,
        facing: Direction,
        mut actors: Vec<WorldActor>,
    ) -> Result<Self, WorldError> {
        validate_player(&map, player)?;
        let player = WorldActor::new(WorldActorId::player(), player, facing, true);
        let mut actor_ids = BTreeSet::from([player.id.clone()]);
        let mut occupied = BTreeSet::from([player.position]);
        for actor in &actors {
            if !actor_ids.insert(actor.id.clone()) {
                return Err(WorldError::DuplicateActor(actor.id.clone()));
            }
            validate_actor(&map, actor)?;
            if actor.blocks_movement && !occupied.insert(actor.position) {
                return Err(WorldError::ActorOverlap {
                    actor: actor.id.clone(),
                    position: actor.position,
                });
            }
        }
        actors.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(Self {
            map,
            player,
            actors,
        })
    }

    pub const fn map(&self) -> &TileMap {
        &self.map
    }

    pub const fn player(&self) -> Position {
        self.player.position
    }

    pub const fn facing(&self) -> Direction {
        self.player.facing
    }

    pub const fn player_id(&self) -> &WorldActorId {
        &self.player.id
    }

    pub fn actors(&self) -> impl Iterator<Item = &WorldActor> {
        std::iter::once(&self.player).chain(self.actors.iter())
    }

    pub fn transition(&self, command: WorldCommand) -> (Self, WorldOutcome) {
        let command = match command {
            WorldCommand::Face(direction) => WorldActorCommand::Face(direction),
            WorldCommand::Move(direction) => WorldActorCommand::Move(direction),
        };
        self.transition_with_actor(self.player_id(), command, true)
            .expect("the player actor always exists")
    }

    pub fn transition_actor(
        &self,
        actor: &WorldActorId,
        command: WorldActorCommand,
    ) -> Result<(Self, WorldOutcome), WorldError> {
        if actor == self.player_id() {
            return Err(WorldError::PlayerActorCommand);
        }
        self.transition_with_actor(actor, command, false)
    }

    fn transition_with_actor(
        &self,
        actor_id: &WorldActorId,
        command: WorldActorCommand,
        triggers_encounters: bool,
    ) -> Result<(Self, WorldOutcome), WorldError> {
        let actor = self
            .actors()
            .find(|actor| actor.id == *actor_id)
            .ok_or_else(|| WorldError::UnknownActor(actor_id.clone()))?;
        let from = actor.position;
        let mut next = self.clone();
        let direction = match command {
            WorldActorCommand::Face(direction) => {
                next.actor_mut(actor_id)
                    .expect("the validated actor exists")
                    .facing = direction;
                return Ok((
                    next,
                    WorldOutcome {
                        event: WorldEvent::Turned {
                            from: actor.facing,
                            to: direction,
                        },
                    },
                ));
            }
            WorldActorCommand::Move(direction) => direction,
        };
        next.actor_mut(actor_id)
            .expect("the validated actor exists")
            .facing = direction;
        let Some(target) = from.neighbor(direction) else {
            return Ok(blocked(next, from));
        };
        let Some(target_tile) = next.map.tile(target) else {
            return Ok(blocked(next, target));
        };
        if !target_tile.is_walkable() {
            return Ok(blocked(next, target));
        }
        let blocking_actor = next
            .actors()
            .find(|other| {
                other.id != *actor_id && other.blocks_movement && other.position == target
            })
            .map(|actor| actor.id.clone());
        if let Some(blocking_actor) = blocking_actor {
            return Ok((
                next,
                WorldOutcome {
                    event: WorldEvent::BlockedByActor {
                        actor: blocking_actor,
                        at: target,
                    },
                },
            ));
        }

        next.actor_mut(actor_id)
            .expect("the validated actor exists")
            .position = target;
        let entered_grass = triggers_encounters
            && next.map.tile(from) != Some(Tile::Grass)
            && target_tile == Tile::Grass;
        Ok((
            next,
            WorldOutcome {
                event: if entered_grass {
                    WorldEvent::EncounterTriggered { at: target }
                } else {
                    WorldEvent::Moved { from, to: target }
                },
            },
        ))
    }

    fn actor_mut(&mut self, actor_id: &WorldActorId) -> Option<&mut WorldActor> {
        if actor_id == self.player_id() {
            Some(&mut self.player)
        } else {
            self.actors.iter_mut().find(|actor| actor.id == *actor_id)
        }
    }
}

fn validate_player(map: &TileMap, player: Position) -> Result<(), WorldError> {
    match map.tile(player) {
        None => Err(WorldError::PlayerOutOfBounds(player)),
        Some(tile) if !tile.is_walkable() => Err(WorldError::PlayerOnBlockedTile(player)),
        Some(_) => Ok(()),
    }
}

fn validate_actor(map: &TileMap, actor: &WorldActor) -> Result<(), WorldError> {
    if actor.id.as_str().trim().is_empty() {
        return Err(WorldError::EmptyActorId);
    }
    match map.tile(actor.position) {
        None => Err(WorldError::ActorOutOfBounds {
            actor: actor.id.clone(),
            position: actor.position,
        }),
        Some(tile) if !tile.is_walkable() => Err(WorldError::ActorOnBlockedTile {
            actor: actor.id.clone(),
            position: actor.position,
        }),
        Some(_) => Ok(()),
    }
}

fn blocked(world: World, at: Position) -> (World, WorldOutcome) {
    (
        world,
        WorldOutcome {
            event: WorldEvent::Blocked { at },
        },
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldError {
    EmptyMap,
    TileCount {
        expected: usize,
        actual: usize,
    },
    EmptyActorId,
    DuplicateActor(WorldActorId),
    UnknownActor(WorldActorId),
    PlayerActorCommand,
    PlayerOutOfBounds(Position),
    PlayerOnBlockedTile(Position),
    ActorOutOfBounds {
        actor: WorldActorId,
        position: Position,
    },
    ActorOnBlockedTile {
        actor: WorldActorId,
        position: Position,
    },
    ActorOverlap {
        actor: WorldActorId,
        position: Position,
    },
}

#[cfg(test)]
mod tests {
    use super::{
        Direction, Position, Tile, TileMap, World, WorldActor, WorldActorCommand, WorldActorId,
        WorldCommand, WorldEvent,
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
        let world =
            World::with_actors(map, Position::new(0, 0), Direction::Right, vec![npc]).unwrap();
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
        let world =
            World::with_actors(map, Position::new(0, 0), Direction::Right, vec![npc]).unwrap();
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
                .any(|actor| actor.id().as_str() == "guide"
                    && actor.position() == Position::new(2, 0))
        );
        assert!(matches!(
            world.transition_actor(
                &WorldActorId::player(),
                WorldActorCommand::Face(Direction::Up)
            ),
            Err(super::WorldError::PlayerActorCommand)
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
            Err(super::WorldError::ActorOverlap { .. })
        ));
        assert!(matches!(
            World::with_actors(
                map,
                Position::new(0, 0),
                Direction::Down,
                vec![actor("a", Position::new(1, 0))]
            ),
            Err(super::WorldError::ActorOnBlockedTile { .. })
        ));
    }

    #[test]
    fn map_and_spawn_boundaries_are_explicit() {
        assert_eq!(TileMap::new(0, 1, vec![]), Err(super::WorldError::EmptyMap));
        let map = TileMap::new(1, 1, vec![Tile::Ground]).unwrap();
        assert_eq!(map.tile(Position::new(1, 0)), None);
        assert_eq!(
            World::new(map.clone(), Position::new(1, 0), Direction::Down),
            Err(super::WorldError::PlayerOutOfBounds(Position::new(1, 0)))
        );
        let blocked = TileMap::new(1, 1, vec![Tile::Wall]).unwrap();
        assert_eq!(
            World::new(blocked, Position::new(0, 0), Direction::Down),
            Err(super::WorldError::PlayerOnBlockedTile(Position::new(0, 0)))
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
}
