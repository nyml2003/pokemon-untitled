use std::collections::BTreeSet;

use battle_ruleset::{BattleRuleset, BattleRulesetRef, RulesetError};
use game_foundation::{
    BattleId, BattleOutcome, CreatureId, CreatureState, CreatureTemplateId, MapId, Money, NpcId,
    Position, ThinSliceContent, TrainerId,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BattleSource {
    Wild { encounter_roll: u8 },
    Trainer { npc: NpcId, trainer: TrainerId },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BattleContinuation {
    map: MapId,
    position: Position,
}

impl BattleContinuation {
    pub fn new(map: MapId, position: Position) -> Self {
        Self { map, position }
    }

    pub fn map(&self) -> &MapId {
        &self.map
    }

    pub const fn position(&self) -> Position {
        self.position
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BattleStartRequest {
    content_version: String,
    ruleset: BattleRulesetRef,
    battle: BattleId,
    source: BattleSource,
    player_party: Vec<CreatureState>,
    participant: CreatureId,
    random_seed: u64,
    continuation: BattleContinuation,
}

impl BattleStartRequest {
    pub fn new(
        content: &ThinSliceContent,
        battle: BattleId,
        source: BattleSource,
        player_party: Vec<CreatureState>,
        participant: CreatureId,
        random_seed: u64,
        continuation: BattleContinuation,
    ) -> Result<Self, BattleContractError> {
        let definition = content
            .battle(&battle)
            .ok_or_else(|| BattleContractError::UnknownBattle(battle.clone()))?;
        validate_source(content, &battle, definition.trainer(), &source)?;
        validate_party(content, &player_party, &participant)?;
        validate_continuation(content, &continuation)?;
        let ruleset = BattleRuleset::legacy_gen3_r1()?.reference().clone();

        Ok(Self {
            content_version: content.content_version().to_owned(),
            ruleset,
            battle,
            source,
            player_party,
            participant,
            random_seed,
            continuation,
        })
    }

    pub fn content_version(&self) -> &str {
        &self.content_version
    }

    pub fn ruleset(&self) -> &BattleRulesetRef {
        &self.ruleset
    }

    pub fn battle(&self) -> &BattleId {
        &self.battle
    }

    pub fn source(&self) -> &BattleSource {
        &self.source
    }

    pub fn player_party(&self) -> &[CreatureState] {
        &self.player_party
    }

    pub fn participant(&self) -> &CreatureId {
        &self.participant
    }

    pub const fn random_seed(&self) -> u64 {
        self.random_seed
    }

    pub fn continuation(&self) -> &BattleContinuation {
        &self.continuation
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveBattleContext {
    request: BattleStartRequest,
}

impl ActiveBattleContext {
    pub fn begin(
        active: Option<&Self>,
        request: BattleStartRequest,
    ) -> Result<Self, BattleContractError> {
        if active.is_some() {
            return Err(BattleContractError::BattleAlreadyActive);
        }
        Ok(Self { request })
    }

    pub fn request(&self) -> &BattleStartRequest {
        &self.request
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BattleParticipantPatch {
    creature: CreatureId,
    hp: u16,
    pp: u8,
}

impl BattleParticipantPatch {
    pub fn creature(&self) -> &CreatureId {
        &self.creature
    }

    pub const fn hp(&self) -> u16 {
        self.hp
    }

    pub const fn pp(&self) -> u8 {
        self.pp
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BattleResultPatch {
    content_version: String,
    ruleset: BattleRulesetRef,
    battle: BattleId,
    outcome: BattleOutcome,
    participant: BattleParticipantPatch,
    experience_reward: u32,
    money_reward: Money,
    defeated_trainer: Option<NpcId>,
    continuation: BattleContinuation,
}

impl BattleResultPatch {
    pub fn from_context(
        content: &ThinSliceContent,
        context: &ActiveBattleContext,
        outcome: BattleOutcome,
        hp: u16,
        pp: u8,
    ) -> Result<Self, BattleContractError> {
        let request = context.request();
        if request.content_version() != content.content_version() {
            return Err(BattleContractError::ContentVersionMismatch {
                expected: content.content_version().to_owned(),
                actual: request.content_version().to_owned(),
            });
        }
        let definition = content
            .battle(request.battle())
            .ok_or_else(|| BattleContractError::UnknownBattle(request.battle().clone()))?;
        let participant = request
            .player_party()
            .iter()
            .find(|creature| creature.id() == request.participant())
            .ok_or_else(|| {
                BattleContractError::ParticipantMissing(request.participant().clone())
            })?;
        validate_creature_state(content, participant, hp, pp)?;

        let (experience_reward, money_reward, defeated_trainer) = match outcome {
            BattleOutcome::Victory => (
                definition.experience_reward(),
                definition.money_reward(),
                definition.trainer().cloned(),
            ),
            BattleOutcome::Defeat => (0, Money::new(0), None),
        };

        Ok(Self {
            content_version: request.content_version().to_owned(),
            ruleset: request.ruleset().clone(),
            battle: request.battle().clone(),
            outcome,
            participant: BattleParticipantPatch {
                creature: participant.id().clone(),
                hp,
                pp,
            },
            experience_reward,
            money_reward,
            defeated_trainer,
            continuation: request.continuation().clone(),
        })
    }

    pub fn content_version(&self) -> &str {
        &self.content_version
    }

    pub fn ruleset(&self) -> &BattleRulesetRef {
        &self.ruleset
    }

    pub fn battle(&self) -> &BattleId {
        &self.battle
    }

    pub const fn outcome(&self) -> BattleOutcome {
        self.outcome
    }

    pub fn participant(&self) -> &BattleParticipantPatch {
        &self.participant
    }

    pub const fn experience_reward(&self) -> u32 {
        self.experience_reward
    }

    pub const fn money_reward(&self) -> Money {
        self.money_reward
    }

    pub fn defeated_trainer(&self) -> Option<&NpcId> {
        self.defeated_trainer.as_ref()
    }

    pub fn continuation(&self) -> &BattleContinuation {
        &self.continuation
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BattleContractError {
    Ruleset(RulesetError),
    UnknownBattle(BattleId),
    UnknownNpc(NpcId),
    UnknownTrainer(TrainerId),
    UnknownCreatureTemplate(CreatureTemplateId),
    UnknownMap(MapId),
    InvalidReturnPosition {
        map: MapId,
        position: Position,
    },
    InvalidEncounterRoll(u8),
    SourceMismatch {
        battle: BattleId,
    },
    EmptyParty,
    DuplicatePartyMember(CreatureId),
    ParticipantMissing(CreatureId),
    InvalidParticipantState {
        creature: CreatureId,
        hp: u16,
        pp: u8,
    },
    ContentVersionMismatch {
        expected: String,
        actual: String,
    },
    BattleAlreadyActive,
}

impl From<RulesetError> for BattleContractError {
    fn from(value: RulesetError) -> Self {
        Self::Ruleset(value)
    }
}

fn validate_source(
    content: &ThinSliceContent,
    battle: &BattleId,
    battle_trainer: Option<&NpcId>,
    source: &BattleSource,
) -> Result<(), BattleContractError> {
    match source {
        BattleSource::Wild { encounter_roll } => {
            if *encounter_roll > 99 {
                return Err(BattleContractError::InvalidEncounterRoll(*encounter_roll));
            }
            if battle_trainer.is_some() {
                return Err(BattleContractError::SourceMismatch {
                    battle: battle.clone(),
                });
            }
        }
        BattleSource::Trainer { npc, trainer } => {
            if content.npc(npc).is_none() {
                return Err(BattleContractError::UnknownNpc(npc.clone()));
            }
            if content.trainer(trainer).is_none() {
                return Err(BattleContractError::UnknownTrainer(trainer.clone()));
            }
            if battle_trainer != Some(npc) {
                return Err(BattleContractError::SourceMismatch {
                    battle: battle.clone(),
                });
            }
        }
    }
    Ok(())
}

fn validate_party(
    content: &ThinSliceContent,
    party: &[CreatureState],
    participant: &CreatureId,
) -> Result<(), BattleContractError> {
    if party.is_empty() {
        return Err(BattleContractError::EmptyParty);
    }
    let mut members = BTreeSet::new();
    for creature in party {
        validate_creature_state(content, creature, creature.hp(), creature.pp())?;
        if !members.insert(creature.id().clone()) {
            return Err(BattleContractError::DuplicatePartyMember(
                creature.id().clone(),
            ));
        }
    }
    if !members.contains(participant) {
        return Err(BattleContractError::ParticipantMissing(participant.clone()));
    }
    Ok(())
}

fn validate_continuation(
    content: &ThinSliceContent,
    continuation: &BattleContinuation,
) -> Result<(), BattleContractError> {
    let map = content
        .map(continuation.map())
        .ok_or_else(|| BattleContractError::UnknownMap(continuation.map().clone()))?;
    if map.tile(continuation.position()).is_none() {
        return Err(BattleContractError::InvalidReturnPosition {
            map: continuation.map().clone(),
            position: continuation.position(),
        });
    }
    Ok(())
}

fn validate_creature_state(
    content: &ThinSliceContent,
    creature: &CreatureState,
    hp: u16,
    pp: u8,
) -> Result<(), BattleContractError> {
    let template = content
        .creature(creature.template())
        .ok_or_else(|| BattleContractError::UnknownCreatureTemplate(creature.template().clone()))?;
    if hp > template.max_hp() || pp > template.max_pp() {
        return Err(BattleContractError::InvalidParticipantState {
            creature: creature.id().clone(),
            hp,
            pp,
        });
    }
    Ok(())
}
