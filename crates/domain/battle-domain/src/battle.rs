use std::cmp::Ordering;

use crate::{
    Accuracy, DamageCategory, MoveId, MoveSlot, Pokemon, PokemonId, Side, Team, TeamSlot,
    TypeEffectiveness, damage_category, rules::calculate_damage, type_effectiveness,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    UseMove(MoveSlot),
    Switch(TeamSlot),
    Run,
    Struggle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BattleCommand {
    side: Side,
    action: Action,
}

impl BattleCommand {
    pub const fn new(side: Side, action: Action) -> Self {
        Self { side, action }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BattleOutcome {
    Winner(Side),
    Escaped(Side),
    Draw,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BattlePhase {
    Turn,
    ForcedReplacement(ReplacementSides),
    Finished(BattleOutcome),
}

impl BattlePhase {
    pub const fn requires_replacement(self, side: Side) -> bool {
        match self {
            Self::ForcedReplacement(sides) => sides.contains(side),
            Self::Turn | Self::Finished(_) => false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplacementSides {
    One,
    Two,
    Both,
}

impl ReplacementSides {
    pub const fn contains(self, side: Side) -> bool {
        match self {
            Self::One => matches!(side, Side::One),
            Self::Two => matches!(side, Side::Two),
            Self::Both => true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UsedMove {
    Move { slot: MoveSlot, id: MoveId },
    Struggle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DamageSource {
    Move {
        side: Side,
        pokemon: PokemonId,
        used_move: UsedMove,
    },
    Recoil {
        side: Side,
        pokemon: PokemonId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BattleEvent {
    CommandAccepted {
        side: Side,
        action: Action,
    },
    TurnStarted {
        turn: u32,
    },
    Switched {
        side: Side,
        from: TeamSlot,
        to: TeamSlot,
        pokemon: PokemonId,
        current_hp: u32,
    },
    MoveUsed {
        side: Side,
        pokemon: PokemonId,
        used_move: UsedMove,
    },
    PpSpent {
        side: Side,
        pokemon: PokemonId,
        move_slot: MoveSlot,
        remaining: u8,
    },
    Missed {
        side: Side,
        target_side: Side,
        target: PokemonId,
    },
    Critical {
        side: Side,
        target_side: Side,
        target: PokemonId,
    },
    Effectiveness {
        side: Side,
        target_side: Side,
        target: PokemonId,
        effectiveness: TypeEffectiveness,
    },
    Damage {
        source: DamageSource,
        target_side: Side,
        target: PokemonId,
        amount: u32,
        remaining_hp: u32,
    },
    Fainted {
        side: Side,
        pokemon: PokemonId,
    },
    ForcedReplacement {
        side: Side,
    },
    BattleFinished {
        outcome: BattleOutcome,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IllegalActionReason {
    WrongPhase,
    MoveDoesNotExist,
    MoveHasNoPp,
    StruggleNotRequired,
    SwitchToActive,
    SwitchTargetFainted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BattleError {
    NoLivingPokemon {
        side: Side,
    },
    DuplicatePokemonId {
        id: PokemonId,
    },
    CommandAlreadySubmitted {
        side: Side,
    },
    ActionNotLegal {
        side: Side,
        action: Action,
        reason: IllegalActionReason,
    },
    BattleAlreadyFinished {
        outcome: BattleOutcome,
    },
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

    pub fn is_waiting_for_opponent(&self) -> bool {
        self.waiting_for_opponent
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Battle {
    teams: [Team; 2],
    active: [TeamSlot; 2],
    phase: BattlePhase,
    pending: [Option<PendingCommand>; 2],
    rng: DeterministicRng,
    turn: u32,
    events: Vec<BattleEvent>,
}

impl Battle {
    pub fn new(team_one: Team, team_two: Team, seed: u64) -> Result<Self, BattleError> {
        let active_one = team_one
            .first_living_slot()
            .ok_or(BattleError::NoLivingPokemon { side: Side::One })?;
        let active_two = team_two
            .first_living_slot()
            .ok_or(BattleError::NoLivingPokemon { side: Side::Two })?;
        for first in team_one.members() {
            if team_two
                .members()
                .iter()
                .any(|second| first.id() == second.id())
            {
                return Err(BattleError::DuplicatePokemonId {
                    id: first.id().clone(),
                });
            }
        }
        Ok(Self {
            teams: [team_one, team_two],
            active: [active_one, active_two],
            phase: BattlePhase::Turn,
            pending: [None, None],
            rng: DeterministicRng::new(seed),
            turn: 1,
            events: Vec::new(),
        })
    }

    pub const fn phase(&self) -> BattlePhase {
        self.phase
    }

    pub const fn turn_number(&self) -> u32 {
        self.turn
    }

    pub fn events(&self) -> &[BattleEvent] {
        &self.events
    }

    pub fn team(&self, side: Side) -> &Team {
        &self.teams[side_index(side)]
    }

    pub fn active_slot(&self, side: Side) -> TeamSlot {
        self.active[side_index(side)]
    }

    pub fn active(&self, side: Side) -> &Pokemon {
        self.team(side).member(self.active_slot(side))
    }

    pub fn legal_actions(&self, side: Side) -> Vec<Action> {
        if self.pending[side_index(side)].is_some() {
            return Vec::new();
        }
        match self.phase {
            BattlePhase::Finished(_) => Vec::new(),
            BattlePhase::ForcedReplacement(_) => {
                if self.phase.requires_replacement(side) {
                    self.legal_switches(side)
                } else {
                    Vec::new()
                }
            }
            BattlePhase::Turn => {
                let active = self.active(side);
                let mut actions = Vec::new();
                for (index, battle_move) in active.moves().iter().enumerate() {
                    if battle_move.current_pp() > 0 {
                        actions.push(Action::UseMove(MoveSlot::from_valid_index(index)));
                    }
                }
                if actions.is_empty() {
                    actions.push(Action::Struggle);
                }
                actions.extend(self.legal_switches(side));
                actions.push(Action::Run);
                actions
            }
        }
    }

    pub fn submit(&mut self, command: BattleCommand) -> Result<SubmitOutcome, BattleError> {
        let mut candidate = self.clone();
        let start = candidate.events.len();
        let waiting_for_opponent = candidate.submit_in_place(command)?;
        let outcome = SubmitOutcome {
            events: candidate.events[start..].to_vec(),
            phase: candidate.phase,
            waiting_for_opponent,
        };
        *self = candidate;
        Ok(outcome)
    }

    fn submit_in_place(&mut self, command: BattleCommand) -> Result<bool, BattleError> {
        if let BattlePhase::Finished(outcome) = self.phase {
            return Err(BattleError::BattleAlreadyFinished { outcome });
        }
        let index = side_index(command.side);
        if self.pending[index].is_some() {
            return Err(BattleError::CommandAlreadySubmitted { side: command.side });
        }
        self.validate_action(command.side, command.action)?;
        self.pending[index] = Some(PendingCommand {
            command,
            replacement: match command.action {
                Action::Switch(slot) => Some(slot),
                Action::UseMove(_) | Action::Run | Action::Struggle => None,
            },
        });
        let commands_ready = self.commands_ready();
        if commands_ready {
            self.publish_pending_commands();
            if matches!(self.phase, BattlePhase::ForcedReplacement(_)) {
                self.resolve_replacements();
            } else {
                self.resolve_turn();
            }
        }
        Ok(!commands_ready)
    }

    fn publish_pending_commands(&mut self) {
        for pending in self.pending.iter().flatten() {
            self.events.push(BattleEvent::CommandAccepted {
                side: pending.command.side,
                action: pending.command.action,
            });
        }
    }

    fn validate_action(&self, side: Side, action: Action) -> Result<(), BattleError> {
        if self.legal_actions(side).contains(&action) {
            return Ok(());
        }
        let reason = if matches!(self.phase, BattlePhase::ForcedReplacement(_)) {
            if !self.phase.requires_replacement(side) {
                IllegalActionReason::WrongPhase
            } else {
                self.switch_error(side, action)
            }
        } else {
            match action {
                Action::UseMove(slot) => {
                    if self.active(side).moves().get(slot.index()).is_some() {
                        IllegalActionReason::MoveHasNoPp
                    } else {
                        IllegalActionReason::MoveDoesNotExist
                    }
                }
                Action::Struggle => IllegalActionReason::StruggleNotRequired,
                Action::Switch(_) | Action::Run => self.switch_error(side, action),
            }
        };
        Err(BattleError::ActionNotLegal {
            side,
            action,
            reason,
        })
    }

    fn switch_error(&self, side: Side, action: Action) -> IllegalActionReason {
        match action {
            Action::Switch(slot) => {
                if slot == self.active_slot(side) {
                    IllegalActionReason::SwitchToActive
                } else {
                    debug_assert!(self.team(side).member(slot).is_fainted());
                    IllegalActionReason::SwitchTargetFainted
                }
            }
            _ => IllegalActionReason::WrongPhase,
        }
    }

    fn commands_ready(&self) -> bool {
        let one_required = self.phase.requires_replacement(Side::One);
        let two_required = self.phase.requires_replacement(Side::Two);
        let one_ready = !one_required || self.pending[0].is_some();
        let two_ready = !two_required || self.pending[1].is_some();
        let replacement_phase = one_required || two_required;
        if replacement_phase {
            one_ready && two_ready
        } else {
            self.pending.iter().all(Option::is_some)
        }
    }

    fn resolve_turn(&mut self) {
        let one = self.pending[0]
            .take()
            .expect("turn requires side one")
            .command;
        let two = self.pending[1]
            .take()
            .expect("turn requires side two")
            .command;
        self.events
            .push(BattleEvent::TurnStarted { turn: self.turn });
        let order = self.action_order(one, two);
        self.resolve_action(order[0]);
        if !matches!(self.phase, BattlePhase::Finished(_)) {
            self.resolve_action(order[1]);
        }
        self.turn = self.turn.saturating_add(1);
        if !matches!(self.phase, BattlePhase::Finished(_)) {
            self.update_phase_after_turn();
        }
    }

    fn action_order(&mut self, one: BattleCommand, two: BattleCommand) -> [BattleCommand; 2] {
        match self.compare_actions(one, two) {
            Ordering::Greater => [one, two],
            Ordering::Less => [two, one],
            Ordering::Equal if self.rng.next_bool() => [one, two],
            Ordering::Equal => [two, one],
        }
    }

    fn compare_actions(&self, one: BattleCommand, two: BattleCommand) -> Ordering {
        action_class(one.action)
            .cmp(&action_class(two.action))
            .then_with(|| self.action_priority(one).cmp(&self.action_priority(two)))
            .then_with(|| {
                self.active(one.side)
                    .stats()
                    .speed()
                    .cmp(&self.active(two.side).stats().speed())
            })
    }

    fn action_priority(&self, command: BattleCommand) -> i8 {
        match command.action {
            Action::UseMove(slot) => self.active(command.side).moves()[slot.index()].priority(),
            Action::Switch(_) | Action::Run | Action::Struggle => 0,
        }
    }

    fn resolve_action(&mut self, command: BattleCommand) {
        if self.active(command.side).is_fainted()
            || self.active(command.side.opponent()).is_fainted()
        {
            return;
        }
        match command.action {
            Action::Switch(to) => self.switch(command.side, to),
            Action::UseMove(slot) => self.use_regular_move(command.side, slot),
            Action::Run => self.run(command.side),
            Action::Struggle => self.use_struggle(command.side),
        }
    }

    fn run(&mut self, side: Side) {
        let outcome = BattleOutcome::Escaped(side);
        self.phase = BattlePhase::Finished(outcome);
        self.events.push(BattleEvent::BattleFinished { outcome });
    }

    fn switch(&mut self, side: Side, to: TeamSlot) {
        let from = self.active_slot(side);
        self.active[side_index(side)] = to;
        let pokemon = self.active(side);
        self.events.push(BattleEvent::Switched {
            side,
            from,
            to,
            pokemon: pokemon.id().clone(),
            current_hp: pokemon.current_hp(),
        });
    }

    fn use_regular_move(&mut self, side: Side, slot: MoveSlot) {
        let attacker_slot = self.active_slot(side);
        let attacker = self.active(side).clone();
        let battle_move = attacker
            .moves()
            .get(slot.index())
            .expect("validated move slot")
            .clone();
        let used_move = UsedMove::Move {
            slot,
            id: battle_move.id().clone(),
        };
        self.events.push(BattleEvent::MoveUsed {
            side,
            pokemon: attacker.id().clone(),
            used_move: used_move.clone(),
        });
        let remaining = {
            let battle_move = self.teams[side_index(side)]
                .member_mut(attacker_slot)
                .move_mut(slot)
                .expect("validated move slot");
            battle_move.spend_pp();
            battle_move.current_pp()
        };
        self.events.push(BattleEvent::PpSpent {
            side,
            pokemon: attacker.id().clone(),
            move_slot: slot,
            remaining,
        });
        self.resolve_hit(
            side,
            attacker,
            used_move,
            battle_move.power(),
            Some(battle_move.move_type()),
            damage_category(battle_move.move_type()),
            battle_move.accuracy(),
            false,
        );
    }

    fn use_struggle(&mut self, side: Side) {
        let attacker = self.active(side).clone();
        self.events.push(BattleEvent::MoveUsed {
            side,
            pokemon: attacker.id().clone(),
            used_move: UsedMove::Struggle,
        });
        self.resolve_hit(
            side,
            attacker,
            UsedMove::Struggle,
            50,
            None,
            DamageCategory::Physical,
            Accuracy::AlwaysHit,
            true,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_hit(
        &mut self,
        side: Side,
        attacker: Pokemon,
        used_move: UsedMove,
        power: u16,
        move_type: Option<crate::PokemonType>,
        category: DamageCategory,
        accuracy: Accuracy,
        recoil: bool,
    ) {
        let target_side = side.opponent();
        let target_slot = self.active_slot(target_side);
        let target = self.active(target_side).clone();
        let hit = match accuracy {
            Accuracy::AlwaysHit => true,
            Accuracy::Percent(chance) => self.rng.range_inclusive(1, 100) <= u64::from(chance),
        };
        if !hit {
            self.events.push(BattleEvent::Missed {
                side,
                target_side,
                target: target.id().clone(),
            });
            return;
        }
        let critical = self.rng.range_inclusive(1, 16) == 1;
        let effectiveness = move_type.map_or(TypeEffectiveness::Normal, |attack_type| {
            type_effectiveness(attack_type, target.primary_type(), target.secondary_type())
        });
        if critical && effectiveness != TypeEffectiveness::Immune {
            self.events.push(BattleEvent::Critical {
                side,
                target_side,
                target: target.id().clone(),
            });
        }
        self.events.push(BattleEvent::Effectiveness {
            side,
            target_side,
            target: target.id().clone(),
            effectiveness,
        });
        let random_percent = self.rng.range_inclusive(85, 100) as u8;
        let damage = calculate_damage(
            &attacker,
            &target,
            power,
            move_type,
            category,
            critical,
            random_percent,
        );
        let actual = self.teams[side_index(target_side)]
            .member_mut(target_slot)
            .apply_damage(damage);
        self.events.push(BattleEvent::Damage {
            source: DamageSource::Move {
                side,
                pokemon: attacker.id().clone(),
                used_move,
            },
            target_side,
            target: target.id().clone(),
            amount: actual,
            remaining_hp: self.active(target_side).current_hp(),
        });
        if self.active(target_side).is_fainted() {
            self.events.push(BattleEvent::Fainted {
                side: target_side,
                pokemon: target.id().clone(),
            });
        }
        if recoil {
            self.apply_struggle_recoil(side, &attacker, actual);
        }
    }

    fn apply_struggle_recoil(&mut self, side: Side, attacker: &Pokemon, dealt: u32) {
        let slot = self.active_slot(side);
        let recoil = (dealt / 4).max(1);
        let actual = self.teams[side_index(side)]
            .member_mut(slot)
            .apply_damage(u64::from(recoil));
        self.events.push(BattleEvent::Damage {
            source: DamageSource::Recoil {
                side,
                pokemon: attacker.id().clone(),
            },
            target_side: side,
            target: attacker.id().clone(),
            amount: actual,
            remaining_hp: self.active(side).current_hp(),
        });
        if self.active(side).is_fainted() {
            self.events.push(BattleEvent::Fainted {
                side,
                pokemon: attacker.id().clone(),
            });
        }
    }

    fn update_phase_after_turn(&mut self) {
        let one_living = self.teams[0].has_living();
        let two_living = self.teams[1].has_living();
        let outcome = match (one_living, two_living) {
            (false, false) => Some(BattleOutcome::Draw),
            (true, false) => Some(BattleOutcome::Winner(Side::One)),
            (false, true) => Some(BattleOutcome::Winner(Side::Two)),
            (true, true) => None,
        };
        if let Some(outcome) = outcome {
            self.phase = BattlePhase::Finished(outcome);
            self.events.push(BattleEvent::BattleFinished { outcome });
            return;
        }
        let side_one = self.active(Side::One).is_fainted();
        let side_two = self.active(Side::Two).is_fainted();
        let replacements = match (side_one, side_two) {
            (true, true) => Some(ReplacementSides::Both),
            (true, false) => Some(ReplacementSides::One),
            (false, true) => Some(ReplacementSides::Two),
            (false, false) => None,
        };
        if let Some(replacements) = replacements {
            self.phase = BattlePhase::ForcedReplacement(replacements);
            if side_one {
                self.events
                    .push(BattleEvent::ForcedReplacement { side: Side::One });
            }
            if side_two {
                self.events
                    .push(BattleEvent::ForcedReplacement { side: Side::Two });
            }
        } else {
            self.phase = BattlePhase::Turn;
        }
    }

    fn resolve_replacements(&mut self) {
        for side in [Side::One, Side::Two] {
            if self.phase.requires_replacement(side) {
                let pending = self.pending[side_index(side)]
                    .take()
                    .expect("required replacement was submitted");
                let to = pending
                    .replacement
                    .expect("validated replacement carries a switch target");
                self.switch(side, to);
            }
        }
        self.pending = [None, None];
        self.phase = BattlePhase::Turn;
    }

    fn legal_switches(&self, side: Side) -> Vec<Action> {
        let active = self.active_slot(side);
        self.team(side)
            .members()
            .iter()
            .enumerate()
            .filter_map(|(index, pokemon)| {
                let slot = TeamSlot::from_valid_index(index);
                (slot != active && !pokemon.is_fainted()).then_some(Action::Switch(slot))
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PendingCommand {
    command: BattleCommand,
    replacement: Option<TeamSlot>,
}

const fn side_index(side: Side) -> usize {
    match side {
        Side::One => 0,
        Side::Two => 1,
    }
}

const fn action_class(action: Action) -> u8 {
    match action {
        Action::Run => 2,
        Action::Switch(_) => 1,
        Action::UseMove(_) | Action::Struggle => 0,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn range_inclusive(&mut self, minimum: u64, maximum: u64) -> u64 {
        debug_assert!(minimum <= maximum);
        minimum + self.next_u64() % (maximum - minimum + 1)
    }

    fn next_bool(&mut self) -> bool {
        self.next_u64() & 1 == 0
    }
}
