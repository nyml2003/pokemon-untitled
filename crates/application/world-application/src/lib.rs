//! Pure use-case boundary for the overworld.

#![forbid(unsafe_code)]

use map_project::{Collision, MapEventKind, MapProject};
pub use world_domain::{
    Direction, Position, Tile, WorldCommand, WorldError, WorldEvent, WorldOutcome,
};
use world_domain::{TileMap, World};

pub const DEMO_MAP_WIDTH: u16 = 16;
pub const DEMO_MAP_HEIGHT: u16 = 10;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldObservation {
    width: u16,
    height: u16,
    tiles: Vec<Tile>,
    player: Position,
    facing: Direction,
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
}

#[derive(Clone)]
pub struct WorldApplication {
    world: World,
}

impl WorldApplication {
    pub const fn new(world: World) -> Self {
        Self { world }
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
        World::new(map, spawn, Direction::Down).map(Self::new)
    }

    pub fn observe(&self) -> WorldObservation {
        WorldObservation {
            width: self.world.map().width(),
            height: self.world.map().height(),
            tiles: self.world.map().tiles().to_vec(),
            player: self.world.player(),
            facing: self.world.facing(),
        }
    }

    pub const fn player(&self) -> Position {
        self.world.player()
    }

    pub fn transition(&self, command: WorldCommand) -> (Self, WorldOutcome) {
        let (world, outcome) = self.world.transition(command);
        (Self::new(world), outcome)
    }
}

#[cfg(test)]
mod tests {
    use map_project::{
        AtomicTileId, Collision, CompositeTile, CompositeTileId, MapEventKind, MapProject,
        MapProjectId, TilePosition,
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
}
