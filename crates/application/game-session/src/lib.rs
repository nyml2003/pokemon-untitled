//! Pure owner of one Gen3 game product session.

#![forbid(unsafe_code)]

mod roster;

use battle_application::{Action, BattleApplication, BattleError, BattleObservation, PokemonId};
use battle_session::{
    BattleCoordinator, BattleSession, BattleSessionSnapshot, OpponentPolicy, SessionError,
};
use game_data::CurrentDataSet;
use world_application::{
    Direction, WorldApplication, WorldApplicationError, WorldError, WorldEvent, WorldObservation,
};

pub use roster::{DemoSpriteManifest, RosterError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameScene {
    World,
    Battle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameCommand {
    FaceWorld(Direction),
    MoveWorld(Direction),
    StepWorld(Direction),
    SubmitBattleAction(Action),
    AdvanceBattlePlayback,
    LeaveFinishedBattle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameEvent {
    World(WorldEvent),
    BattleStarted,
    BattleActionSubmitted,
    BattlePlaybackAdvanced { remains: bool },
    ReturnedToWorld,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GameEvents(Vec<GameEvent>);

impl GameEvents {
    fn one(event: GameEvent) -> Self {
        Self(vec![event])
    }

    fn two(first: GameEvent, second: GameEvent) -> Self {
        Self(vec![first, second])
    }

    pub fn iter(&self) -> impl Iterator<Item = &GameEvent> {
        self.0.iter()
    }

    pub fn world_event(&self) -> Option<WorldEvent> {
        self.0.iter().find_map(|event| match event {
            GameEvent::World(event) => Some(event.clone()),
            _ => None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameSnapshot {
    scene: GameScene,
    world: WorldObservation,
    battle: Option<GameBattleSnapshot>,
}

impl GameSnapshot {
    pub const fn scene(&self) -> GameScene {
        self.scene
    }

    pub const fn world(&self) -> &WorldObservation {
        &self.world
    }

    pub const fn battle(&self) -> Option<&GameBattleSnapshot> {
        self.battle.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameBattleSnapshot {
    session: BattleSessionSnapshot,
    observation: BattleObservation,
    own_sprite_slot: usize,
    opponent_sprite_slot: usize,
}

impl GameBattleSnapshot {
    pub const fn session(&self) -> &BattleSessionSnapshot {
        &self.session
    }

    pub const fn observation(&self) -> &BattleObservation {
        &self.observation
    }

    pub fn is_finished(&self) -> bool {
        matches!(
            self.session.interaction(),
            battle_session::BattleInteraction::Finished(_)
        )
    }

    pub const fn own_sprite_slot(&self) -> usize {
        self.own_sprite_slot
    }

    pub const fn opponent_sprite_slot(&self) -> usize {
        self.opponent_sprite_slot
    }
}

pub struct GameSession {
    data: CurrentDataSet,
    world: WorldApplication,
    world_observation: WorldObservation,
    battle: Option<GameBattleSession>,
    scene: GameScene,
    roster_seed: u64,
}

impl GameSession {
    pub fn new(
        data: CurrentDataSet,
        world: WorldApplication,
        roster_seed: u64,
    ) -> Result<Self, GameError> {
        roster::demo_teams(&data, roster_seed).map_err(GameSetupError::from)?;
        let world_observation = world.observe()?;
        Ok(Self {
            data,
            world,
            world_observation,
            battle: None,
            scene: GameScene::World,
            roster_seed,
        })
    }

    pub fn new_demo(data: CurrentDataSet, roster_seed: u64) -> Result<Self, GameError> {
        Self::new(data, WorldApplication::demo()?, roster_seed)
    }

    pub fn snapshot(&self) -> GameSnapshot {
        GameSnapshot {
            scene: self.scene,
            world: self.world_observation.clone(),
            battle: self.battle.as_ref().map(GameBattleSession::snapshot),
        }
    }

    pub fn sprite_manifest(&self) -> Result<DemoSpriteManifest, GameError> {
        roster::sprite_manifest(&self.data, self.roster_seed)
            .map_err(GameSetupError::from)
            .map_err(Into::into)
    }

    pub fn legal_player_actions(&self) -> Vec<Action> {
        self.battle
            .as_ref()
            .filter(|battle| !battle.has_pending_playback() && !battle.is_finished())
            .map_or_else(Vec::new, GameBattleSession::legal_actions)
    }

    pub fn has_pending_playback(&self) -> bool {
        self.battle
            .as_ref()
            .is_some_and(GameBattleSession::has_pending_playback)
    }

    pub fn transition(mut self, command: GameCommand) -> (Self, Result<GameEvents, GameError>) {
        let result = match command {
            GameCommand::FaceWorld(direction) => self.face_world(direction),
            GameCommand::MoveWorld(direction) => self.move_world(direction),
            GameCommand::StepWorld(direction) => self.step_world(direction),
            GameCommand::SubmitBattleAction(action) => self.submit_battle_action(action),
            GameCommand::AdvanceBattlePlayback => self.advance_battle_playback(),
            GameCommand::LeaveFinishedBattle => self.leave_finished_battle(),
        };
        (self, result)
    }

    /// Advances autonomous world behavior once without accepting player input.
    ///
    /// Runtime hosts own the real-time cadence and call this only while the
    /// world scene is active.
    pub fn advance_world_tick(mut self) -> (Self, Result<GameEvents, GameError>) {
        let result = self.advance_world_tick_mut();
        (self, result)
    }

    fn face_world(&mut self, direction: Direction) -> Result<GameEvents, GameError> {
        self.require_scene(GameScene::World)?;
        let (world, outcome) = self
            .world
            .transition(world_application::WorldCommand::Face(direction));
        self.world = world;
        self.refresh_world_observation()?;
        let event = outcome.event();
        Ok(GameEvents::one(GameEvent::World(event)))
    }

    fn move_world(&mut self, direction: Direction) -> Result<GameEvents, GameError> {
        self.require_scene(GameScene::World)?;
        let (world, outcome) = self
            .world
            .transition(world_application::WorldCommand::Move(direction));
        self.world = world;
        self.refresh_world_observation()?;
        let event = outcome.event();
        if outcome.starts_battle() {
            self.battle = Some(GameBattleSession::new(&self.data, self.roster_seed)?);
            self.scene = GameScene::Battle;
            return Ok(GameEvents::two(
                GameEvent::World(event),
                GameEvent::BattleStarted,
            ));
        }
        Ok(GameEvents::one(GameEvent::World(event)))
    }

    fn step_world(&mut self, direction: Direction) -> Result<GameEvents, GameError> {
        self.require_scene(GameScene::World)?;
        if self.world_observation.facing() == direction {
            self.move_world(direction)
        } else {
            self.face_world(direction)
        }
    }

    fn advance_world_tick_mut(&mut self) -> Result<GameEvents, GameError> {
        self.require_scene(GameScene::World)?;
        self.world = self.world.advance_npcs()?;
        self.refresh_world_observation()?;
        Ok(GameEvents::default())
    }

    fn refresh_world_observation(&mut self) -> Result<(), GameError> {
        self.world_observation = self.world.observe()?;
        Ok(())
    }

    fn submit_battle_action(&mut self, action: Action) -> Result<GameEvents, GameError> {
        self.require_scene(GameScene::Battle)?;
        let battle = self.battle.take().ok_or(GameError::BattleStateMissing)?;
        if battle.has_pending_playback() || battle.is_finished() {
            self.battle = Some(battle);
            return Err(GameError::PlayerActionUnavailable);
        }
        let (battle, result) = battle.submit(action);
        self.battle = Some(battle);
        result?;
        Ok(GameEvents::one(GameEvent::BattleActionSubmitted))
    }

    fn advance_battle_playback(&mut self) -> Result<GameEvents, GameError> {
        self.require_scene(GameScene::Battle)?;
        let battle = self.battle.take().ok_or(GameError::BattleStateMissing)?;
        let (battle, advanced) = battle.advance();
        let remains = battle.has_pending_playback();
        self.battle = Some(battle);
        let advanced = advanced?;
        if !advanced {
            return Err(GameError::PlaybackUnavailable);
        }
        Ok(GameEvents::one(GameEvent::BattlePlaybackAdvanced {
            remains,
        }))
    }

    fn leave_finished_battle(&mut self) -> Result<GameEvents, GameError> {
        self.require_scene(GameScene::Battle)?;
        let battle = self.battle.as_ref().ok_or(GameError::BattleStateMissing)?;
        if !battle.is_finished() {
            return Err(GameError::BattleNotFinished);
        }
        self.battle = None;
        self.scene = GameScene::World;
        Ok(GameEvents::one(GameEvent::ReturnedToWorld))
    }

    fn require_scene(&self, expected: GameScene) -> Result<(), GameError> {
        if self.scene == expected {
            return Ok(());
        }
        Err(GameError::WrongScene {
            expected,
            actual: self.scene,
        })
    }
}

struct GameBattleSession {
    session: BattleSession<DemoOpponentPolicy>,
    own_sprite_ids: Vec<PokemonId>,
    opponent_sprite_ids: Vec<PokemonId>,
}

impl GameBattleSession {
    fn new(data: &CurrentDataSet, roster_seed: u64) -> Result<Self, GameSetupError> {
        let (player_team, opponent_team) = roster::demo_teams(data, roster_seed)?;
        let own_sprite_ids = player_team
            .members()
            .iter()
            .map(|pokemon| pokemon.id().clone())
            .collect();
        let opponent_sprite_ids = opponent_team
            .members()
            .iter()
            .map(|pokemon| pokemon.id().clone())
            .collect();
        let application =
            BattleApplication::new(player_team, opponent_team, roster_seed ^ 0xA2B3_C4D5)?;
        Ok(Self {
            session: BattleSession::new(BattleCoordinator::new(application, DemoOpponentPolicy))?,
            own_sprite_ids,
            opponent_sprite_ids,
        })
    }

    fn snapshot(&self) -> GameBattleSnapshot {
        let session = self.session.snapshot();
        let observation = self.session.settled_observation();
        let own_sprite_slot = sprite_slot(&self.own_sprite_ids, session.scene().own().id());
        let opponent_sprite_slot =
            sprite_slot(&self.opponent_sprite_ids, session.scene().opponent().id());
        GameBattleSnapshot {
            session,
            observation,
            own_sprite_slot,
            opponent_sprite_slot,
        }
    }

    fn legal_actions(&self) -> Vec<Action> {
        self.session.legal_actions().to_vec()
    }

    fn submit(mut self, action: Action) -> (Self, Result<(), SessionError>) {
        let (session, result) = self.session.submit(action);
        self.session = session;
        (self, result)
    }

    fn advance(mut self) -> (Self, Result<bool, SessionError>) {
        let (session, advanced) = self.session.advance();
        self.session = session;
        (self, advanced)
    }

    fn has_pending_playback(&self) -> bool {
        self.session.has_pending_playback()
    }

    fn is_finished(&self) -> bool {
        self.session.is_finished()
    }
}

fn sprite_slot(ids: &[PokemonId], displayed: &PokemonId) -> usize {
    ids.iter().position(|id| id == displayed).unwrap_or(0)
}

struct DemoOpponentPolicy;

impl OpponentPolicy for DemoOpponentPolicy {
    fn choose_action(
        &self,
        _observation: &BattleObservation,
        legal_actions: &[Action],
    ) -> Option<Action> {
        legal_actions
            .iter()
            .copied()
            .find(|action| matches!(action, Action::UseMove(_)))
            .or_else(|| legal_actions.first().copied())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameError {
    World(WorldError),
    WorldApplication(WorldApplicationError),
    Battle(SessionError),
    Setup(GameSetupError),
    PlayerActionUnavailable,
    PlaybackUnavailable,
    BattleStateMissing,
    BattleNotFinished,
    WrongScene {
        expected: GameScene,
        actual: GameScene,
    },
}

impl From<WorldError> for GameError {
    fn from(error: WorldError) -> Self {
        Self::World(error)
    }
}

impl From<WorldApplicationError> for GameError {
    fn from(error: WorldApplicationError) -> Self {
        Self::WorldApplication(error)
    }
}

impl From<SessionError> for GameError {
    fn from(error: SessionError) -> Self {
        Self::Battle(error)
    }
}

impl From<GameSetupError> for GameError {
    fn from(error: GameSetupError) -> Self {
        Self::Setup(error)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameSetupError {
    Roster(RosterError),
    Battle(BattleError),
    Session(SessionError),
}

impl From<RosterError> for GameSetupError {
    fn from(error: RosterError) -> Self {
        Self::Roster(error)
    }
}

impl From<BattleError> for GameSetupError {
    fn from(error: BattleError) -> Self {
        Self::Battle(error)
    }
}

impl From<SessionError> for GameSetupError {
    fn from(error: SessionError) -> Self {
        Self::Session(error)
    }
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
