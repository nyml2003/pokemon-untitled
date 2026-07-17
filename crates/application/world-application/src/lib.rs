//! Pure use-case boundary for the overworld.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

pub use map_project::CharacterAppearanceId;
use map_project::{Collision, MapDirection, MapEventKind, MapProject};
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
    UnboundScript { script: narrative_cps::ScriptId },
    ScriptActorMissing { actor: narrative_cps::ActorId },
    ScriptControlsPlayer,
    DuplicateScriptActor { actor: narrative_cps::ActorId },
}

impl From<WorldError> for WorldApplicationError {
    fn from(error: WorldError) -> Self {
        Self::World(error)
    }
}

impl WorldApplication {
    pub fn new(world: World) -> Self {
        let default_appearance =
            CharacterAppearanceId::new("red").expect("the fixed player appearance is valid");
        let appearances = world
            .actors()
            .map(|actor| (actor.id().clone(), default_appearance.clone()))
            .collect();
        Self {
            world,
            appearances,
            npc_scripts: BTreeMap::new(),
            speech: BTreeMap::new(),
        }
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
            let actor_id = WorldActorId::new(&actor.as_str()["actor:".len()..])
                .expect("a validated actor resource has a non-empty name");
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
                    speech: self.speech.get(actor.id()).cloned(),
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
    pub fn advance_npcs(&self) -> Self {
        let mut world = self.world.clone();
        let mut scripts = self.npc_scripts.clone();
        let mut speech = self.speech.clone();
        for (actor, state) in &mut scripts {
            let node = state
                .program
                .continuation(state.continuation)
                .expect("validated script continuations always exist")
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
        Self {
            world,
            appearances: self.appearances.clone(),
            npc_scripts: scripts,
            speech,
        }
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
mod tests {
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

    #[test]
    fn actor_bound_scripts_drive_npc_movement_and_speech() {
        let material = CompositeTile::new(
            CompositeTileId::new("ground").unwrap(),
            vec![AtomicTileId::new("tile-0001").unwrap()],
        );
        let mut project =
            MapProject::blank(MapProjectId::new("demo").unwrap(), 6, 4, Some(material));
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
        let after_player_move = after_player_move.observe();
        let npc_after_player_move = after_player_move
            .actors()
            .iter()
            .find(|actor| actor.id().as_str() == "forest-guide")
            .unwrap();
        assert_eq!(npc_after_player_move.position(), Position::new(3, 1));
        assert_eq!(npc_after_player_move.facing(), Direction::Left);
        assert_eq!(npc_after_player_move.speech(), None);

        let world = world.advance_npcs();
        let observation = world.observe();
        let npc = observation
            .actors()
            .iter()
            .find(|actor| actor.id().as_str() == "forest-guide")
            .unwrap();
        assert_eq!(npc.position(), Position::new(3, 1));
        assert_eq!(npc.speech().unwrap().as_str(), "text:hello");

        let world = world.advance_npcs();
        let npc = world
            .observe()
            .actors()
            .iter()
            .find(|actor| actor.id().as_str() == "forest-guide")
            .unwrap()
            .clone();
        assert_eq!(npc.position(), Position::new(4, 1));
        assert_eq!(npc.facing(), Direction::Right);
        assert_eq!(npc.speech(), None);

        let world = world.advance_npcs().advance_npcs().advance_npcs();
        let observation = world.observe();
        let npc = observation
            .actors()
            .iter()
            .find(|actor| actor.id().as_str() == "forest-guide")
            .unwrap();
        assert_eq!(npc.position(), Position::new(3, 1));
        assert_eq!(npc.facing(), Direction::Up);
    }
}
