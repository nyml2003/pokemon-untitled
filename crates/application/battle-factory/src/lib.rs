//! 纯应用层对战工厂会话：租借队伍、连胜与战后交换规则。
//!
//! 该 crate 持有确定性工厂状态机，复用 `game-session::roster` 的随机队伍生成
//! 与 `battle-session` 的真实战斗会话。它不创建窗口、不读取输入设备，
//! 也不生成渲染命令；外部只能经 [`FactorySession::transition`] 改变它。

#![forbid(unsafe_code)]

use battle_application::{
    Action, BattleApplication, BattleError, BattleUnit, BattleUnitId, HitPoints, StatStages, Team,
    ValidationError, VolatileStatuses,
};
use battle_session::{
    BattleCoordinator, BattleInteraction, BattleSession, BattleSessionSnapshot,
    ObservedBattleOutcome, OpponentPolicy, Participant, SessionError,
};
use game_data::{CurrentDataSet, PokemonFormId};
use game_session::{DemoSpriteManifest, RosterError, demo_manifest_from_teams, random_team};

/// 租借队伍使用的固定等级。
pub const FACTORY_LEVEL: u8 = 50;

/// 工厂会话的当前阶段。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FactoryPhase {
    /// 未开始或已结束，等待开始下一轮。
    Ready,
    /// 对战中。
    Battle,
    /// 胜利后的交换选择。
    SwapOffer,
    /// 本轮结束（失败或清关）。
    Finished,
}

/// 工厂会话的外部命令。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FactoryCommand {
    /// 用给定种子开始一轮，目标连胜为 `target_streak`。
    StartRun {
        seed: u64,
        target_streak: u32,
    },
    /// 生成下一个对手并进入战斗。
    StartNextBattle,
    SubmitBattleAction(Action),
    AdvanceBattlePlayback,
    /// 在战斗结束后结算结果。
    LeaveFinishedBattle,
    /// 用对方槽位替换租借队伍槽位。
    ConfirmSwap {
        rental_slot: usize,
        opponent_slot: usize,
    },
    /// 跳过本次交换。
    SkipSwap,
}

/// 工厂会话产生的事件。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FactoryEvent {
    RunStarted,
    BattleStarted,
    BattleActionSubmitted,
    BattlePlaybackAdvanced {
        remains: bool,
    },
    BattleResolved {
        won: bool,
    },
    SwapApplied {
        rental_slot: usize,
        opponent_slot: usize,
    },
    RunEnded {
        streak: u32,
        cleared: bool,
    },
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FactoryEvents(Vec<FactoryEvent>);

impl FactoryEvents {
    fn one(event: FactoryEvent) -> Self {
        Self(vec![event])
    }

    fn two(first: FactoryEvent, second: FactoryEvent) -> Self {
        Self(vec![first, second])
    }

    pub fn iter(&self) -> impl Iterator<Item = &FactoryEvent> {
        self.0.iter()
    }
}

/// 工厂会话的只读快照。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactorySnapshot {
    phase: FactoryPhase,
    streak: u32,
    target_streak: u32,
    rental: Vec<FactoryMember>,
    opponent: Option<Vec<FactoryMember>>,
    battle: Option<BattleSessionSnapshot>,
    own_sprite_slot: usize,
    opponent_sprite_slot: usize,
}

impl FactorySnapshot {
    pub const fn phase(&self) -> FactoryPhase {
        self.phase
    }

    pub const fn streak(&self) -> u32 {
        self.streak
    }

    pub const fn target_streak(&self) -> u32 {
        self.target_streak
    }

    pub fn rental(&self) -> &[FactoryMember] {
        &self.rental
    }

    pub fn opponent(&self) -> Option<&[FactoryMember]> {
        self.opponent.as_deref()
    }

    pub const fn battle(&self) -> Option<&BattleSessionSnapshot> {
        self.battle.as_ref()
    }

    pub const fn own_sprite_slot(&self) -> usize {
        self.own_sprite_slot
    }

    pub const fn opponent_sprite_slot(&self) -> usize {
        self.opponent_sprite_slot
    }
}

/// 工厂展示用队伍成员摘要。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactoryMember {
    name: String,
    form: PokemonFormId,
    level: u8,
    current_hp: u32,
    max_hp: u32,
    fainted: bool,
}

impl FactoryMember {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn form(&self) -> PokemonFormId {
        self.form
    }

    pub const fn level(&self) -> u8 {
        self.level
    }

    pub const fn current_hp(&self) -> u32 {
        self.current_hp
    }

    pub const fn max_hp(&self) -> u32 {
        self.max_hp
    }

    pub const fn fainted(&self) -> bool {
        self.fainted
    }
}

pub struct FactorySession {
    data: CurrentDataSet,
    run_seed: u64,
    target_streak: u32,
    rental: Vec<BattleUnit>,
    opponent: Option<Team>,
    battle: Option<BattleSession<FactoryOpponentPolicy>>,
    phase: FactoryPhase,
    streak: u32,
    swap_counter: u64,
}

impl FactorySession {
    pub fn new(data: CurrentDataSet) -> Self {
        Self {
            data,
            run_seed: 0,
            target_streak: 0,
            rental: Vec::new(),
            opponent: None,
            battle: None,
            phase: FactoryPhase::Ready,
            streak: 0,
            swap_counter: 0,
        }
    }

    pub fn snapshot(&self) -> FactorySnapshot {
        let (own_sprite_slot, opponent_sprite_slot) = self.sprite_slots();
        FactorySnapshot {
            phase: self.phase,
            streak: self.streak,
            target_streak: self.target_streak,
            rental: self.rental.iter().map(factory_member).collect(),
            opponent: self
                .opponent
                .as_ref()
                .map(|team| team.members().iter().map(factory_member).collect()),
            battle: self.battle.as_ref().map(BattleSession::snapshot),
            own_sprite_slot,
            opponent_sprite_slot,
        }
    }

    /// 当前双方的精灵资源清单；尚未生成对手时返回 `None`。
    pub fn sprite_manifest(&self) -> Result<Option<DemoSpriteManifest>, FactoryError> {
        let Some(opponent) = self.opponent.as_ref() else {
            return Ok(None);
        };
        let rental = Team::new(self.rental.clone())?;
        Ok(Some(demo_manifest_from_teams(&rental, opponent)))
    }

    pub fn legal_player_actions(&self) -> Vec<Action> {
        self.battle
            .as_ref()
            .filter(|battle| !battle.has_pending_playback() && !battle.is_finished())
            .map_or_else(Vec::new, |battle| battle.legal_actions().to_vec())
    }

    pub fn has_pending_playback(&self) -> bool {
        self.battle
            .as_ref()
            .is_some_and(BattleSession::has_pending_playback)
    }

    pub fn is_finished(&self) -> bool {
        self.battle.as_ref().is_some_and(BattleSession::is_finished)
    }

    pub fn transition(
        mut self,
        command: FactoryCommand,
    ) -> (Self, Result<FactoryEvents, FactoryError>) {
        let result = match command {
            FactoryCommand::StartRun {
                seed,
                target_streak,
            } => self.start_run(seed, target_streak),
            FactoryCommand::StartNextBattle => self.start_next_battle(),
            FactoryCommand::SubmitBattleAction(action) => self.submit_battle_action(action),
            FactoryCommand::AdvanceBattlePlayback => self.advance_battle_playback(),
            FactoryCommand::LeaveFinishedBattle => self.leave_finished_battle(),
            FactoryCommand::ConfirmSwap {
                rental_slot,
                opponent_slot,
            } => self.confirm_swap(rental_slot, opponent_slot),
            FactoryCommand::SkipSwap => self.skip_swap(),
        };
        (self, result)
    }

    fn start_run(&mut self, seed: u64, target_streak: u32) -> Result<FactoryEvents, FactoryError> {
        if !matches!(self.phase, FactoryPhase::Ready | FactoryPhase::Finished) {
            return Err(FactoryError::WrongPhase {
                expected: FactoryPhase::Ready,
                actual: self.phase,
            });
        }
        if target_streak == 0 {
            return Err(FactoryError::InvalidTargetStreak(target_streak));
        }
        let rental = random_team(&self.data, seed, "factory-rental")?;
        self.run_seed = seed;
        self.target_streak = target_streak;
        self.rental = rental.members().to_vec();
        self.opponent = None;
        self.battle = None;
        self.phase = FactoryPhase::Ready;
        self.streak = 0;
        self.swap_counter = 0;
        Ok(FactoryEvents::one(FactoryEvent::RunStarted))
    }

    fn start_next_battle(&mut self) -> Result<FactoryEvents, FactoryError> {
        self.require_phase(FactoryPhase::Ready)?;
        let seed = self.opponent_seed();
        let opponent = random_team(&self.data, seed, "factory-opponent")?;
        let rental = Team::new(self.rental.clone())?;
        let battle_seed = seed ^ 0xA2B3_C4D5;
        let application = BattleApplication::new(rental, opponent.clone(), battle_seed)?;
        let battle =
            BattleSession::new(BattleCoordinator::new(application, FactoryOpponentPolicy))?;
        self.opponent = Some(opponent);
        self.battle = Some(battle);
        self.phase = FactoryPhase::Battle;
        Ok(FactoryEvents::one(FactoryEvent::BattleStarted))
    }

    fn submit_battle_action(&mut self, action: Action) -> Result<FactoryEvents, FactoryError> {
        let battle = self.battle.take().ok_or(FactoryError::BattleMissing)?;
        if battle.has_pending_playback() || battle.is_finished() {
            self.battle = Some(battle);
            return Err(FactoryError::PlayerActionUnavailable);
        }
        let (battle, result) = battle.submit(action);
        self.battle = Some(battle);
        result?;
        Ok(FactoryEvents::one(FactoryEvent::BattleActionSubmitted))
    }

    fn advance_battle_playback(&mut self) -> Result<FactoryEvents, FactoryError> {
        let battle = self.battle.take().ok_or(FactoryError::BattleMissing)?;
        let (battle, advanced) = battle.advance();
        let remains = battle.has_pending_playback();
        self.battle = Some(battle);
        if !advanced? {
            return Err(FactoryError::PlaybackUnavailable);
        }
        Ok(FactoryEvents::one(FactoryEvent::BattlePlaybackAdvanced {
            remains,
        }))
    }

    fn leave_finished_battle(&mut self) -> Result<FactoryEvents, FactoryError> {
        let battle = self.battle.take().ok_or(FactoryError::BattleMissing)?;
        if !battle.is_finished() {
            self.battle = Some(battle);
            return Err(FactoryError::BattleNotFinished);
        }
        let won = matches!(
            battle.snapshot().interaction(),
            BattleInteraction::Finished(prompt)
                if prompt.outcome() == ObservedBattleOutcome::Winner(Participant::Own)
        );
        self.battle = None;
        self.heal_rental();
        if won {
            self.streak = self.streak.saturating_add(1);
            let resolved = FactoryEvent::BattleResolved { won: true };
            if self.streak >= self.target_streak {
                self.phase = FactoryPhase::Finished;
                return Ok(FactoryEvents::two(
                    resolved,
                    FactoryEvent::RunEnded {
                        streak: self.streak,
                        cleared: true,
                    },
                ));
            }
            self.phase = FactoryPhase::SwapOffer;
            return Ok(FactoryEvents::one(resolved));
        }
        self.phase = FactoryPhase::Finished;
        Ok(FactoryEvents::two(
            FactoryEvent::BattleResolved { won: false },
            FactoryEvent::RunEnded {
                streak: self.streak,
                cleared: false,
            },
        ))
    }

    fn confirm_swap(
        &mut self,
        rental_slot: usize,
        opponent_slot: usize,
    ) -> Result<FactoryEvents, FactoryError> {
        self.require_phase(FactoryPhase::SwapOffer)?;
        let opponent = self
            .opponent
            .as_ref()
            .ok_or(FactoryError::OpponentMissing)?;
        let source = opponent
            .members()
            .get(opponent_slot)
            .ok_or(FactoryError::InvalidOpponentSlot(opponent_slot))?;
        let target = self
            .rental
            .get_mut(rental_slot)
            .ok_or(FactoryError::InvalidRentalSlot(rental_slot))?;
        let mut replacement = source.clone();
        let id = BattleUnitId::new(format!("factory-rental-swap-{}", self.swap_counter))?;
        replacement.id = id;
        self.swap_counter = self.swap_counter.saturating_add(1);
        *target = replacement;
        heal_unit(target);
        self.phase = FactoryPhase::Ready;
        Ok(FactoryEvents::one(FactoryEvent::SwapApplied {
            rental_slot,
            opponent_slot,
        }))
    }

    fn skip_swap(&mut self) -> Result<FactoryEvents, FactoryError> {
        self.require_phase(FactoryPhase::SwapOffer)?;
        self.phase = FactoryPhase::Ready;
        Ok(FactoryEvents::default())
    }

    fn require_phase(&self, expected: FactoryPhase) -> Result<(), FactoryError> {
        if self.phase == expected {
            Ok(())
        } else {
            Err(FactoryError::WrongPhase {
                expected,
                actual: self.phase,
            })
        }
    }

    fn sprite_slots(&self) -> (usize, usize) {
        let Some(battle) = self.battle.as_ref() else {
            return (0, 0);
        };
        let snapshot = battle.snapshot();
        let own_id = snapshot.scene().own().id();
        let opponent_id = snapshot.scene().opponent().id();
        let own_slot = self
            .rental
            .iter()
            .position(|unit| unit.id() == own_id)
            .unwrap_or(0);
        let opponent_slot = self
            .opponent
            .as_ref()
            .and_then(|team| {
                team.members()
                    .iter()
                    .position(|unit| unit.id() == opponent_id)
            })
            .unwrap_or(0);
        (own_slot, opponent_slot)
    }
    fn opponent_seed(&self) -> u64 {
        self.run_seed
            ^ u64::from(self.streak)
                .saturating_add(1)
                .wrapping_mul(0x9E37_79B9_7F4A_7C15)
    }

    fn heal_rental(&mut self) {
        for unit in &mut self.rental {
            heal_unit(unit);
        }
    }
}

fn factory_member(unit: &BattleUnit) -> FactoryMember {
    FactoryMember {
        name: unit.name().to_owned(),
        form: PokemonFormId(unit.species().form_id().value()),
        level: unit.level(),
        current_hp: unit.current_hp(),
        max_hp: unit.max_hp(),
        fainted: unit.is_fainted(),
    }
}

fn heal_unit(unit: &mut BattleUnit) {
    let max = unit.state.max_hp();
    unit.state.set_hp(HitPoints::clamped(max, max));
    for battle_move in &mut unit.state.moves {
        battle_move.restore_pp();
    }
    unit.state.major_status = None;
    unit.state.stages = StatStages::neutral();
    unit.state.volatile_statuses = VolatileStatuses::default();
}

struct FactoryOpponentPolicy;

impl OpponentPolicy for FactoryOpponentPolicy {
    fn choose_action(
        &self,
        _observation: &battle_session::BattleObservation,
        legal_actions: &[Action],
    ) -> Option<Action> {
        legal_actions
            .iter()
            .copied()
            .find(|action| matches!(action, Action::UseMove(_)))
            .or_else(|| legal_actions.first().copied())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FactoryError {
    Roster(RosterError),
    Session(SessionError),
    Battle(BattleError),
    Validation(ValidationError),
    InvalidTargetStreak(u32),
    WrongPhase {
        expected: FactoryPhase,
        actual: FactoryPhase,
    },
    BattleMissing,
    PlayerActionUnavailable,
    PlaybackUnavailable,
    BattleNotFinished,
    OpponentMissing,
    InvalidRentalSlot(usize),
    InvalidOpponentSlot(usize),
}

impl From<RosterError> for FactoryError {
    fn from(error: RosterError) -> Self {
        Self::Roster(error)
    }
}

impl From<SessionError> for FactoryError {
    fn from(error: SessionError) -> Self {
        Self::Session(error)
    }
}

impl From<BattleError> for FactoryError {
    fn from(error: BattleError) -> Self {
        Self::Battle(error)
    }
}

impl From<ValidationError> for FactoryError {
    fn from(error: ValidationError) -> Self {
        Self::Validation(error)
    }
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
