//! Pure application boundary shared by human and agent battle clients.

#![forbid(unsafe_code)]

mod engine;
mod observation;
mod rules;

pub use battle_domain::{
    Ability, Accuracy, Action, BattleError, BattleOutcome, BattlePhase, BattleStat, BattleState,
    BattleStats, BattleUnit, BattleUnitId, CalculatedStats, EffectTarget, EffortValues, FormId,
    IllegalActionReason, IndividualValues, MAX_EFFORT_VALUE, MAX_INDIVIDUAL_VALUE, MAX_MOVES,
    MAX_STAT_STAGE, MAX_TOTAL_EFFORT_VALUE, MIN_STAT_STAGE, MajorStatus, MajorStatusKind, Move,
    MoveCategory, MoveEffect, MoveId, MoveSlot, NationalDexId, Nature, NonHpStat, PokemonType,
    ReplacementSides, Side, Species, StageChanges, StatBlock, StatName, StatProjectionError,
    StatStages, TEAM_SIZE, Team, TeamSlot, TrainingValues, TypeEffectiveness, ValidationError,
    VolatileStatus, VolatileStatuses, Weather, WeatherAccuracyModifier, WeatherMoveModifier,
    WeatherState, calculate_gen3_stats,
};
pub use observation::{
    BattleEvent, BattleObservation, BattleTransition, DamageProjection, DamageSource,
    ObservedBattleOutcome, OpponentSideObservation, OwnSideObservation, Participant,
    RevealedCombatant, RevealedMoveObservation, RevealedPokemonObservation, SubmitOutcome,
    TransitionError, UsedMove,
};

use battle_domain::{Battle, BattleCommand};
use std::sync::Arc;

/// 提交一条命令并结算战斗状态机。
pub fn submit_battle(
    battle: &mut Battle,
    command: BattleCommand,
) -> Result<battle_domain::SubmitOutcome, BattleError> {
    engine::submit(battle, command)
}

/// 查询指定阵营当前可提交的动作。
pub fn legal_actions_battle(battle: &Battle, side: Side) -> Vec<Action> {
    engine::legal_actions(battle, side)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BattlePerspective {
    side: Side,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BattleCheckpoint {
    owner: Arc<()>,
    viewer: Side,
    event_offset: usize,
    before: BattleObservation,
}

pub struct BattleApplication {
    battle: Battle,
    checkpoint_owner: Arc<()>,
}

impl BattleApplication {
    pub fn new(team_one: Team, team_two: Team, seed: u64) -> Result<Self, BattleError> {
        Ok(Self {
            battle: Battle::new(team_one, team_two, seed)?,
            checkpoint_owner: Arc::new(()),
        })
    }

    pub fn perspectives(&self) -> (BattlePerspective, BattlePerspective) {
        (
            BattlePerspective { side: Side::One },
            BattlePerspective { side: Side::Two },
        )
    }

    pub fn observe(
        &self,
        perspective: &BattlePerspective,
    ) -> Result<BattleObservation, BattleError> {
        observation::observe(&self.battle, perspective.side)
    }

    pub fn legal_actions(&self, perspective: &BattlePerspective) -> Vec<Action> {
        engine::legal_actions(&self.battle, perspective.side)
    }

    pub fn submit(
        &mut self,
        perspective: &BattlePerspective,
        action: Action,
    ) -> Result<SubmitOutcome, BattleError> {
        let viewer = perspective.side;
        let outcome = engine::submit(&mut self.battle, BattleCommand::new(viewer, action))?;
        observation::submit_outcome(&self.battle, outcome, viewer)
    }

    pub fn event_log(
        &self,
        perspective: &BattlePerspective,
    ) -> Result<Vec<BattleEvent>, BattleError> {
        observation::event_log(&self.battle, perspective.side)
    }

    pub fn checkpoint(
        &self,
        perspective: &BattlePerspective,
    ) -> Result<BattleCheckpoint, BattleError> {
        Ok(BattleCheckpoint {
            owner: Arc::clone(&self.checkpoint_owner),
            viewer: perspective.side,
            event_offset: self.battle.events().len(),
            before: self.observe(perspective)?,
        })
    }

    pub fn transition_since(
        &self,
        checkpoint: BattleCheckpoint,
    ) -> Result<BattleTransition, TransitionError> {
        if !Arc::ptr_eq(&checkpoint.owner, &self.checkpoint_owner) {
            return Err(TransitionError::CheckpointOwnerMismatch);
        }
        if checkpoint.event_offset > self.battle.events().len() {
            return Err(TransitionError::EventLogRewound);
        }
        Ok(BattleTransition::new(
            checkpoint.before,
            observation::events_since(&self.battle, checkpoint.viewer, checkpoint.event_offset)
                .map_err(TransitionError::Observation)?,
            observation::observe(&self.battle, checkpoint.viewer)
                .map_err(TransitionError::Observation)?,
        ))
    }
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
