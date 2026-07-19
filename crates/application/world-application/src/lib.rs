//! Pure use-case boundary for the overworld.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

pub use map_project::CharacterAppearanceId;
use map_project::{Collision, MapDirection, MapError, MapEventKind, MapProject};
pub use narrative_cps::TextId;
use narrative_cps::{CpsNode, ScriptDirection, ScriptProgram};
pub use world_domain::{
    Direction, Position, Tile, WorldActorCommand, WorldActorId, WorldCommand, WorldError,
    WorldEvent, WorldOutcome,
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
    speech: Option<TextId>,
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

    pub fn speech(&self) -> Option<&TextId> {
        self.speech.as_ref()
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
    npc_scripts: BTreeMap<WorldActorId, NpcScriptState>,
    speech: BTreeMap<WorldActorId, TextId>,
}

#[derive(Clone)]
struct NpcScriptState {
    program: ScriptProgram,
    continuation: narrative_cps::ContinuationId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldApplicationError {
    World(WorldError),
    Appearance(MapError),
    UnboundScript {
        script: narrative_cps::ScriptId,
    },
    ScriptActorMissing {
        actor: narrative_cps::ActorId,
    },
    MalformedScriptActor {
        actor: narrative_cps::ActorId,
    },
    ScriptControlsPlayer,
    DuplicateScriptActor {
        actor: narrative_cps::ActorId,
    },
    MissingAppearance {
        actor: WorldActorId,
    },
    MissingContinuation {
        script: narrative_cps::ScriptId,
        continuation: narrative_cps::ContinuationId,
    },
}

impl From<WorldError> for WorldApplicationError {
    fn from(error: WorldError) -> Self {
        Self::World(error)
    }
}

impl From<MapError> for WorldApplicationError {
    fn from(error: MapError) -> Self {
        Self::Appearance(error)
    }
}

impl WorldApplication {
    pub fn new(world: World) -> Result<Self, WorldApplicationError> {
        let default_appearance = CharacterAppearanceId::new("red")?;
        let appearances = world
            .actors()
            .map(|actor| (actor.id().clone(), default_appearance.clone()))
            .collect();
        Ok(Self {
            world,
            appearances,
            npc_scripts: BTreeMap::new(),
            speech: BTreeMap::new(),
        })
    }

    pub fn demo() -> Result<Self, WorldApplicationError> {
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
        Self::new(world)
    }

    pub fn from_map_project(project: &MapProject) -> Result<Self, WorldApplicationError> {
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
        let mut appearances =
            BTreeMap::from([(WorldActorId::player(), CharacterAppearanceId::new("red")?)]);
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
        Ok(Self {
            world,
            appearances,
            npc_scripts: BTreeMap::new(),
            speech: BTreeMap::new(),
        })
    }

    pub fn from_map_project_with_scripts(
        project: &MapProject,
        scripts: impl IntoIterator<Item = ScriptProgram>,
    ) -> Result<Self, WorldApplicationError> {
        Self::from_map_project(project)?.with_npc_scripts(scripts)
    }

    pub fn with_npc_scripts(
        mut self,
        scripts: impl IntoIterator<Item = ScriptProgram>,
    ) -> Result<Self, WorldApplicationError> {
        for program in scripts {
            let actor =
                program
                    .actor()
                    .cloned()
                    .ok_or_else(|| WorldApplicationError::UnboundScript {
                        script: program.id().clone(),
                    })?;
            if actor.as_str() == "actor:player" {
                return Err(WorldApplicationError::ScriptControlsPlayer);
            }
            let actor_id =
                WorldActorId::new(actor.as_str().strip_prefix("actor:").ok_or_else(|| {
                    WorldApplicationError::MalformedScriptActor {
                        actor: actor.clone(),
                    }
                })?)?;
            if !self
                .world
                .actors()
                .any(|candidate| candidate.id() == &actor_id)
            {
                return Err(WorldApplicationError::ScriptActorMissing { actor });
            }
            if self.npc_scripts.contains_key(&actor_id) {
                return Err(WorldApplicationError::DuplicateScriptActor { actor });
            }
            self.npc_scripts.insert(
                actor_id,
                NpcScriptState {
                    continuation: program.entry(),
                    program,
                },
            );
        }
        Ok(self)
    }

    /// Builds a renderable observation of the current world state.
    ///
    /// Missing appearance data is reported so a corrupted map cannot silently
    /// omit or misrender an actor.
    pub fn observe(&self) -> Result<WorldObservation, WorldApplicationError> {
        Ok(WorldObservation {
            width: self.world.map().width(),
            height: self.world.map().height(),
            tiles: self.world.map().tiles().to_vec(),
            player: self.world.player(),
            facing: self.world.facing(),
            actors: self
                .world
                .actors()
                .map(|actor| {
                    let appearance = self
                        .appearances
                        .get(actor.id())
                        .ok_or_else(|| WorldApplicationError::MissingAppearance {
                            actor: actor.id().clone(),
                        })?
                        .clone();
                    Ok(WorldActorObservation {
                        id: actor.id().clone(),
                        role: if actor.id() == self.world.player_id() {
                            WorldActorRole::Player
                        } else {
                            WorldActorRole::Npc
                        },
                        position: actor.position(),
                        facing: actor.facing(),
                        appearance,
                        speech: self.speech.get(actor.id()).cloned(),
                    })
                })
                .collect::<Result<Vec<_>, WorldApplicationError>>()?,
        })
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
                npc_scripts: self.npc_scripts.clone(),
                speech: self.speech.clone(),
            },
            outcome,
        )
    }

    /// Advances every actor-bound script by one logical world tick.
    ///
    /// The caller supplies the cadence. Keeping real time outside this pure
    /// application boundary makes the resulting world state deterministic.
    pub fn advance_npcs(&self) -> Result<Self, WorldApplicationError> {
        let mut world = self.world.clone();
        let mut scripts = self.npc_scripts.clone();
        let mut speech = self.speech.clone();
        for (actor, state) in &mut scripts {
            let node = state
                .program
                .continuation(state.continuation)
                .ok_or_else(|| WorldApplicationError::MissingContinuation {
                    script: state.program.id().clone(),
                    continuation: state.continuation,
                })?
                .clone();
            match node {
                CpsNode::Move { direction, next } => {
                    if let Ok((next_world, _)) = world.transition_actor(
                        actor,
                        WorldActorCommand::Move(direction_from_script(direction)),
                    ) {
                        world = next_world;
                    }
                    speech.remove(actor);
                    state.continuation = next;
                }
                CpsNode::Face { direction, next } => {
                    if let Ok((next_world, _)) = world.transition_actor(
                        actor,
                        WorldActorCommand::Face(direction_from_script(direction)),
                    ) {
                        world = next_world;
                    }
                    speech.remove(actor);
                    state.continuation = next;
                }
                CpsNode::Say { text, next } => {
                    speech.insert(actor.clone(), text);
                    state.continuation = next;
                }
                CpsNode::Wait { .. } => {}
                CpsNode::End => state.continuation = state.program.entry(),
            }
        }
        Ok(Self {
            world,
            appearances: self.appearances.clone(),
            npc_scripts: scripts,
            speech,
        })
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

const fn direction_from_script(direction: ScriptDirection) -> Direction {
    match direction {
        ScriptDirection::Up => Direction::Up,
        ScriptDirection::Down => Direction::Down,
        ScriptDirection::Left => Direction::Left,
        ScriptDirection::Right => Direction::Right,
    }
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
