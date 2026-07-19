use battle_domain::{
    Ability, Action, Battle, BattleError, BattleEvent as DomainEvent,
    BattleOutcome as DomainBattleOutcome, BattlePhase, BattleStat,
    DamageSource as DomainDamageSource, MajorStatus, MajorStatusKind, Move, MoveCategory, MoveId,
    MoveSlot, Pokemon, PokemonId, PokemonType, Side, StatStages,
    SubmitOutcome as DomainSubmitOutcome, TEAM_SIZE, TeamSlot, TypeEffectiveness,
    UsedMove as DomainUsedMove, Weather, WeatherState,
};

use crate::Accuracy;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Participant {
    Own,
    Opponent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservedBattleOutcome {
    Winner(Participant),
    Escaped(Participant),
    Draw,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BattleObservation {
    viewer: Side,
    turn: u32,
    phase: BattlePhase,
    weather: Option<WeatherState>,
    own: OwnSideObservation,
    opponent: OpponentSideObservation,
}

impl BattleObservation {
    pub const fn viewer(&self) -> Side {
        self.viewer
    }

    pub const fn turn(&self) -> u32 {
        self.turn
    }

    pub const fn phase(&self) -> BattlePhase {
        self.phase
    }

    pub const fn weather(&self) -> Option<WeatherState> {
        self.weather
    }

    pub const fn own(&self) -> &OwnSideObservation {
        &self.own
    }

    pub const fn opponent(&self) -> &OpponentSideObservation {
        &self.opponent
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnSideObservation {
    active_slot: TeamSlot,
    members: [Pokemon; TEAM_SIZE],
}

impl OwnSideObservation {
    pub const fn active_slot(&self) -> TeamSlot {
        self.active_slot
    }

    pub const fn members(&self) -> &[Pokemon; TEAM_SIZE] {
        &self.members
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpponentSideObservation {
    active: RevealedPokemonObservation,
    revealed_bench: Vec<RevealedPokemonObservation>,
    unrevealed_count: usize,
}

impl OpponentSideObservation {
    pub const fn active(&self) -> &RevealedPokemonObservation {
        &self.active
    }

    pub fn revealed_bench(&self) -> &[RevealedPokemonObservation] {
        &self.revealed_bench
    }

    pub const fn unrevealed_count(&self) -> usize {
        self.unrevealed_count
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevealedPokemonObservation {
    id: PokemonId,
    name: String,
    level: u8,
    primary_type: PokemonType,
    secondary_type: Option<PokemonType>,
    max_hp: u32,
    current_hp: u32,
    substitute_hp: Option<u32>,
    major_status: Option<MajorStatus>,
    stages: StatStages,
    revealed_moves: Vec<RevealedMoveObservation>,
}

impl RevealedPokemonObservation {
    pub fn id(&self) -> &PokemonId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn level(&self) -> u8 {
        self.level
    }

    pub const fn primary_type(&self) -> PokemonType {
        self.primary_type
    }

    pub const fn secondary_type(&self) -> Option<PokemonType> {
        self.secondary_type
    }

    pub const fn max_hp(&self) -> u32 {
        self.max_hp
    }

    pub const fn current_hp(&self) -> u32 {
        self.current_hp
    }

    pub const fn substitute_hp(&self) -> Option<u32> {
        self.substitute_hp
    }

    pub const fn major_status(&self) -> Option<MajorStatus> {
        self.major_status
    }

    pub const fn stages(&self) -> StatStages {
        self.stages
    }

    pub const fn is_fainted(&self) -> bool {
        self.current_hp == 0
    }

    pub fn revealed_moves(&self) -> &[RevealedMoveObservation] {
        &self.revealed_moves
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevealedCombatant {
    id: PokemonId,
    name: String,
    level: u8,
    primary_type: PokemonType,
    secondary_type: Option<PokemonType>,
    max_hp: u32,
    current_hp: u32,
    substitute_hp: Option<u32>,
    major_status: Option<MajorStatus>,
    stages: StatStages,
}

impl RevealedCombatant {
    pub fn id(&self) -> &PokemonId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn level(&self) -> u8 {
        self.level
    }

    pub const fn primary_type(&self) -> PokemonType {
        self.primary_type
    }

    pub const fn secondary_type(&self) -> Option<PokemonType> {
        self.secondary_type
    }

    pub const fn max_hp(&self) -> u32 {
        self.max_hp
    }

    pub const fn current_hp(&self) -> u32 {
        self.current_hp
    }

    pub const fn substitute_hp(&self) -> Option<u32> {
        self.substitute_hp
    }

    pub const fn major_status(&self) -> Option<MajorStatus> {
        self.major_status
    }

    pub const fn stages(&self) -> StatStages {
        self.stages
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevealedMoveObservation {
    id: MoveId,
    name: String,
    move_type: PokemonType,
    category: MoveCategory,
    power: u16,
    accuracy: Accuracy,
    priority: i8,
}

impl RevealedMoveObservation {
    pub fn id(&self) -> &MoveId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn move_type(&self) -> PokemonType {
        self.move_type
    }

    pub const fn category(&self) -> MoveCategory {
        self.category
    }

    pub const fn power(&self) -> u16 {
        self.power
    }

    pub const fn accuracy(&self) -> Accuracy {
        self.accuracy
    }

    pub const fn priority(&self) -> i8 {
        self.priority
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UsedMove {
    Move { id: MoveId, name: String },
    Struggle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DamageSource {
    Move {
        participant: Participant,
        pokemon: PokemonId,
        used_move: UsedMove,
    },
    Recoil {
        participant: Participant,
        pokemon: PokemonId,
    },
    Ability {
        participant: Participant,
        pokemon: PokemonId,
        ability: Ability,
    },
    Status {
        participant: Participant,
        pokemon: PokemonId,
        status: MajorStatus,
    },
    Weather {
        weather: Weather,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BattleEvent {
    OwnCommandAccepted {
        action: Action,
    },
    OpponentCommandCommitted,
    TurnStarted {
        turn: u32,
    },
    OwnSwitched {
        from: TeamSlot,
        to: TeamSlot,
        pokemon: RevealedCombatant,
    },
    OpponentSwitched {
        pokemon: RevealedCombatant,
    },
    MoveUsed {
        participant: Participant,
        pokemon: PokemonId,
        used_move: UsedMove,
    },
    OwnPpSpent {
        pokemon: PokemonId,
        move_slot: MoveSlot,
        remaining: u8,
    },
    StatusApplied {
        participant: Participant,
        pokemon: PokemonId,
        status: MajorStatus,
    },
    StatusFailed {
        participant: Participant,
        target: Participant,
        pokemon: PokemonId,
        status: MajorStatusKind,
    },
    StatusPreventsAction {
        participant: Participant,
        pokemon: PokemonId,
        status: MajorStatus,
    },
    StatusCured {
        participant: Participant,
        pokemon: PokemonId,
        status: MajorStatusKind,
    },
    StatusAdvanced {
        participant: Participant,
        pokemon: PokemonId,
        status: MajorStatus,
    },
    ProtectionActivated {
        participant: Participant,
        pokemon: PokemonId,
    },
    ProtectionFailed {
        participant: Participant,
        pokemon: PokemonId,
    },
    MoveBlocked {
        participant: Participant,
        target: Participant,
        pokemon: PokemonId,
    },
    SubstituteCreated {
        participant: Participant,
        pokemon: PokemonId,
        substitute_hp: u32,
        current_hp: u32,
    },
    SubstituteBlocked {
        participant: Participant,
        target: Participant,
        pokemon: PokemonId,
    },
    SubstituteDamaged {
        participant: Participant,
        pokemon: PokemonId,
        amount: u32,
        remaining_hp: u32,
    },
    SubstituteBroke {
        participant: Participant,
        pokemon: PokemonId,
    },
    WeatherStarted {
        weather: Weather,
        turns_remaining: Option<u8>,
    },
    WeatherUpdated {
        weather: Weather,
        turns_remaining: u8,
    },
    WeatherEnded {
        weather: Weather,
    },
    AbilityActivated {
        participant: Participant,
        pokemon: PokemonId,
        ability: Ability,
    },
    Flinched {
        participant: Participant,
        pokemon: PokemonId,
    },
    StatStageChanged {
        participant: Participant,
        pokemon: PokemonId,
        stat: BattleStat,
        change: i8,
        stage: i8,
    },
    Healed {
        participant: Participant,
        pokemon: PokemonId,
        amount: u32,
        current_hp: u32,
    },
    EffectFailed {
        participant: Participant,
        target: Participant,
        pokemon: PokemonId,
    },
    Missed {
        participant: Participant,
        target: Participant,
        pokemon: PokemonId,
    },
    Critical {
        participant: Participant,
        target: Participant,
        pokemon: PokemonId,
    },
    Effectiveness {
        participant: Participant,
        target: Participant,
        pokemon: PokemonId,
        effectiveness: TypeEffectiveness,
    },
    Damage {
        source: DamageSource,
        target: Participant,
        pokemon: PokemonId,
        amount: u32,
        remaining_hp: u32,
    },
    Fainted {
        participant: Participant,
        pokemon: PokemonId,
    },
    ForcedReplacement {
        participant: Participant,
    },
    BattleFinished {
        outcome: ObservedBattleOutcome,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BattleTransition {
    before: BattleObservation,
    events: Vec<BattleEvent>,
    after: BattleObservation,
}

impl BattleTransition {
    pub(crate) fn new(
        before: BattleObservation,
        events: Vec<BattleEvent>,
        after: BattleObservation,
    ) -> Self {
        debug_assert_eq!(before.viewer(), after.viewer());
        Self {
            before,
            events,
            after,
        }
    }

    pub const fn before(&self) -> &BattleObservation {
        &self.before
    }

    pub fn events(&self) -> &[BattleEvent] {
        &self.events
    }

    pub const fn after(&self) -> &BattleObservation {
        &self.after
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransitionError {
    CheckpointOwnerMismatch,
    EventLogRewound,
    Observation(BattleError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubmitOutcome {
    events: Vec<BattleEvent>,
    phase: BattlePhase,
    waiting_for_opponent: bool,
}

impl SubmitOutcome {
    pub fn events(&self) -> &[BattleEvent] {
        &self.events
    }

    pub const fn phase(&self) -> BattlePhase {
        self.phase
    }

    pub const fn is_waiting_for_opponent(&self) -> bool {
        self.waiting_for_opponent
    }
}

pub(crate) fn submit_outcome(
    battle: &Battle,
    outcome: DomainSubmitOutcome,
    viewer: Side,
) -> Result<SubmitOutcome, BattleError> {
    Ok(SubmitOutcome {
        events: observe_events(battle, outcome.events(), viewer)?,
        phase: outcome.phase(),
        waiting_for_opponent: outcome.is_waiting_for_opponent(),
    })
}

pub(crate) fn observe(battle: &Battle, viewer: Side) -> Result<BattleObservation, BattleError> {
    let opponent = viewer.opponent();
    Ok(BattleObservation {
        viewer,
        turn: battle.turn_number(),
        phase: battle.phase(),
        weather: battle.weather(),
        own: OwnSideObservation {
            active_slot: battle.active_slot(viewer),
            members: battle.team(viewer).members().clone(),
        },
        opponent: opponent_observation(battle, opponent)?,
    })
}

pub(crate) fn event_log(battle: &Battle, viewer: Side) -> Result<Vec<BattleEvent>, BattleError> {
    observe_events(battle, battle.events(), viewer)
}

pub(crate) fn events_since(
    battle: &Battle,
    viewer: Side,
    event_offset: usize,
) -> Result<Vec<BattleEvent>, BattleError> {
    observe_events(battle, &battle.events()[event_offset..], viewer)
}

fn opponent_observation(
    battle: &Battle,
    opponent: Side,
) -> Result<OpponentSideObservation, BattleError> {
    let active = battle.active(opponent);
    let revealed = revealed_pokemon_ids(battle, opponent);
    let revealed_bench = revealed
        .iter()
        .filter(|id| *id != active.id())
        .map(|id| revealed_pokemon(battle, opponent, id))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(OpponentSideObservation {
        active: revealed_pokemon(battle, opponent, active.id())?,
        revealed_bench,
        unrevealed_count: TEAM_SIZE - revealed.len(),
    })
}

fn revealed_pokemon_ids(battle: &Battle, side: Side) -> Vec<PokemonId> {
    let mut revealed = Vec::new();
    for event in battle.events() {
        match event {
            DomainEvent::Switched {
                side: event_side,
                from,
                pokemon,
                ..
            } if *event_side == side => {
                push_unique(&mut revealed, battle.team(side).member(*from).id().clone());
                push_unique(&mut revealed, pokemon.clone());
            }
            DomainEvent::MoveUsed {
                side: event_side,
                pokemon,
                ..
            }
            | DomainEvent::Fainted {
                side: event_side,
                pokemon,
            } if *event_side == side => push_unique(&mut revealed, pokemon.clone()),
            _ => {}
        }
    }
    push_unique(&mut revealed, battle.active(side).id().clone());
    revealed
}

fn push_unique(revealed: &mut Vec<PokemonId>, pokemon: PokemonId) {
    if !revealed.contains(&pokemon) {
        revealed.push(pokemon);
    }
}

fn revealed_pokemon(
    battle: &Battle,
    side: Side,
    pokemon_id: &PokemonId,
) -> Result<RevealedPokemonObservation, BattleError> {
    let pokemon = battle
        .team(side)
        .members()
        .iter()
        .find(|pokemon| pokemon.id() == pokemon_id)
        .ok_or(BattleError::StateInconsistent {
            detail: "revealed pokemon does not belong to the observed team",
        })?;
    Ok(RevealedPokemonObservation {
        id: pokemon.id().clone(),
        name: pokemon.name().to_owned(),
        level: pokemon.level(),
        primary_type: pokemon.primary_type(),
        secondary_type: pokemon.secondary_type(),
        max_hp: pokemon.max_hp(),
        current_hp: pokemon.current_hp(),
        substitute_hp: pokemon.substitute_hp(),
        major_status: pokemon.major_status(),
        stages: pokemon.stages(),
        revealed_moves: revealed_moves(battle, side, pokemon)?,
    })
}

fn revealed_pokemon_at(
    battle: &Battle,
    side: Side,
    pokemon_id: &PokemonId,
    current_hp: u32,
) -> Result<RevealedCombatant, BattleError> {
    let pokemon = battle
        .team(side)
        .members()
        .iter()
        .find(|pokemon| pokemon.id() == pokemon_id)
        .ok_or(BattleError::StateInconsistent {
            detail: "switched pokemon does not belong to its team",
        })?;
    Ok(RevealedCombatant {
        id: pokemon.id().clone(),
        name: pokemon.name().to_owned(),
        level: pokemon.level(),
        primary_type: pokemon.primary_type(),
        secondary_type: pokemon.secondary_type(),
        max_hp: pokemon.max_hp(),
        current_hp,
        substitute_hp: pokemon.substitute_hp(),
        major_status: pokemon.major_status(),
        stages: pokemon.stages(),
    })
}

fn revealed_moves(
    battle: &Battle,
    side: Side,
    pokemon: &Pokemon,
) -> Result<Vec<RevealedMoveObservation>, BattleError> {
    pokemon
        .moves()
        .iter()
        .enumerate()
        .map(|(index, battle_move)| {
            MoveSlot::new(index)
                .map_err(|_| BattleError::StateInconsistent {
                    detail: "pokemon has a move outside the supported move slots",
                })
                .map(|slot| {
                    move_was_used(battle, side, pokemon.id(), slot)
                        .then(|| reveal_move(battle_move))
                })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|moves| moves.into_iter().flatten().collect())
}

fn move_was_used(battle: &Battle, side: Side, pokemon: &PokemonId, slot: MoveSlot) -> bool {
    battle.events().iter().any(|event| {
        matches!(
            event,
            DomainEvent::MoveUsed {
                side: event_side,
                pokemon: event_pokemon,
                used_move: DomainUsedMove::Move { slot: event_slot, .. },
            } if *event_side == side && event_pokemon == pokemon && *event_slot == slot
        )
    })
}

fn reveal_move(battle_move: &Move) -> RevealedMoveObservation {
    RevealedMoveObservation {
        id: battle_move.id().clone(),
        name: battle_move.name().to_owned(),
        move_type: battle_move.move_type(),
        category: battle_move.category(),
        power: battle_move.power(),
        accuracy: battle_move.accuracy(),
        priority: battle_move.priority(),
    }
}

fn observe_events(
    battle: &Battle,
    events: &[DomainEvent],
    viewer: Side,
) -> Result<Vec<BattleEvent>, BattleError> {
    events
        .iter()
        .map(|event| observe_event(battle, event, viewer))
        .filter_map(Result::transpose)
        .collect()
}

fn observe_event(
    battle: &Battle,
    event: &DomainEvent,
    viewer: Side,
) -> Result<Option<BattleEvent>, BattleError> {
    Ok(Some(match event {
        DomainEvent::CommandAccepted { side, action } if *side == viewer => {
            BattleEvent::OwnCommandAccepted { action: *action }
        }
        DomainEvent::CommandAccepted { .. } => BattleEvent::OpponentCommandCommitted,
        DomainEvent::TurnStarted { turn } => BattleEvent::TurnStarted { turn: *turn },
        DomainEvent::Switched {
            side,
            from,
            to,
            pokemon,
            current_hp,
        } if *side == viewer => BattleEvent::OwnSwitched {
            from: *from,
            to: *to,
            pokemon: revealed_pokemon_at(battle, *side, pokemon, *current_hp)?,
        },
        DomainEvent::Switched {
            side,
            pokemon,
            current_hp,
            ..
        } => BattleEvent::OpponentSwitched {
            pokemon: revealed_pokemon_at(battle, *side, pokemon, *current_hp)?,
        },
        DomainEvent::MoveUsed {
            side,
            pokemon,
            used_move,
        } => BattleEvent::MoveUsed {
            participant: participant(*side, viewer),
            pokemon: pokemon.clone(),
            used_move: observe_used_move(battle, *side, pokemon, used_move)?,
        },
        DomainEvent::PpSpent { side, .. } if *side != viewer => return Ok(None),
        DomainEvent::PpSpent {
            side: _,
            pokemon,
            move_slot,
            remaining,
        } => BattleEvent::OwnPpSpent {
            pokemon: pokemon.clone(),
            move_slot: *move_slot,
            remaining: *remaining,
        },
        DomainEvent::StatusApplied {
            side,
            pokemon,
            status,
        } => BattleEvent::StatusApplied {
            participant: participant(*side, viewer),
            pokemon: pokemon.clone(),
            status: *status,
        },
        DomainEvent::StatusFailed {
            side,
            target_side,
            target,
            status,
        } => BattleEvent::StatusFailed {
            participant: participant(*side, viewer),
            target: participant(*target_side, viewer),
            pokemon: target.clone(),
            status: *status,
        },
        DomainEvent::StatusPreventsAction {
            side,
            pokemon,
            status,
        } => BattleEvent::StatusPreventsAction {
            participant: participant(*side, viewer),
            pokemon: pokemon.clone(),
            status: *status,
        },
        DomainEvent::StatusCured {
            side,
            pokemon,
            status,
        } => BattleEvent::StatusCured {
            participant: participant(*side, viewer),
            pokemon: pokemon.clone(),
            status: *status,
        },
        DomainEvent::StatusAdvanced {
            side,
            pokemon,
            status,
        } => BattleEvent::StatusAdvanced {
            participant: participant(*side, viewer),
            pokemon: pokemon.clone(),
            status: *status,
        },
        DomainEvent::ProtectionActivated { side, pokemon } => BattleEvent::ProtectionActivated {
            participant: participant(*side, viewer),
            pokemon: pokemon.clone(),
        },
        DomainEvent::ProtectionFailed { side, pokemon } => BattleEvent::ProtectionFailed {
            participant: participant(*side, viewer),
            pokemon: pokemon.clone(),
        },
        DomainEvent::MoveBlocked {
            side,
            target_side,
            target,
        } => BattleEvent::MoveBlocked {
            participant: participant(*side, viewer),
            target: participant(*target_side, viewer),
            pokemon: target.clone(),
        },
        DomainEvent::SubstituteCreated {
            side,
            pokemon,
            substitute_hp,
            current_hp,
        } => BattleEvent::SubstituteCreated {
            participant: participant(*side, viewer),
            pokemon: pokemon.clone(),
            substitute_hp: *substitute_hp,
            current_hp: *current_hp,
        },
        DomainEvent::SubstituteBlocked {
            side,
            target_side,
            target,
        } => BattleEvent::SubstituteBlocked {
            participant: participant(*side, viewer),
            target: participant(*target_side, viewer),
            pokemon: target.clone(),
        },
        DomainEvent::SubstituteDamaged {
            side,
            pokemon,
            amount,
            remaining_hp,
        } => BattleEvent::SubstituteDamaged {
            participant: participant(*side, viewer),
            pokemon: pokemon.clone(),
            amount: *amount,
            remaining_hp: *remaining_hp,
        },
        DomainEvent::SubstituteBroke { side, pokemon } => BattleEvent::SubstituteBroke {
            participant: participant(*side, viewer),
            pokemon: pokemon.clone(),
        },
        DomainEvent::WeatherStarted {
            weather,
            turns_remaining,
        } => BattleEvent::WeatherStarted {
            weather: *weather,
            turns_remaining: *turns_remaining,
        },
        DomainEvent::WeatherUpdated {
            weather,
            turns_remaining,
        } => BattleEvent::WeatherUpdated {
            weather: *weather,
            turns_remaining: *turns_remaining,
        },
        DomainEvent::WeatherEnded { weather } => BattleEvent::WeatherEnded { weather: *weather },
        DomainEvent::AbilityActivated {
            side,
            pokemon,
            ability,
        } => BattleEvent::AbilityActivated {
            participant: participant(*side, viewer),
            pokemon: pokemon.clone(),
            ability: *ability,
        },
        DomainEvent::Flinched { side, pokemon } => BattleEvent::Flinched {
            participant: participant(*side, viewer),
            pokemon: pokemon.clone(),
        },
        DomainEvent::StatStageChanged {
            side,
            pokemon,
            stat,
            change,
            stage,
        } => BattleEvent::StatStageChanged {
            participant: participant(*side, viewer),
            pokemon: pokemon.clone(),
            stat: *stat,
            change: *change,
            stage: *stage,
        },
        DomainEvent::Healed {
            side,
            pokemon,
            amount,
            current_hp,
        } => BattleEvent::Healed {
            participant: participant(*side, viewer),
            pokemon: pokemon.clone(),
            amount: *amount,
            current_hp: *current_hp,
        },
        DomainEvent::EffectFailed {
            side,
            target_side,
            target,
        } => BattleEvent::EffectFailed {
            participant: participant(*side, viewer),
            target: participant(*target_side, viewer),
            pokemon: target.clone(),
        },
        DomainEvent::Missed {
            side,
            target_side,
            target,
        } => BattleEvent::Missed {
            participant: participant(*side, viewer),
            target: participant(*target_side, viewer),
            pokemon: target.clone(),
        },
        DomainEvent::Critical {
            side,
            target_side,
            target,
        } => BattleEvent::Critical {
            participant: participant(*side, viewer),
            target: participant(*target_side, viewer),
            pokemon: target.clone(),
        },
        DomainEvent::Effectiveness {
            side,
            target_side,
            target,
            effectiveness,
        } => BattleEvent::Effectiveness {
            participant: participant(*side, viewer),
            target: participant(*target_side, viewer),
            pokemon: target.clone(),
            effectiveness: *effectiveness,
        },
        DomainEvent::Damage {
            source,
            target_side,
            target,
            amount,
            remaining_hp,
        } => BattleEvent::Damage {
            source: observe_damage_source(battle, source, viewer)?,
            target: participant(*target_side, viewer),
            pokemon: target.clone(),
            amount: *amount,
            remaining_hp: *remaining_hp,
        },
        DomainEvent::Fainted { side, pokemon } => BattleEvent::Fainted {
            participant: participant(*side, viewer),
            pokemon: pokemon.clone(),
        },
        DomainEvent::ForcedReplacement { side } => BattleEvent::ForcedReplacement {
            participant: participant(*side, viewer),
        },
        DomainEvent::BattleFinished { outcome } => BattleEvent::BattleFinished {
            outcome: observe_outcome(*outcome, viewer),
        },
    }))
}

fn observe_used_move(
    battle: &Battle,
    side: Side,
    pokemon: &PokemonId,
    used_move: &DomainUsedMove,
) -> Result<UsedMove, BattleError> {
    match used_move {
        DomainUsedMove::Move { id, .. } => {
            let name = battle
                .team(side)
                .members()
                .iter()
                .find(|member| member.id() == pokemon)
                .and_then(|member| {
                    member
                        .moves()
                        .iter()
                        .find(|battle_move| battle_move.id() == id)
                })
                .ok_or(BattleError::StateInconsistent {
                    detail: "move-used event references a move not owned by its combatant",
                })?
                .name()
                .to_owned();
            Ok(UsedMove::Move {
                id: id.clone(),
                name,
            })
        }
        DomainUsedMove::Struggle => Ok(UsedMove::Struggle),
    }
}

fn observe_damage_source(
    battle: &Battle,
    source: &DomainDamageSource,
    viewer: Side,
) -> Result<DamageSource, BattleError> {
    Ok(match source {
        DomainDamageSource::Move {
            side,
            pokemon,
            used_move,
        } => DamageSource::Move {
            participant: participant(*side, viewer),
            pokemon: pokemon.clone(),
            used_move: observe_used_move(battle, *side, pokemon, used_move)?,
        },
        DomainDamageSource::Recoil { side, pokemon } => DamageSource::Recoil {
            participant: participant(*side, viewer),
            pokemon: pokemon.clone(),
        },
        DomainDamageSource::Ability {
            side,
            pokemon,
            ability,
        } => DamageSource::Ability {
            participant: participant(*side, viewer),
            pokemon: pokemon.clone(),
            ability: *ability,
        },
        DomainDamageSource::Status {
            side,
            pokemon,
            status,
        } => DamageSource::Status {
            participant: participant(*side, viewer),
            pokemon: pokemon.clone(),
            status: *status,
        },
        DomainDamageSource::Weather { weather } => DamageSource::Weather { weather: *weather },
    })
}

const fn participant(side: Side, viewer: Side) -> Participant {
    match (side, viewer) {
        (Side::One, Side::One) | (Side::Two, Side::Two) => Participant::Own,
        (Side::One, Side::Two) | (Side::Two, Side::One) => Participant::Opponent,
    }
}

const fn observe_outcome(outcome: DomainBattleOutcome, viewer: Side) -> ObservedBattleOutcome {
    match outcome {
        DomainBattleOutcome::Winner(side) => {
            ObservedBattleOutcome::Winner(participant(side, viewer))
        }
        DomainBattleOutcome::Escaped(side) => {
            ObservedBattleOutcome::Escaped(participant(side, viewer))
        }
        DomainBattleOutcome::Draw => ObservedBattleOutcome::Draw,
    }
}
