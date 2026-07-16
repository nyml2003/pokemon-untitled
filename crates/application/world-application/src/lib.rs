//! Pure use-case boundary for the overworld.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

pub use map_project::CharacterAppearanceId;
use map_project::{Collision, MapDirection, MapEventKind, MapProject};
pub use world_domain::{
    Direction, Position, Tile, WorldActorId, WorldCommand, WorldError, WorldEvent, WorldOutcome,
};
use world_domain::{TileMap, World, WorldActor};

pub const DEMO_MAP_WIDTH: u16 = 16;
pub const DEMO_MAP_HEIGHT: u16 = 10;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldObservation {
    width: u16,
    height: u16,
    tiles: Vec<Tile>,
    player: Position,
    facing: Direction,
    actors: Vec<WorldActorObservation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldActorRole {
    Player,
    Npc,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldActorObservation {
    id: WorldActorId,
    role: WorldActorRole,
    position: Position,
    facing: Direction,
    appearance: CharacterAppearanceId,
}

impl WorldActorObservation {
    pub const fn id(&self) -> &WorldActorId {
        &self.id
    }

    pub const fn role(&self) -> WorldActorRole {
        self.role
    }

    pub const fn position(&self) -> Position {
        self.position
    }

    pub const fn facing(&self) -> Direction {
        self.facing
    }

    pub const fn appearance(&self) -> &CharacterAppearanceId {
        &self.appearance
    }
}

impl WorldObservation {
    pub const fn width(&self) -> u16 {
        self.width
    }

    pub const fn height(&self) -> u16 {
        self.height
    }

    pub fn tile(&self, position: Position) -> Option<Tile> {
        if position.x() >= self.width || position.y() >= self.height {
            return None;
        }
        Some(
            self.tiles
                [usize::from(position.y()) * usize::from(self.width) + usize::from(position.x())],
        )
    }

    pub const fn player(&self) -> Position {
        self.player
    }

    pub const fn facing(&self) -> Direction {
        self.facing
    }

    pub fn actors(&self) -> &[WorldActorObservation] {
        &self.actors
    }
}

#[derive(Clone)]
pub struct WorldApplication {
    world: World,
    appearances: BTreeMap<WorldActorId, CharacterAppearanceId>,
}

impl WorldApplication {
    pub fn new(world: World) -> Self {
        let default_appearance =
            CharacterAppearanceId::new("red").expect("the fixed player appearance is valid");
        let appearances = world
            .actors()
            .map(|actor| (actor.id().clone(), default_appearance.clone()))
            .collect();
        Self { world, appearances }
    }

    pub fn demo() -> Result<Self, WorldError> {
        let mut tiles = vec![Tile::Ground; usize::from(DEMO_MAP_WIDTH * DEMO_MAP_HEIGHT)];
        for y in 0..DEMO_MAP_HEIGHT {
            for x in 0..DEMO_MAP_WIDTH {
                let border =
                    x == 0 || y == 0 || x + 1 == DEMO_MAP_WIDTH || y + 1 == DEMO_MAP_HEIGHT;
                let grass = (6..=10).contains(&x) && (2..=7).contains(&y);
                let rocks = matches!((x, y), (3, 3) | (4, 3) | (12, 5) | (12, 6));
                let tile = if border || rocks {
                    Tile::Wall
                } else if grass {
                    Tile::Grass
                } else {
                    Tile::Ground
                };
                tiles[usize::from(y * DEMO_MAP_WIDTH + x)] = tile;
            }
        }
        let map = TileMap::new(DEMO_MAP_WIDTH, DEMO_MAP_HEIGHT, tiles)?;
        let world = World::new(map, Position::new(3, 6), Direction::Down)?;
        Ok(Self::new(world))
    }

    pub fn from_map_project(project: &MapProject) -> Result<Self, WorldError> {
        let tiles = project
            .collision_cells
            .iter()
            .zip(&project.event_cells)
            .map(|(collision, event)| match (collision, event) {
                (Collision::Blocked, _) => Tile::Wall,
                (Collision::Walkable, Some(MapEventKind::Encounter)) => Tile::Grass,
                (Collision::Walkable, None) => Tile::Ground,
            })
            .collect();
        let map = TileMap::new(project.width, project.height, tiles)?;
        let spawn = Position::new(project.player_spawn.x(), project.player_spawn.y());
        let mut appearances = BTreeMap::from([(
            WorldActorId::player(),
            CharacterAppearanceId::new("red").expect("the fixed player appearance is valid"),
        )]);
        let mut actors = Vec::with_capacity(project.actors.len());
        for actor in &project.actors {
            let id = WorldActorId::new(actor.id.as_str())?;
            appearances.insert(id.clone(), actor.appearance.clone());
            actors.push(WorldActor::new(
                id,
                Position::new(actor.position.x(), actor.position.y()),
                direction_from_map(actor.facing),
                true,
            ));
        }
        let world = World::with_actors(map, spawn, Direction::Down, actors)?;
        Ok(Self { world, appearances })
    }

    pub fn observe(&self) -> WorldObservation {
        WorldObservation {
            width: self.world.map().width(),
            height: self.world.map().height(),
            tiles: self.world.map().tiles().to_vec(),
            player: self.world.player(),
            facing: self.world.facing(),
            actors: self
                .world
                .actors()
                .map(|actor| WorldActorObservation {
                    id: actor.id().clone(),
                    role: if actor.id() == self.world.player_id() {
                        WorldActorRole::Player
                    } else {
                        WorldActorRole::Npc
                    },
                    position: actor.position(),
                    facing: actor.facing(),
                    appearance: self
                        .appearances
                        .get(actor.id())
                        .expect("every world actor has an appearance")
                        .clone(),
                })
                .collect(),
        }
    }

    pub const fn player(&self) -> Position {
        self.world.player()
    }

    pub fn transition(&self, command: WorldCommand) -> (Self, WorldOutcome) {
        let (world, outcome) = self.world.transition(command);
        (
            Self {
                world,
                appearances: self.appearances.clone(),
            },
            outcome,
        )
    }
}

const fn direction_from_map(direction: MapDirection) -> Direction {
    match direction {
        MapDirection::Up => Direction::Up,
        MapDirection::Down => Direction::Down,
        MapDirection::Left => Direction::Left,
        MapDirection::Right => Direction::Right,
    }
}

#[cfg(test)]
mod tests {
    use map_project::{
        AtomicTileId, CharacterAppearanceId, Collision, CompositeTile, CompositeTileId, MapActor,
        MapActorId, MapDirection, MapEventKind, MapProject, MapProjectId, TilePosition,
    };

    use super::{Direction, Position, Tile, WorldApplication, WorldCommand};

    #[test]
    fn demo_map_exposes_a_walkable_spawn_and_nearby_grass() {
        let mut application = WorldApplication::demo().unwrap();
        let opening = application.observe();

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
            .observe();
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
        let mut project =
            MapProject::blank(MapProjectId::new("demo").unwrap(), 3, 1, Some(material));
        project.player_spawn = TilePosition::new(0, 0);
        project.actors.push(MapActor::new(
            MapActorId::new("guide").unwrap(),
            TilePosition::new(2, 0),
            MapDirection::Left,
            CharacterAppearanceId::new("dppt/000").unwrap(),
        ));
        let world = WorldApplication::from_map_project(&project).unwrap();
        let (world, _) = world.transition(WorldCommand::Move(Direction::Right));
        let observation = world.observe();
        let npc = observation
            .actors()
            .iter()
            .find(|actor| actor.role() == super::WorldActorRole::Npc)
            .unwrap();
        assert_eq!(npc.appearance().as_str(), "dppt/000");
    }
}
