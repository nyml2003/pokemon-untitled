//! Pure integer-grid world rules.

#![forbid(unsafe_code)]

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
pub enum WorldEvent {
    Turned { from: Direction, to: Direction },
    Moved { from: Position, to: Position },
    Blocked { at: Position },
    EncounterTriggered { at: Position },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldOutcome {
    event: WorldEvent,
}

impl WorldOutcome {
    pub const fn event(self) -> WorldEvent {
        self.event
    }

    pub const fn starts_battle(self) -> bool {
        matches!(self.event, WorldEvent::EncounterTriggered { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    map: TileMap,
    player: Position,
    facing: Direction,
}

impl World {
    pub fn new(map: TileMap, player: Position, facing: Direction) -> Result<Self, WorldError> {
        match map.tile(player) {
            None => return Err(WorldError::PlayerOutOfBounds(player)),
            Some(tile) if !tile.is_walkable() => {
                return Err(WorldError::PlayerOnBlockedTile(player));
            }
            Some(_) => {}
        }
        Ok(Self {
            map,
            player,
            facing,
        })
    }

    pub const fn map(&self) -> &TileMap {
        &self.map
    }

    pub const fn player(&self) -> Position {
        self.player
    }

    pub const fn facing(&self) -> Direction {
        self.facing
    }

    pub fn transition(&self, command: WorldCommand) -> (Self, WorldOutcome) {
        let mut next = self.clone();
        let direction = match command {
            WorldCommand::Face(direction) => {
                let from = next.facing;
                next.facing = direction;
                return (
                    next,
                    WorldOutcome {
                        event: WorldEvent::Turned {
                            from,
                            to: direction,
                        },
                    },
                );
            }
            WorldCommand::Move(direction) => direction,
        };
        next.facing = direction;
        let Some(target) = next.player.neighbor(direction) else {
            return (
                next.clone(),
                WorldOutcome {
                    event: WorldEvent::Blocked { at: next.player },
                },
            );
        };
        let Some(target_tile) = next.map.tile(target) else {
            return (
                next,
                WorldOutcome {
                    event: WorldEvent::Blocked { at: target },
                },
            );
        };
        if !target_tile.is_walkable() {
            return (
                next,
                WorldOutcome {
                    event: WorldEvent::Blocked { at: target },
                },
            );
        }

        let from = next.player;
        let entered_grass = next.map.tile(from) != Some(Tile::Grass) && target_tile == Tile::Grass;
        next.player = target;
        (
            next,
            WorldOutcome {
                event: if entered_grass {
                    WorldEvent::EncounterTriggered { at: target }
                } else {
                    WorldEvent::Moved { from, to: target }
                },
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldError {
    EmptyMap,
    TileCount { expected: usize, actual: usize },
    PlayerOutOfBounds(Position),
    PlayerOnBlockedTile(Position),
}

#[cfg(test)]
mod tests {
    use super::{Direction, Position, Tile, TileMap, World, WorldCommand, WorldEvent};

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
    fn invalid_tile_count_is_rejected() {
        assert!(TileMap::new(2, 2, vec![Tile::Ground]).is_err());
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
