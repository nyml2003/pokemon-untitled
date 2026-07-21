use battle_application::{
    Action, BattleApplication, BattleObservation, ObservedBattleOutcome, Participant,
};
use battle_ruleset::{BattleRuleset, BattleRulesetRef, RulesetError};
use battle_session::{
    BattleCoordinator, BattleInteraction, BattleSession, BattleSessionSnapshot, OpponentPolicy,
    SessionError,
};
use game_data::CurrentDataSet;
use game_foundation::{
    BattleOutcome, BattleResolution, ContentPackage, ContentPackageError, ContentPackageManifest,
    Direction, GameCommand as FoundationCommand, GameError as FoundationError,
    GameEvent as FoundationEvent, GameState, ItemId, NpcId, SaveEnvelope, SaveError,
    ThinSliceContent, WarpId,
};

use crate::{
    ActiveBattleContext, BattleContinuation, BattleContractError, BattleResultPatch, BattleSource,
    BattleStartRequest, RosterError, roster,
};

pub enum ProductCommand {
    NewGame,
    Move(Direction),
    InteractFront,
    Interact(NpcId),
    Warp(WarpId),
    BeginEncounter {
        roll: u8,
    },
    Buy {
        npc: NpcId,
        item: ItemId,
        quantity: u16,
    },
    BuyFromFront {
        item: ItemId,
        quantity: u16,
    },
    SubmitBattleAction(Action),
    AdvanceBattlePlayback,
    LeaveFinishedBattle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProductEvent {
    Foundation(FoundationEvent),
    BattleStarted,
    BattleActionSubmitted,
    BattlePlaybackAdvanced {
        remains: bool,
    },
    BattleResolved {
        foundation: FoundationEvent,
        patch: BattleResultPatch,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProductEvents(Vec<ProductEvent>);

impl ProductEvents {
    fn one(event: ProductEvent) -> Self {
        Self(vec![event])
    }

    fn two(first: ProductEvent, second: ProductEvent) -> Self {
        Self(vec![first, second])
    }

    pub fn iter(&self) -> impl Iterator<Item = &ProductEvent> {
        self.0.iter()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductSnapshot {
    state: GameState,
    battle: Option<ProductBattleSnapshot>,
    save_available: bool,
    ruleset: BattleRulesetRef,
    content_package: ContentPackageManifest,
}

impl ProductSnapshot {
    pub fn state(&self) -> &GameState {
        &self.state
    }

    pub fn battle(&self) -> Option<&ProductBattleSnapshot> {
        self.battle.as_ref()
    }

    pub const fn save_available(&self) -> bool {
        self.save_available
    }

    pub fn ruleset(&self) -> &BattleRulesetRef {
        &self.ruleset
    }

    pub fn content_package(&self) -> &ContentPackageManifest {
        &self.content_package
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductBattleSnapshot {
    session: BattleSessionSnapshot,
    observation: BattleObservation,
}

impl ProductBattleSnapshot {
    pub const fn session(&self) -> &BattleSessionSnapshot {
        &self.session
    }

    pub const fn observation(&self) -> &BattleObservation {
        &self.observation
    }

    pub fn is_finished(&self) -> bool {
        matches!(self.session.interaction(), BattleInteraction::Finished(_))
    }
}

pub struct ProductSession {
    data: CurrentDataSet,
    content: ThinSliceContent,
    content_package: ContentPackageManifest,
    ruleset: BattleRuleset,
    state: GameState,
    battle: Option<ProductBattleSession>,
}

impl ProductSession {
    pub fn new(data: CurrentDataSet, content: ThinSliceContent) -> Result<Self, ProductError> {
        Self::from_package(
            data,
            ContentPackage::new(ContentPackageManifest::starter_region()?, content)?,
        )
    }

    pub fn from_package(
        data: CurrentDataSet,
        package: ContentPackage,
    ) -> Result<Self, ProductError> {
        let (manifest, content) = package.into_parts();
        let state = GameState::new(&content)?;
        Self::from_package_state(data, manifest, content, state)
    }

    pub fn from_state(
        data: CurrentDataSet,
        content: ThinSliceContent,
        state: GameState,
    ) -> Result<Self, ProductError> {
        Self::from_package_state(
            data,
            ContentPackageManifest::starter_region()?,
            content,
            state,
        )
    }

    pub fn from_package_state(
        data: CurrentDataSet,
        content_package: ContentPackageManifest,
        content: ThinSliceContent,
        state: GameState,
    ) -> Result<Self, ProductError> {
        let ruleset = BattleRuleset::legacy_gen3_r1()?;
        let expected_ruleset = ruleset.reference().storage_key();
        if content_package.ruleset_reference() != expected_ruleset {
            return Err(ProductError::ContentPackageRulesetMismatch {
                expected: expected_ruleset,
                actual: content_package.ruleset_reference().to_owned(),
            });
        }
        roster::validate_product_content(&data, &content, &ruleset)?;
        state.validate(&content)?;
        if state.active_battle().is_some() {
            return Err(ProductError::SaveUnavailableDuringBattle);
        }
        Ok(Self {
            data,
            content,
            content_package,
            ruleset,
            state,
            battle: None,
        })
    }

    pub fn from_save(
        data: CurrentDataSet,
        content: ThinSliceContent,
        save: SaveEnvelope,
    ) -> Result<Self, ProductError> {
        let package = ContentPackage::new(ContentPackageManifest::starter_region()?, content)?;
        Self::from_package_save(data, package, save)
    }

    pub fn from_package_save(
        data: CurrentDataSet,
        package: ContentPackage,
        save: SaveEnvelope,
    ) -> Result<Self, ProductError> {
        let (content_package, content) = package.into_parts();
        let ruleset = BattleRuleset::legacy_gen3_r1()?;
        let expected = ruleset.reference().storage_key();
        let actual = save
            .ruleset_reference()
            .ok_or(ProductError::RulesetReferenceMissing)?;
        if actual != expected {
            return Err(ProductError::RulesetReferenceMismatch {
                expected,
                actual: actual.to_owned(),
            });
        }
        let expected_package = content_package.storage_key();
        let actual_package = save
            .content_package_reference()
            .ok_or(ProductError::ContentPackageReferenceMissing)?;
        if actual_package != expected_package {
            return Err(ProductError::ContentPackageReferenceMismatch {
                expected: expected_package,
                actual: actual_package.to_owned(),
            });
        }
        Self::from_package_state(data, content_package, content, save.state().clone())
    }

    pub fn snapshot(&self) -> ProductSnapshot {
        ProductSnapshot {
            state: self.state.clone(),
            battle: self.battle.as_ref().map(ProductBattleSession::snapshot),
            save_available: self.battle.is_none(),
            ruleset: self.ruleset.reference().clone(),
            content_package: self.content_package.clone(),
        }
    }

    pub fn save(&self) -> Result<SaveEnvelope, ProductError> {
        if self.battle.is_some() {
            return Err(ProductError::SaveUnavailableDuringBattle);
        }
        SaveEnvelope::from_state_with_references(
            &self.content,
            self.state.clone(),
            self.ruleset.reference().storage_key(),
            self.content_package.storage_key(),
        )
        .map_err(Into::into)
    }

    pub fn legal_player_actions(&self) -> Vec<Action> {
        self.battle
            .as_ref()
            .filter(|battle| {
                !battle.session.has_pending_playback() && !battle.session.is_finished()
            })
            .map_or_else(Vec::new, |battle| battle.session.legal_actions().to_vec())
    }

    pub fn transition(
        mut self,
        command: ProductCommand,
    ) -> (Self, Result<ProductEvents, ProductError>) {
        let result = match command {
            ProductCommand::NewGame => self.apply_foundation(FoundationCommand::NewGame),
            ProductCommand::Move(direction) => {
                self.apply_foundation(FoundationCommand::Move { direction })
            }
            ProductCommand::InteractFront => self.interact_front(),
            ProductCommand::Interact(npc) => self.interact(npc),
            ProductCommand::Warp(warp) => self.apply_foundation(FoundationCommand::Warp { warp }),
            ProductCommand::BeginEncounter { roll } => self.begin_encounter(roll),
            ProductCommand::Buy {
                npc,
                item,
                quantity,
            } => self.apply_foundation(FoundationCommand::Buy {
                npc,
                item,
                quantity,
            }),
            ProductCommand::BuyFromFront { item, quantity } => self.buy_from_front(item, quantity),
            ProductCommand::SubmitBattleAction(action) => self.submit_battle_action(action),
            ProductCommand::AdvanceBattlePlayback => self.advance_battle_playback(),
            ProductCommand::LeaveFinishedBattle => self.leave_finished_battle(),
        };
        (self, result)
    }

    fn apply_foundation(
        &mut self,
        command: FoundationCommand,
    ) -> Result<ProductEvents, ProductError> {
        self.require_no_battle()?;
        let (state, event) = self.state.clone().transition(&self.content, command);
        let event = event?;
        self.state = state;
        Ok(ProductEvents::one(ProductEvent::Foundation(event)))
    }

    fn interact(&mut self, npc: NpcId) -> Result<ProductEvents, ProductError> {
        self.require_no_battle()?;
        let (state, event) = self.state.clone().transition(
            &self.content,
            FoundationCommand::Interact { npc: npc.clone() },
        );
        let event = event?;
        let trainer = match &event {
            FoundationEvent::TrainerBattleStarted { trainer, .. } => trainer.clone(),
            _ => {
                self.state = state;
                return Ok(ProductEvents::one(ProductEvent::Foundation(event)));
            }
        };
        self.start_battle(
            state,
            event,
            BattleSource::Trainer {
                npc,
                trainer: trainer.clone(),
            },
            battle_seed(trainer.as_str().as_bytes()),
        )
    }

    fn interact_front(&mut self) -> Result<ProductEvents, ProductError> {
        let npc = self.front_npc().ok_or(ProductError::NoNpcInFront)?;
        self.interact(npc)
    }

    fn buy_from_front(
        &mut self,
        item: ItemId,
        quantity: u16,
    ) -> Result<ProductEvents, ProductError> {
        let npc = self.front_npc().ok_or(ProductError::NoNpcInFront)?;
        self.apply_foundation(FoundationCommand::Buy {
            npc,
            item,
            quantity,
        })
    }

    fn begin_encounter(&mut self, roll: u8) -> Result<ProductEvents, ProductError> {
        self.require_no_battle()?;
        let (state, event) = self
            .state
            .clone()
            .transition(&self.content, FoundationCommand::Encounter { roll });
        let event = event?;
        self.start_battle(
            state,
            event,
            BattleSource::Wild {
                encounter_roll: roll,
            },
            u64::from(roll),
        )
    }

    fn start_battle(
        &mut self,
        state: GameState,
        event: FoundationEvent,
        source: BattleSource,
        random_seed: u64,
    ) -> Result<ProductEvents, ProductError> {
        let active = state
            .active_battle()
            .ok_or(ProductError::Foundation(FoundationError::BattleMissing))?;
        let request = BattleStartRequest::new(
            &self.content,
            active.battle().clone(),
            source,
            state.party().to_vec(),
            active.participant().clone(),
            random_seed,
            BattleContinuation::new(state.map().clone(), state.position()),
        )?;
        let context = ActiveBattleContext::begin(None, request)?;
        let battle = ProductBattleSession::new(&self.data, &self.content, &self.ruleset, context)?;
        self.state = state;
        self.battle = Some(battle);
        Ok(ProductEvents::two(
            ProductEvent::Foundation(event),
            ProductEvent::BattleStarted,
        ))
    }

    fn submit_battle_action(&mut self, action: Action) -> Result<ProductEvents, ProductError> {
        let battle = self.battle.take().ok_or(ProductError::BattleMissing)?;
        if battle.session.has_pending_playback() || battle.session.is_finished() {
            self.battle = Some(battle);
            return Err(ProductError::PlayerActionUnavailable);
        }
        let (battle, result) = battle.submit(action);
        self.battle = Some(battle);
        result?;
        Ok(ProductEvents::one(ProductEvent::BattleActionSubmitted))
    }

    fn advance_battle_playback(&mut self) -> Result<ProductEvents, ProductError> {
        let battle = self.battle.take().ok_or(ProductError::BattleMissing)?;
        let (battle, advanced) = battle.advance();
        let remains = battle.session.has_pending_playback();
        self.battle = Some(battle);
        if !advanced? {
            return Err(ProductError::PlaybackUnavailable);
        }
        Ok(ProductEvents::one(ProductEvent::BattlePlaybackAdvanced {
            remains,
        }))
    }

    fn leave_finished_battle(&mut self) -> Result<ProductEvents, ProductError> {
        let battle = self.battle.take().ok_or(ProductError::BattleMissing)?;
        if !battle.session.is_finished() {
            self.battle = Some(battle);
            return Err(ProductError::BattleNotFinished);
        }
        let patch = battle.result_patch(&self.content)?;
        let resolution = BattleResolution::new(
            patch.battle().clone(),
            patch.participant().creature().clone(),
            patch.outcome(),
            patch.participant().hp(),
            patch.participant().pp(),
        );
        let (state, event) = self
            .state
            .clone()
            .apply_battle_resolution(&self.content, resolution);
        match event {
            Ok(foundation) => {
                self.state = state;
                Ok(ProductEvents::one(ProductEvent::BattleResolved {
                    foundation,
                    patch,
                }))
            }
            Err(error) => {
                self.battle = Some(battle);
                Err(error.into())
            }
        }
    }

    fn require_no_battle(&self) -> Result<(), ProductError> {
        if self.battle.is_some() {
            return Err(ProductError::StateCommandUnavailableDuringBattle);
        }
        Ok(())
    }

    fn front_npc(&self) -> Option<NpcId> {
        let position = self.state.position();
        let facing = self.state.facing();
        self.content
            .npcs_on_map(self.state.map())
            .find(|npc| {
                let npc_position = npc.actor().position();
                match facing {
                    Direction::Up => {
                        npc_position.x() == position.x()
                            && npc_position.y().checked_add(1) == Some(position.y())
                    }
                    Direction::Down => {
                        npc_position.x() == position.x()
                            && position.y().checked_add(1) == Some(npc_position.y())
                    }
                    Direction::Left => {
                        npc_position.y() == position.y()
                            && npc_position.x().checked_add(1) == Some(position.x())
                    }
                    Direction::Right => {
                        npc_position.y() == position.y()
                            && position.x().checked_add(1) == Some(npc_position.x())
                    }
                }
            })
            .map(|npc| npc.actor().id().clone())
    }
}

struct ProductBattleSession {
    session: BattleSession<ProductOpponentPolicy>,
    context: ActiveBattleContext,
}

impl ProductBattleSession {
    fn new(
        data: &CurrentDataSet,
        content: &ThinSliceContent,
        ruleset: &BattleRuleset,
        context: ActiveBattleContext,
    ) -> Result<Self, ProductError> {
        let (player, opponent) = roster::product_teams(data, content, ruleset, context.request())?;
        let application =
            BattleApplication::new(player, opponent, context.request().random_seed())?;
        let session =
            BattleSession::new(BattleCoordinator::new(application, ProductOpponentPolicy))?;
        Ok(Self { session, context })
    }

    fn snapshot(&self) -> ProductBattleSnapshot {
        ProductBattleSnapshot {
            session: self.session.snapshot(),
            observation: self.session.settled_observation(),
        }
    }

    fn submit(self, action: Action) -> (Self, Result<(), SessionError>) {
        let (session, result) = self.session.submit(action);
        (
            Self {
                session,
                context: self.context,
            },
            result,
        )
    }

    fn advance(self) -> (Self, Result<bool, SessionError>) {
        let (session, result) = self.session.advance();
        (
            Self {
                session,
                context: self.context,
            },
            result,
        )
    }

    fn result_patch(&self, content: &ThinSliceContent) -> Result<BattleResultPatch, ProductError> {
        let outcome = match self.session.snapshot().interaction() {
            BattleInteraction::Finished(prompt) => match prompt.outcome() {
                ObservedBattleOutcome::Winner(Participant::Own) => BattleOutcome::Victory,
                ObservedBattleOutcome::Winner(Participant::Opponent)
                | ObservedBattleOutcome::Escaped(_)
                | ObservedBattleOutcome::Draw => BattleOutcome::Defeat,
            },
            _ => return Err(ProductError::BattleNotFinished),
        };
        let observation = self.session.settled_observation();
        let participant = observation
            .own()
            .members()
            .first()
            .ok_or(ProductError::BattleObservationMissing)?;
        let move_ = participant
            .moves()
            .first()
            .ok_or(ProductError::BattleObservationMissing)?;
        let hp = u16::try_from(participant.current_hp())
            .map_err(|_| ProductError::BattleValueOutOfRange(participant.current_hp()))?;
        BattleResultPatch::from_context(content, &self.context, outcome, hp, move_.current_pp())
            .map_err(Into::into)
    }
}

struct ProductOpponentPolicy;

impl OpponentPolicy for ProductOpponentPolicy {
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

fn battle_seed(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |value, byte| {
        (value ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[derive(Debug)]
pub enum ProductError {
    Foundation(FoundationError),
    Contract(BattleContractError),
    Roster(RosterError),
    Battle(battle_application::BattleError),
    Session(SessionError),
    Save(SaveError),
    ContentPackage(ContentPackageError),
    Ruleset(RulesetError),
    RulesetReferenceMissing,
    RulesetReferenceMismatch { expected: String, actual: String },
    ContentPackageReferenceMissing,
    ContentPackageReferenceMismatch { expected: String, actual: String },
    ContentPackageRulesetMismatch { expected: String, actual: String },
    StateCommandUnavailableDuringBattle,
    SaveUnavailableDuringBattle,
    NoNpcInFront,
    BattleMissing,
    PlayerActionUnavailable,
    PlaybackUnavailable,
    BattleNotFinished,
    BattleObservationMissing,
    BattleValueOutOfRange(u32),
}

impl From<FoundationError> for ProductError {
    fn from(value: FoundationError) -> Self {
        Self::Foundation(value)
    }
}

impl From<BattleContractError> for ProductError {
    fn from(value: BattleContractError) -> Self {
        Self::Contract(value)
    }
}

impl From<RosterError> for ProductError {
    fn from(value: RosterError) -> Self {
        Self::Roster(value)
    }
}

impl From<battle_application::BattleError> for ProductError {
    fn from(value: battle_application::BattleError) -> Self {
        Self::Battle(value)
    }
}

impl From<SessionError> for ProductError {
    fn from(value: SessionError) -> Self {
        Self::Session(value)
    }
}

impl From<SaveError> for ProductError {
    fn from(value: SaveError) -> Self {
        Self::Save(value)
    }
}

impl From<ContentPackageError> for ProductError {
    fn from(value: ContentPackageError) -> Self {
        Self::ContentPackage(value)
    }
}

impl From<RulesetError> for ProductError {
    fn from(value: RulesetError) -> Self {
        Self::Ruleset(value)
    }
}
