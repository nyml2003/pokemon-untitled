use std::cmp::Ordering;

use crate::{
    Ability, Accuracy, BattleStat, DamageCategory, EffectTarget, MajorStatus, MajorStatusKind,
    Move, MoveCategory, MoveEffect, MoveId, MoveSlot, Pokemon, PokemonId, Side, Team, TeamSlot,
    TypeEffectiveness, Weather, WeatherState,
    rules::{
        calculate_damage, low_hp_type_boost_applies, thick_fat_applies, weather_adjusted_accuracy,
        weather_adjusted_move,
    },
    type_effectiveness,
};

/// 一方在当前对战阶段可以提交的动作。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    UseMove(MoveSlot),
    Switch(TeamSlot),
    Run,
    Struggle,
}

/// 一方提交给对战状态机的动作。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BattleCommand {
    side: Side,
    action: Action,
}

impl BattleCommand {
    /// 创建指定阵营的一条命令。
    pub const fn new(side: Side, action: Action) -> Self {
        Self { side, action }
    }
}

/// 已结束对战的结果。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BattleOutcome {
    Winner(Side),
    Escaped(Side),
    Draw,
}

/// 对战状态机当前接受的命令类型。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BattlePhase {
    Turn,
    ForcedReplacement(ReplacementSides),
    Finished(BattleOutcome),
}

impl BattlePhase {
    /// 返回此阶段是否要求指定阵营先替换倒下的出战宝可梦。
    pub const fn requires_replacement(self, side: Side) -> bool {
        match self {
            Self::ForcedReplacement(sides) => sides.contains(side),
            Self::Turn | Self::Finished(_) => false,
        }
    }
}

/// 需要在强制替换阶段提交换人命令的阵营集合。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplacementSides {
    One,
    Two,
    Both,
}

impl ReplacementSides {
    /// 返回指定阵营是否在此集合中。
    pub const fn contains(self, side: Side) -> bool {
        match self {
            Self::One => matches!(side, Side::One),
            Self::Two => matches!(side, Side::Two),
            Self::Both => true,
        }
    }
}

/// 事件中记录的实际出招。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UsedMove {
    Move { slot: MoveSlot, id: MoveId },
    Struggle,
}

/// 伤害事件的直接来源。
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
    Ability {
        side: Side,
        pokemon: PokemonId,
        ability: Ability,
    },
    Status {
        side: Side,
        pokemon: PokemonId,
        status: MajorStatus,
    },
    Weather {
        weather: Weather,
    },
}

/// 对战结算产生的有序事实记录。
///
/// 事件只描述已经发生的状态变化；调用方不应依靠事件重放来绕过 `Battle` 的合法性校验。
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
    StatusApplied {
        side: Side,
        pokemon: PokemonId,
        status: MajorStatus,
    },
    StatusFailed {
        side: Side,
        target_side: Side,
        target: PokemonId,
        status: MajorStatusKind,
    },
    StatusPreventsAction {
        side: Side,
        pokemon: PokemonId,
        status: MajorStatus,
    },
    StatusCured {
        side: Side,
        pokemon: PokemonId,
        status: MajorStatusKind,
    },
    StatusAdvanced {
        side: Side,
        pokemon: PokemonId,
        status: MajorStatus,
    },
    ProtectionActivated {
        side: Side,
        pokemon: PokemonId,
    },
    ProtectionFailed {
        side: Side,
        pokemon: PokemonId,
    },
    MoveBlocked {
        side: Side,
        target_side: Side,
        target: PokemonId,
    },
    SubstituteCreated {
        side: Side,
        pokemon: PokemonId,
        substitute_hp: u32,
        current_hp: u32,
    },
    SubstituteBlocked {
        side: Side,
        target_side: Side,
        target: PokemonId,
    },
    SubstituteDamaged {
        side: Side,
        pokemon: PokemonId,
        amount: u32,
        remaining_hp: u32,
    },
    SubstituteBroke {
        side: Side,
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
        side: Side,
        pokemon: PokemonId,
        ability: Ability,
    },
    Flinched {
        side: Side,
        pokemon: PokemonId,
    },
    StatStageChanged {
        side: Side,
        pokemon: PokemonId,
        stat: BattleStat,
        change: i8,
        stage: i8,
    },
    Healed {
        side: Side,
        pokemon: PokemonId,
        amount: u32,
        current_hp: u32,
    },
    EffectFailed {
        side: Side,
        target_side: Side,
        target: PokemonId,
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

/// 动作在当前状态下不合法的具体原因。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IllegalActionReason {
    WrongPhase,
    MoveDoesNotExist,
    MoveHasNoPp,
    StruggleNotRequired,
    SwitchToActive,
    SwitchTargetFainted,
    SwitchPrevented,
}

/// 构造或推进对战时违反的领域规则。
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

/// 单次成功提交后产生的增量结果。
///
/// `events` 只含本次提交新增的事件，而不是整个对战的事件历史。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubmitOutcome {
    events: Vec<BattleEvent>,
    phase: BattlePhase,
    waiting_for_opponent: bool,
}

impl SubmitOutcome {
    /// 返回本次提交新增的事件。
    pub fn events(&self) -> &[BattleEvent] {
        &self.events
    }

    /// 返回命令处理后的对战阶段。
    pub const fn phase(&self) -> BattlePhase {
        self.phase
    }

    /// 返回对战是否仍在等待另一方提交命令。
    pub fn is_waiting_for_opponent(&self) -> bool {
        self.waiting_for_opponent
    }
}

/// 维护回合、队伍和事件历史的确定性双人对战状态机。
///
/// 通过 [`Battle::legal_actions`] 查询可提交动作，再使用 [`Battle::submit`] 推进状态。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Battle {
    teams: [Team; 2],
    active: [TeamSlot; 2],
    phase: BattlePhase,
    pending: [Option<PendingCommand>; 2],
    rng: DeterministicRng,
    turn: u32,
    events: Vec<BattleEvent>,
    flinched: [bool; 2],
    protected: [bool; 2],
    flash_fire: [bool; 2],
    weather: Option<WeatherState>,
}

impl Battle {
    /// 用两支各含存活成员的队伍和确定性种子创建对战。
    ///
    /// 两队不得包含相同的 `PokemonId`。
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
        let mut battle = Self {
            teams: [team_one, team_two],
            active: [active_one, active_two],
            phase: BattlePhase::Turn,
            pending: [None, None],
            rng: DeterministicRng::new(seed),
            turn: 1,
            events: Vec::new(),
            flinched: [false; 2],
            protected: [false; 2],
            flash_fire: [false; 2],
            weather: None,
        };
        for side in [Side::One, Side::Two] {
            battle.activate_entry_ability(side);
        }
        Ok(battle)
    }

    /// 返回当前需要提交的命令阶段。
    pub const fn phase(&self) -> BattlePhase {
        self.phase
    }

    /// 返回从一开始递增的当前回合编号。
    pub const fn turn_number(&self) -> u32 {
        self.turn
    }

    /// 返回对战开始以来累积的全部事件。
    pub fn events(&self) -> &[BattleEvent] {
        &self.events
    }

    /// 返回当前天气及其剩余回合数。
    ///
    /// 天气仍会保留在此处，即使 Air Lock 或 Cloud Nine 暂时压制其战斗效果。
    pub const fn weather(&self) -> Option<WeatherState> {
        self.weather
    }

    fn effective_weather(&self) -> Option<Weather> {
        let weather = self.weather.map(WeatherState::weather)?;
        let suppressed = [Side::One, Side::Two].into_iter().any(|side| {
            !self.active(side).is_fainted()
                && matches!(
                    self.active(side).ability(),
                    Some(Ability::AirLock | Ability::CloudNine)
                )
        });
        (!suppressed).then_some(weather)
    }

    /// 返回指定阵营的完整队伍。
    pub fn team(&self, side: Side) -> &Team {
        &self.teams[side_index(side)]
    }

    /// 返回指定阵营当前出战成员的队伍槽位。
    pub fn active_slot(&self, side: Side) -> TeamSlot {
        self.active[side_index(side)]
    }

    /// 返回指定阵营当前出战的宝可梦。
    pub fn active(&self, side: Side) -> &Pokemon {
        self.team(side).member(self.active_slot(side))
    }

    /// 返回指定阵营此刻可提交的全部动作。
    ///
    /// 已提交命令的一方、非当前操作阶段的一方和已结束的对战都会返回空列表。
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
                if !self.is_trapped(side) {
                    actions.extend(self.legal_switches(side));
                    actions.push(Action::Run);
                }
                actions
            }
        }
    }

    /// 原子地提交一条已校验的命令并在双方命令齐备时结算。
    ///
    /// 如果命令不合法，状态和事件历史都保持不变。
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
                } else if self.team(side).member(slot).is_fainted() {
                    IllegalActionReason::SwitchTargetFainted
                } else if self.is_trapped(side) {
                    IllegalActionReason::SwitchPrevented
                } else {
                    unreachable!("a legal switch was rejected")
                }
            }
            Action::Run if self.is_trapped(side) => IllegalActionReason::SwitchPrevented,
            Action::UseMove(_) | Action::Struggle | Action::Run => IllegalActionReason::WrongPhase,
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
        self.flinched = [false; 2];
        self.protected = [false; 2];
        let order = self.action_order(one, two);
        self.resolve_action(order[0]);
        if !matches!(self.phase, BattlePhase::Finished(_)) {
            self.resolve_action(order[1]);
        }
        self.turn = self.turn.saturating_add(1);
        if !matches!(self.phase, BattlePhase::Finished(_)) {
            self.resolve_end_of_turn();
        }
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
                    .effective_speed_in_weather(self.effective_weather())
                    .cmp(
                        &self
                            .active(two.side)
                            .effective_speed_in_weather(self.effective_weather()),
                    )
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
        if !self.can_act(command.side) {
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
        let leaving = self.active(side).clone();
        self.flash_fire[side_index(side)] = false;
        self.teams[side_index(side)]
            .member_mut(from)
            .reset_switch_modifiers();
        if leaving.ability() == Some(Ability::NaturalCure)
            && let Some(status) = self.teams[side_index(side)]
                .member_mut(from)
                .cure_major_status()
        {
            self.events.push(BattleEvent::StatusCured {
                side,
                pokemon: leaving.id().clone(),
                status,
            });
        }
        self.active[side_index(side)] = to;
        let pokemon = self.active(side);
        self.events.push(BattleEvent::Switched {
            side,
            from,
            to,
            pokemon: pokemon.id().clone(),
            current_hp: pokemon.current_hp(),
        });
        self.activate_entry_ability(side);
    }

    fn activate_entry_ability(&mut self, side: Side) {
        let pokemon = self.active(side).clone();
        let Some(ability) = pokemon.ability() else {
            return;
        };
        if !matches!(
            ability,
            Ability::AirLock
                | Ability::CloudNine
                | Ability::Intimidate
                | Ability::Drizzle
                | Ability::Drought
                | Ability::SandStream
        ) {
            return;
        }
        self.events.push(BattleEvent::AbilityActivated {
            side,
            pokemon: pokemon.id().clone(),
            ability,
        });
        match ability {
            Ability::AirLock | Ability::CloudNine => {}
            Ability::Intimidate => {
                let target_side = side.opponent();
                let target_slot = self.active_slot(target_side);
                let target = self.active(target_side).id().clone();
                if let Some(ability) = self
                    .active(target_side)
                    .ability_blocks_opponent_stat_drop(BattleStat::Attack)
                {
                    self.events.push(BattleEvent::AbilityActivated {
                        side: target_side,
                        pokemon: target,
                        ability,
                    });
                    return;
                }
                let previous = self.active(target_side).stages().get(BattleStat::Attack);
                if let Some(stage) = self.teams[side_index(target_side)]
                    .member_mut(target_slot)
                    .change_stage(BattleStat::Attack, -1)
                {
                    self.events.push(BattleEvent::StatStageChanged {
                        side: target_side,
                        pokemon: target,
                        stat: BattleStat::Attack,
                        change: stage - previous,
                        stage,
                    });
                }
            }
            Ability::Drizzle => self.start_weather(Weather::Rain, None),
            Ability::Drought => self.start_weather(Weather::Sun, None),
            Ability::SandStream => self.start_weather(Weather::Sandstorm, None),
            _ => unreachable!("entry ability was checked"),
        }
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
        let target_side = side.opponent();
        let pp_cost = if battle_move.effect().targets_opponent()
            && self.active(target_side).ability() == Some(Ability::Pressure)
        {
            self.events.push(BattleEvent::AbilityActivated {
                side: target_side,
                pokemon: self.active(target_side).id().clone(),
                ability: Ability::Pressure,
            });
            2
        } else {
            1
        };
        let remaining = {
            let battle_move = self.teams[side_index(side)]
                .member_mut(attacker_slot)
                .move_mut(slot)
                .expect("validated move slot");
            for _ in 0..pp_cost {
                battle_move.spend_pp();
            }
            battle_move.current_pp()
        };
        self.events.push(BattleEvent::PpSpent {
            side,
            pokemon: attacker.id().clone(),
            move_slot: slot,
            remaining,
        });
        if !matches!(battle_move.effect(), MoveEffect::ProtectUser) {
            self.teams[side_index(side)]
                .member_mut(attacker_slot)
                .reset_protect_streak();
        }
        let (power, move_type, category) = weather_adjusted_move(
            battle_move.weather_move(),
            battle_move.power(),
            battle_move.move_type(),
            battle_move.category(),
            self.effective_weather(),
        );
        if category == MoveCategory::Status {
            self.resolve_status_move(
                side,
                &attacker,
                move_type,
                self.accuracy_for_move(&battle_move, &attacker, category),
                battle_move.effect(),
            );
        } else {
            let accuracy = self.accuracy_for_move(&battle_move, &attacker, category);
            self.resolve_hit(
                side,
                attacker,
                used_move,
                power,
                Some(move_type),
                damage_category(category),
                accuracy,
                false,
                battle_move.effect(),
            );
        }
    }

    fn use_struggle(&mut self, side: Side) {
        let attacker = self.active(side).clone();
        self.teams[side_index(side)]
            .member_mut(self.active_slot(side))
            .reset_protect_streak();
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
            MoveEffect::None,
        );
    }

    fn resolve_status_move(
        &mut self,
        side: Side,
        attacker: &Pokemon,
        move_type: crate::PokemonType,
        accuracy: Accuracy,
        effect: MoveEffect,
    ) {
        let target_side = side.opponent();
        let target = self.active(target_side).clone();
        self.activate_accuracy_ability(side, attacker, MoveCategory::Status, accuracy);
        let hit = self.check_accuracy(accuracy, attacker, &target);
        if !hit {
            self.events.push(BattleEvent::Missed {
                side,
                target_side,
                target: target.id().clone(),
            });
            return;
        }
        if let Some(ability) = target.ability_blocks_move(move_type) {
            self.activate_move_blocking_ability(target_side, &target, ability);
            self.events.push(BattleEvent::Effectiveness {
                side,
                target_side,
                target: target.id().clone(),
                effectiveness: TypeEffectiveness::Immune,
            });
            return;
        }
        if effect.targets_opponent() && self.protected[side_index(target_side)] {
            self.events.push(BattleEvent::MoveBlocked {
                side,
                target_side,
                target: target.id().clone(),
            });
            return;
        }
        if effect.targets_opponent() && target.substitute_hp().is_some() {
            self.events.push(BattleEvent::SubstituteBlocked {
                side,
                target_side,
                target: target.id().clone(),
            });
            return;
        }
        self.apply_move_effect(side, target_side, effect, false);
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
        effect: MoveEffect,
    ) {
        let target_side = side.opponent();
        let target_slot = self.active_slot(target_side);
        let target = self.active(target_side).clone();
        let move_category = match category {
            DamageCategory::Physical => MoveCategory::Physical,
            DamageCategory::Special => MoveCategory::Special,
        };
        self.activate_accuracy_ability(side, &attacker, move_category, accuracy);
        let hit = self.check_accuracy(accuracy, &attacker, &target);
        if !hit {
            self.events.push(BattleEvent::Missed {
                side,
                target_side,
                target: target.id().clone(),
            });
            return;
        }
        if self.protected[side_index(target_side)] {
            self.events.push(BattleEvent::MoveBlocked {
                side,
                target_side,
                target: target.id().clone(),
            });
            return;
        }
        if let Some(ability) = move_type.and_then(|kind| target.ability_blocks_move(kind)) {
            self.activate_move_blocking_ability(target_side, &target, ability);
            self.events.push(BattleEvent::Effectiveness {
                side,
                target_side,
                target: target.id().clone(),
                effectiveness: TypeEffectiveness::Immune,
            });
            if matches!(ability, Ability::WaterAbsorb | Ability::VoltAbsorb) {
                let amount = (u64::from(target.max_hp()) / 4).max(1);
                let actual = self.teams[side_index(target_side)]
                    .member_mut(target_slot)
                    .heal(amount);
                if actual > 0 {
                    self.events.push(BattleEvent::Healed {
                        side: target_side,
                        pokemon: target.id().clone(),
                        amount: actual,
                        current_hp: self.active(target_side).current_hp(),
                    });
                }
            }
            return;
        }
        let fixed_damage = effect.fixed_damage_for(attacker.level());
        let critical_roll = fixed_damage.is_none() && self.rng.range_inclusive(1, 16) == 1;
        let critical = if critical_roll
            && matches!(
                target.ability(),
                Some(Ability::BattleArmor | Ability::ShellArmor)
            ) {
            let ability = target.ability().expect("matched ability is present");
            self.events.push(BattleEvent::AbilityActivated {
                side: target_side,
                pokemon: target.id().clone(),
                ability,
            });
            false
        } else {
            critical_roll
        };
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
        let flash_fire_boosted = fixed_damage.is_none()
            && move_type == Some(crate::PokemonType::Fire)
            && self.flash_fire[side_index(side)];
        if flash_fire_boosted {
            self.events.push(BattleEvent::AbilityActivated {
                side,
                pokemon: attacker.id().clone(),
                ability: Ability::FlashFire,
            });
        }
        if fixed_damage.is_none()
            && effectiveness != TypeEffectiveness::Immune
            && category == DamageCategory::Physical
            && attacker.physical_attack_ability_is_active()
        {
            self.events.push(BattleEvent::AbilityActivated {
                side,
                pokemon: attacker.id().clone(),
                ability: attacker
                    .ability()
                    .expect("physical attack ability was checked"),
            });
        }
        if fixed_damage.is_none()
            && effectiveness != TypeEffectiveness::Immune
            && category == DamageCategory::Physical
            && target.defense_ability_is_active()
        {
            self.events.push(BattleEvent::AbilityActivated {
                side: target_side,
                pokemon: target.id().clone(),
                ability: Ability::MarvelScale,
            });
        }
        if fixed_damage.is_none()
            && effectiveness != TypeEffectiveness::Immune
            && low_hp_type_boost_applies(&attacker, move_type)
        {
            self.events.push(BattleEvent::AbilityActivated {
                side,
                pokemon: attacker.id().clone(),
                ability: attacker.ability().expect("low HP type boost was checked"),
            });
        }
        if fixed_damage.is_none()
            && effectiveness != TypeEffectiveness::Immune
            && thick_fat_applies(&target, move_type)
        {
            self.events.push(BattleEvent::AbilityActivated {
                side: target_side,
                pokemon: target.id().clone(),
                ability: Ability::ThickFat,
            });
        }
        let damage = match fixed_damage {
            Some(_) if effectiveness == TypeEffectiveness::Immune => 0,
            Some(damage) => damage,
            None => calculate_damage(
                &attacker,
                &target,
                if flash_fire_boosted {
                    power.saturating_mul(3) / 2
                } else {
                    power
                },
                move_type,
                category,
                critical,
                self.rng.range_inclusive(85, 100) as u8,
                self.effective_weather(),
            ),
        };
        if let Some((actual, remaining_hp, broke)) = self.teams[side_index(target_side)]
            .member_mut(target_slot)
            .damage_substitute(damage)
        {
            self.events.push(BattleEvent::SubstituteDamaged {
                side: target_side,
                pokemon: target.id().clone(),
                amount: actual,
                remaining_hp,
            });
            if broke {
                self.events.push(BattleEvent::SubstituteBroke {
                    side: target_side,
                    pokemon: target.id().clone(),
                });
            }
            if recoil {
                self.apply_struggle_recoil(side, &attacker, actual);
            } else {
                self.apply_damaging_move_effect(side, target_side, &attacker, effect, actual, true);
            }
            return;
        }
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
        if actual > 0 {
            self.apply_damaging_move_effect(side, target_side, &attacker, effect, actual, false);
            if !self.active(target_side).is_fainted() {
                self.apply_move_effect(side, target_side, effect, true);
            }
        }
        if recoil {
            self.apply_struggle_recoil(side, &attacker, actual);
        }
    }

    fn can_act(&mut self, side: Side) -> bool {
        let slot = self.active_slot(side);
        let pokemon = self.active(side).clone();
        if self.flinched[side_index(side)] {
            self.events.push(BattleEvent::Flinched {
                side,
                pokemon: pokemon.id().clone(),
            });
            return false;
        }
        match pokemon.major_status() {
            Some(MajorStatus::Sleep { .. }) => {
                let mut remaining = self.teams[side_index(side)]
                    .member_mut(slot)
                    .advance_sleep()
                    .expect("active pokemon was asleep");
                if pokemon.ability() == Some(Ability::EarlyBird) && remaining > 0 {
                    remaining = self.teams[side_index(side)]
                        .member_mut(slot)
                        .advance_sleep()
                        .unwrap_or(0);
                }
                if remaining == 0 {
                    self.events.push(BattleEvent::StatusCured {
                        side,
                        pokemon: pokemon.id().clone(),
                        status: MajorStatusKind::Sleep,
                    });
                    true
                } else {
                    self.events.push(BattleEvent::StatusPreventsAction {
                        side,
                        pokemon: pokemon.id().clone(),
                        status: MajorStatus::Sleep {
                            turns_remaining: remaining,
                        },
                    });
                    false
                }
            }
            Some(MajorStatus::Freeze) => {
                self.events.push(BattleEvent::StatusPreventsAction {
                    side,
                    pokemon: pokemon.id().clone(),
                    status: MajorStatus::Freeze,
                });
                false
            }
            Some(MajorStatus::Paralysis) if self.rng.range_inclusive(1, 4) == 1 => {
                self.events.push(BattleEvent::StatusPreventsAction {
                    side,
                    pokemon: pokemon.id().clone(),
                    status: MajorStatus::Paralysis,
                });
                false
            }
            Some(
                MajorStatus::BadlyPoisoned { .. }
                | MajorStatus::Burn
                | MajorStatus::Poison
                | MajorStatus::Paralysis,
            )
            | None => true,
        }
    }

    fn apply_move_effect(
        &mut self,
        side: Side,
        target_side: Side,
        effect: MoveEffect,
        damaging_secondary: bool,
    ) {
        if damaging_secondary
            && effect.is_non_damaging_secondary_effect()
            && effect.targets_opponent()
            && let Some(ability) = self.active(target_side).ability_blocks_secondary_effect()
        {
            self.events.push(BattleEvent::AbilityActivated {
                side: target_side,
                pokemon: self.active(target_side).id().clone(),
                ability,
            });
            return;
        }
        match effect {
            MoveEffect::None => {}
            MoveEffect::InflictMajorStatus { status, chance } => {
                let chance = self.secondary_effect_chance(side, chance, damaging_secondary);
                if self.rng.range_inclusive(1, 100) > u64::from(chance) {
                    return;
                }
                let applied = match status {
                    MajorStatusKind::Burn => MajorStatus::Burn,
                    MajorStatusKind::Freeze => MajorStatus::Freeze,
                    MajorStatusKind::Paralysis => MajorStatus::Paralysis,
                    MajorStatusKind::Poison => MajorStatus::Poison,
                    MajorStatusKind::BadlyPoisoned => MajorStatus::BadlyPoisoned { stage: 1 },
                    MajorStatusKind::Sleep => MajorStatus::Sleep {
                        turns_remaining: self.rng.range_inclusive(1, 3) as u8,
                    },
                };
                let target = self.active(target_side).id().clone();
                let ability = self
                    .active(target_side)
                    .major_status()
                    .is_none()
                    .then(|| self.active(target_side).ability_blocks_status(status))
                    .flatten();
                if self.teams[side_index(target_side)]
                    .member_mut(self.active_slot(target_side))
                    .inflict_major_status(applied)
                {
                    self.events.push(BattleEvent::StatusApplied {
                        side: target_side,
                        pokemon: target.clone(),
                        status: applied,
                    });
                    self.apply_synchronize(side, target_side, target, status);
                } else {
                    if let Some(ability) = ability {
                        self.events.push(BattleEvent::AbilityActivated {
                            side: target_side,
                            pokemon: target.clone(),
                            ability,
                        });
                    }
                    self.events.push(BattleEvent::StatusFailed {
                        side,
                        target_side,
                        target,
                        status,
                    });
                }
            }
            MoveEffect::ChangeStages { target, changes } => {
                self.apply_stage_changes(side, target_side, target, changes);
            }
            MoveEffect::ChangeStagesWithChance {
                target,
                changes,
                chance,
            } => {
                let chance = self.secondary_effect_chance(side, chance, damaging_secondary);
                if self.rng.range_inclusive(1, 100) <= u64::from(chance) {
                    self.apply_stage_changes(side, target_side, target, changes);
                }
            }
            MoveEffect::HealUser {
                numerator,
                denominator,
            } => {
                let slot = self.active_slot(side);
                let pokemon = self.active(side).id().clone();
                let amount = (u64::from(self.active(side).max_hp()) * u64::from(numerator)
                    / u64::from(denominator))
                .max(1);
                let actual = self.teams[side_index(side)].member_mut(slot).heal(amount);
                if actual == 0 {
                    self.events.push(BattleEvent::EffectFailed {
                        side,
                        target_side: side,
                        target: pokemon,
                    });
                } else {
                    self.events.push(BattleEvent::Healed {
                        side,
                        pokemon,
                        amount: actual,
                        current_hp: self.active(side).current_hp(),
                    });
                }
            }
            MoveEffect::DrainUser { .. }
            | MoveEffect::FixedDamage(_)
            | MoveEffect::FlinchTarget { .. }
            | MoveEffect::RecoilUser { .. } => {}
            MoveEffect::CopyTargetStages => {
                self.copy_target_stages(side, target_side);
            }
            MoveEffect::Haze => {
                for affected_side in [side, target_side] {
                    let slot = self.active_slot(affected_side);
                    let pokemon = self.active(affected_side).id().clone();
                    for stat in BattleStat::ALL {
                        let previous = self.active(affected_side).stages().get(stat);
                        if previous == 0 {
                            continue;
                        }
                        self.teams[side_index(affected_side)]
                            .member_mut(slot)
                            .change_stage(stat, -previous)
                            .expect("non-neutral stage must change");
                        self.events.push(BattleEvent::StatStageChanged {
                            side: affected_side,
                            pokemon: pokemon.clone(),
                            stat,
                            change: -previous,
                            stage: 0,
                        });
                    }
                }
            }
            MoveEffect::Rest => {
                let slot = self.active_slot(side);
                let pokemon = self.active(side).id().clone();
                let Some((healed, previous_status)) =
                    self.teams[side_index(side)].member_mut(slot).rest()
                else {
                    self.events.push(BattleEvent::EffectFailed {
                        side,
                        target_side: side,
                        target: pokemon,
                    });
                    return;
                };
                if let Some(status) = previous_status {
                    self.events.push(BattleEvent::StatusCured {
                        side,
                        pokemon: pokemon.clone(),
                        status: status.kind(),
                    });
                }
                if healed > 0 {
                    self.events.push(BattleEvent::Healed {
                        side,
                        pokemon: pokemon.clone(),
                        amount: healed,
                        current_hp: self.active(side).current_hp(),
                    });
                }
                self.events.push(BattleEvent::StatusApplied {
                    side,
                    pokemon,
                    status: MajorStatus::Sleep { turns_remaining: 3 },
                });
            }
            MoveEffect::Refresh => {
                let slot = self.active_slot(side);
                let pokemon = self.active(side).id().clone();
                match self.teams[side_index(side)].member_mut(slot).refresh() {
                    Some(status) => self.events.push(BattleEvent::StatusCured {
                        side,
                        pokemon,
                        status,
                    }),
                    None => self.events.push(BattleEvent::EffectFailed {
                        side,
                        target_side: side,
                        target: pokemon,
                    }),
                }
            }
            MoveEffect::CreateSubstitute => {
                let slot = self.active_slot(side);
                let pokemon = self.active(side).id().clone();
                if let Some(substitute_hp) = self.teams[side_index(side)]
                    .member_mut(slot)
                    .create_substitute()
                {
                    self.events.push(BattleEvent::SubstituteCreated {
                        side,
                        pokemon,
                        substitute_hp,
                        current_hp: self.active(side).current_hp(),
                    });
                } else {
                    self.events.push(BattleEvent::EffectFailed {
                        side,
                        target_side: side,
                        target: pokemon,
                    });
                }
            }
            MoveEffect::ProtectUser => {
                let slot = self.active_slot(side);
                let pokemon = self.active(side).id().clone();
                let streak = self.active(side).protect_streak();
                let denominator = 1_u64 << u32::from(streak.min(6));
                let succeeds = denominator == 1 || self.rng.range_inclusive(1, denominator) == 1;
                if succeeds {
                    self.teams[side_index(side)]
                        .member_mut(slot)
                        .record_protect_success();
                    self.protected[side_index(side)] = true;
                    self.events
                        .push(BattleEvent::ProtectionActivated { side, pokemon });
                } else {
                    self.teams[side_index(side)]
                        .member_mut(slot)
                        .reset_protect_streak();
                    self.events
                        .push(BattleEvent::ProtectionFailed { side, pokemon });
                }
            }
            MoveEffect::StartWeather(weather) => {
                const WEATHER_DURATION: u8 = 5;
                self.start_weather(weather, Some(WEATHER_DURATION));
            }
        }
    }

    fn apply_synchronize(
        &mut self,
        source_side: Side,
        target_side: Side,
        target: PokemonId,
        status: MajorStatusKind,
    ) {
        let reflected = match status {
            MajorStatusKind::Burn => MajorStatus::Burn,
            MajorStatusKind::Paralysis => MajorStatus::Paralysis,
            MajorStatusKind::Poison => MajorStatus::Poison,
            MajorStatusKind::BadlyPoisoned | MajorStatusKind::Freeze | MajorStatusKind::Sleep => {
                return;
            }
        };
        if self.active(target_side).ability() != Some(Ability::Synchronize)
            || self.active(source_side).major_status().is_some()
            || !self.teams[side_index(source_side)]
                .member_mut(self.active_slot(source_side))
                .inflict_major_status(reflected)
        {
            return;
        }
        let source = self.active(source_side).id().clone();
        self.events.push(BattleEvent::AbilityActivated {
            side: target_side,
            pokemon: target,
            ability: Ability::Synchronize,
        });
        self.events.push(BattleEvent::StatusApplied {
            side: source_side,
            pokemon: source,
            status: reflected,
        });
    }

    fn apply_stage_changes(
        &mut self,
        side: Side,
        target_side: Side,
        target: EffectTarget,
        changes: crate::StageChanges,
    ) {
        let affected_side = match target {
            EffectTarget::User => side,
            EffectTarget::Opponent => target_side,
        };
        let pokemon = self.active(affected_side).id().clone();
        let mut changed = false;
        for stat in BattleStat::ALL {
            let amount = changes.get(stat);
            if amount == 0 {
                continue;
            }
            let blocked_by = (target == EffectTarget::Opponent && amount < 0)
                .then(|| {
                    self.active(affected_side)
                        .ability_blocks_opponent_stat_drop(stat)
                })
                .flatten();
            if let Some(ability) = blocked_by {
                self.events.push(BattleEvent::AbilityActivated {
                    side: affected_side,
                    pokemon: pokemon.clone(),
                    ability,
                });
                continue;
            }
            let previous = self.active(affected_side).stages().get(stat);
            if let Some(stage) = self.teams[side_index(affected_side)]
                .member_mut(self.active_slot(affected_side))
                .change_stage(stat, amount)
            {
                changed = true;
                self.events.push(BattleEvent::StatStageChanged {
                    side: affected_side,
                    pokemon: pokemon.clone(),
                    stat,
                    change: stage - previous,
                    stage,
                });
            }
        }
        if !changed {
            self.events.push(BattleEvent::EffectFailed {
                side,
                target_side: affected_side,
                target: pokemon,
            });
        }
    }

    fn copy_target_stages(&mut self, side: Side, target_side: Side) {
        let slot = self.active_slot(side);
        let pokemon = self.active(side).id().clone();
        let target_stages = self.active(target_side).stages();
        let mut changed = false;
        for stat in BattleStat::ALL {
            let previous = self.active(side).stages().get(stat);
            let target_stage = target_stages.get(stat);
            let change = target_stage - previous;
            if change == 0 {
                continue;
            }
            let stage = self.teams[side_index(side)]
                .member_mut(slot)
                .change_stage(stat, change)
                .expect("difference between valid stages must be valid");
            changed = true;
            self.events.push(BattleEvent::StatStageChanged {
                side,
                pokemon: pokemon.clone(),
                stat,
                change,
                stage,
            });
        }
        if !changed {
            self.events.push(BattleEvent::EffectFailed {
                side,
                target_side: side,
                target: pokemon,
            });
        }
    }

    fn check_accuracy(&mut self, accuracy: Accuracy, attacker: &Pokemon, target: &Pokemon) -> bool {
        let Accuracy::Percent(base) = accuracy else {
            return true;
        };
        let (accuracy_numerator, accuracy_denominator) =
            accuracy_stage_fraction(attacker.stages().get(BattleStat::Accuracy));
        let (evasion_numerator, evasion_denominator) =
            accuracy_stage_fraction(target.stages().get(BattleStat::Evasion));
        let chance =
            (u64::from(base) * u64::from(accuracy_numerator) * u64::from(evasion_denominator)
                / (u64::from(accuracy_denominator) * u64::from(evasion_numerator)))
            .min(100);
        self.rng.range_inclusive(1, 100) <= chance
    }

    fn accuracy_for_move(
        &self,
        battle_move: &Move,
        attacker: &Pokemon,
        category: MoveCategory,
    ) -> Accuracy {
        let weather_accuracy = weather_adjusted_accuracy(
            battle_move.weather_accuracy(),
            battle_move.accuracy(),
            self.effective_weather(),
        );
        match attacker.accuracy_ability(category, weather_accuracy) {
            Some(Ability::CompoundEyes) => match weather_accuracy {
                Accuracy::AlwaysHit => Accuracy::AlwaysHit,
                Accuracy::Percent(value) => {
                    Accuracy::Percent((u16::from(value) * 13 / 10).min(100) as u8)
                }
            },
            Some(Ability::Hustle) => match weather_accuracy {
                Accuracy::AlwaysHit => Accuracy::AlwaysHit,
                Accuracy::Percent(value) => Accuracy::Percent((u16::from(value) * 4 / 5) as u8),
            },
            None => weather_accuracy,
            Some(_) => unreachable!("accuracy ability was checked"),
        }
    }

    fn activate_accuracy_ability(
        &mut self,
        side: Side,
        attacker: &Pokemon,
        category: MoveCategory,
        accuracy: Accuracy,
    ) {
        let Some(ability) = attacker.accuracy_ability(category, accuracy) else {
            return;
        };
        self.events.push(BattleEvent::AbilityActivated {
            side,
            pokemon: attacker.id().clone(),
            ability,
        });
    }

    fn activate_move_blocking_ability(&mut self, side: Side, pokemon: &Pokemon, ability: Ability) {
        self.events.push(BattleEvent::AbilityActivated {
            side,
            pokemon: pokemon.id().clone(),
            ability,
        });
        if ability == Ability::FlashFire {
            self.flash_fire[side_index(side)] = true;
        }
    }

    fn start_weather(&mut self, weather: Weather, turns_remaining: Option<u8>) {
        self.weather = Some(match turns_remaining {
            Some(turns) => WeatherState::with_turns(weather, turns),
            None => WeatherState::permanent(weather),
        });
        self.events.push(BattleEvent::WeatherStarted {
            weather,
            turns_remaining,
        });
    }

    fn resolve_end_of_turn(&mut self) {
        for side in [Side::One, Side::Two] {
            if self.active(side).is_fainted() {
                continue;
            }
            let status = self.active(side).major_status();
            let Some(MajorStatus::Burn | MajorStatus::Poison | MajorStatus::BadlyPoisoned { .. }) =
                status
            else {
                continue;
            };
            let slot = self.active_slot(side);
            let pokemon = self.active(side).clone();
            let damage = match status.expect("status was checked") {
                MajorStatus::BadlyPoisoned { stage } => {
                    u64::from((pokemon.max_hp() / 16).max(1)) * u64::from(stage)
                }
                MajorStatus::Burn | MajorStatus::Poison => u64::from((pokemon.max_hp() / 8).max(1)),
                MajorStatus::Freeze | MajorStatus::Paralysis | MajorStatus::Sleep { .. } => {
                    unreachable!("status was filtered")
                }
            };
            let actual = self.teams[side_index(side)]
                .member_mut(slot)
                .apply_damage(damage);
            self.events.push(BattleEvent::Damage {
                source: DamageSource::Status {
                    side,
                    pokemon: pokemon.id().clone(),
                    status: status.expect("status was checked"),
                },
                target_side: side,
                target: pokemon.id().clone(),
                amount: actual,
                remaining_hp: self.active(side).current_hp(),
            });
            if self.active(side).is_fainted() {
                self.events.push(BattleEvent::Fainted {
                    side,
                    pokemon: pokemon.id().clone(),
                });
            }
            if matches!(status, Some(MajorStatus::BadlyPoisoned { .. })) {
                let stage = self.teams[side_index(side)]
                    .member_mut(slot)
                    .advance_badly_poison()
                    .expect("active pokemon was badly poisoned");
                self.events.push(BattleEvent::StatusAdvanced {
                    side,
                    pokemon: pokemon.id().clone(),
                    status: MajorStatus::BadlyPoisoned { stage },
                });
            }
        }
        self.resolve_weather_end_of_turn();
        self.resolve_speed_boost_end_of_turn();
        self.resolve_shed_skin_end_of_turn();
    }

    fn resolve_speed_boost_end_of_turn(&mut self) {
        for side in [Side::One, Side::Two] {
            if self.active(side).is_fainted()
                || self.active(side).ability() != Some(Ability::SpeedBoost)
            {
                continue;
            }
            let slot = self.active_slot(side);
            let pokemon = self.active(side).id().clone();
            let previous = self.active(side).stages().get(BattleStat::Speed);
            let Some(stage) = self.teams[side_index(side)]
                .member_mut(slot)
                .change_stage(BattleStat::Speed, 1)
            else {
                continue;
            };
            self.events.push(BattleEvent::AbilityActivated {
                side,
                pokemon: pokemon.clone(),
                ability: Ability::SpeedBoost,
            });
            self.events.push(BattleEvent::StatStageChanged {
                side,
                pokemon,
                stat: BattleStat::Speed,
                change: stage - previous,
                stage,
            });
        }
    }

    fn resolve_shed_skin_end_of_turn(&mut self) {
        const SHED_SKIN_CHANCE: u64 = 30;
        for side in [Side::One, Side::Two] {
            if self.active(side).is_fainted()
                || self.active(side).ability() != Some(Ability::ShedSkin)
                || self.active(side).major_status().is_none()
                || self.rng.range_inclusive(1, 100) > SHED_SKIN_CHANCE
            {
                continue;
            }
            let slot = self.active_slot(side);
            let pokemon = self.active(side).id().clone();
            let status = self.teams[side_index(side)]
                .member_mut(slot)
                .cure_major_status()
                .expect("status was checked");
            self.events.push(BattleEvent::AbilityActivated {
                side,
                pokemon: pokemon.clone(),
                ability: Ability::ShedSkin,
            });
            self.events.push(BattleEvent::StatusCured {
                side,
                pokemon,
                status,
            });
        }
    }

    fn resolve_weather_end_of_turn(&mut self) {
        let Some(state) = self.weather else {
            return;
        };
        if self.effective_weather().is_some() {
            for side in [Side::One, Side::Two] {
                if self.active(side).is_fainted()
                    || !weather_damages(state.weather(), self.active(side))
                {
                    continue;
                }
                let slot = self.active_slot(side);
                let pokemon = self.active(side).clone();
                let amount = u64::from((pokemon.max_hp() / 16).max(1));
                let actual = self.teams[side_index(side)]
                    .member_mut(slot)
                    .apply_damage(amount);
                self.events.push(BattleEvent::Damage {
                    source: DamageSource::Weather {
                        weather: state.weather(),
                    },
                    target_side: side,
                    target: pokemon.id().clone(),
                    amount: actual,
                    remaining_hp: self.active(side).current_hp(),
                });
                if self.active(side).is_fainted() {
                    self.events.push(BattleEvent::Fainted {
                        side,
                        pokemon: pokemon.id().clone(),
                    });
                }
            }
            self.resolve_weather_abilities_end_of_turn(state.weather());
        }
        let state = self.weather.as_mut().expect("weather was checked");
        match state.elapse() {
            Some(true) => self.events.push(BattleEvent::WeatherUpdated {
                weather: state.weather(),
                turns_remaining: state
                    .turns_remaining()
                    .expect("temporary weather was checked"),
            }),
            Some(false) => {
                let weather = state.weather();
                self.weather = None;
                self.events.push(BattleEvent::WeatherEnded { weather });
            }
            None => {}
        }
    }

    fn resolve_weather_abilities_end_of_turn(&mut self, weather: Weather) {
        if weather != Weather::Rain {
            return;
        }
        for side in [Side::One, Side::Two] {
            if self.active(side).is_fainted()
                || self.active(side).ability() != Some(Ability::RainDish)
            {
                continue;
            }
            let slot = self.active_slot(side);
            let pokemon = self.active(side).clone();
            let amount = u64::from((pokemon.max_hp() / 16).max(1));
            let actual = self.teams[side_index(side)].member_mut(slot).heal(amount);
            if actual == 0 {
                continue;
            }
            self.events.push(BattleEvent::AbilityActivated {
                side,
                pokemon: pokemon.id().clone(),
                ability: Ability::RainDish,
            });
            self.events.push(BattleEvent::Healed {
                side,
                pokemon: pokemon.id().clone(),
                amount: actual,
                current_hp: self.active(side).current_hp(),
            });
        }
    }

    fn apply_damaging_move_effect(
        &mut self,
        side: Side,
        target_side: Side,
        attacker: &Pokemon,
        effect: MoveEffect,
        dealt: u32,
        hit_substitute: bool,
    ) {
        match effect {
            MoveEffect::DrainUser {
                numerator,
                denominator,
            } if !hit_substitute => {
                let amount =
                    (u64::from(dealt) * u64::from(numerator) / u64::from(denominator)).max(1);
                let target = self.active(target_side).clone();
                if target.ability() == Some(Ability::LiquidOoze) {
                    self.events.push(BattleEvent::AbilityActivated {
                        side: target_side,
                        pokemon: target.id().clone(),
                        ability: Ability::LiquidOoze,
                    });
                    let slot = self.active_slot(side);
                    let actual = self.teams[side_index(side)]
                        .member_mut(slot)
                        .apply_damage(amount);
                    self.events.push(BattleEvent::Damage {
                        source: DamageSource::Ability {
                            side: target_side,
                            pokemon: target.id().clone(),
                            ability: Ability::LiquidOoze,
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
                } else {
                    let slot = self.active_slot(side);
                    let actual = self.teams[side_index(side)].member_mut(slot).heal(amount);
                    if actual > 0 {
                        self.events.push(BattleEvent::Healed {
                            side,
                            pokemon: attacker.id().clone(),
                            amount: actual,
                            current_hp: self.active(side).current_hp(),
                        });
                    }
                }
            }
            MoveEffect::RecoilUser {
                numerator,
                denominator,
            } => {
                if attacker.ability() == Some(Ability::RockHead) {
                    self.events.push(BattleEvent::AbilityActivated {
                        side,
                        pokemon: attacker.id().clone(),
                        ability: Ability::RockHead,
                    });
                } else {
                    self.apply_recoil(side, attacker, dealt, numerator, denominator);
                }
            }
            MoveEffect::FixedDamage(_) => {}
            MoveEffect::FlinchTarget { chance } if !hit_substitute => {
                if let Some(ability) = self.active(target_side).ability_blocks_secondary_effect() {
                    self.events.push(BattleEvent::AbilityActivated {
                        side: target_side,
                        pokemon: self.active(target_side).id().clone(),
                        ability,
                    });
                    return;
                }
                let chance = self.secondary_effect_chance(side, chance, true);
                if self.rng.range_inclusive(1, 100) > u64::from(chance) {
                    return;
                }
                let target = self.active(target_side);
                if target.ability() == Some(Ability::InnerFocus) {
                    self.events.push(BattleEvent::AbilityActivated {
                        side: target_side,
                        pokemon: target.id().clone(),
                        ability: Ability::InnerFocus,
                    });
                } else {
                    self.flinched[side_index(target_side)] = true;
                }
            }
            _ => {}
        }
    }

    fn secondary_effect_chance(&mut self, side: Side, chance: u8, damaging_secondary: bool) -> u8 {
        if !damaging_secondary
            || chance == 100
            || self.active(side).ability() != Some(Ability::SereneGrace)
        {
            return chance;
        }
        self.events.push(BattleEvent::AbilityActivated {
            side,
            pokemon: self.active(side).id().clone(),
            ability: Ability::SereneGrace,
        });
        chance.saturating_mul(2).min(100)
    }

    fn apply_struggle_recoil(&mut self, side: Side, attacker: &Pokemon, dealt: u32) {
        self.apply_recoil(side, attacker, dealt, 1, 4);
    }

    fn apply_recoil(
        &mut self,
        side: Side,
        attacker: &Pokemon,
        dealt: u32,
        numerator: u8,
        denominator: u8,
    ) {
        let slot = self.active_slot(side);
        let recoil = (u64::from(dealt) * u64::from(numerator) / u64::from(denominator)).max(1);
        let actual = self.teams[side_index(side)]
            .member_mut(slot)
            .apply_damage(recoil);
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

    fn is_trapped(&self, side: Side) -> bool {
        let target = self.active(side);
        let opponent = self.active(side.opponent());
        match opponent.ability() {
            Some(Ability::ShadowTag) => target.ability() != Some(Ability::ShadowTag),
            Some(Ability::ArenaTrap) => {
                target.primary_type() != crate::PokemonType::Flying
                    && target.secondary_type() != Some(crate::PokemonType::Flying)
                    && target.ability() != Some(Ability::Levitate)
            }
            _ => false,
        }
    }
}

const fn damage_category(category: MoveCategory) -> DamageCategory {
    match category {
        MoveCategory::Physical => DamageCategory::Physical,
        MoveCategory::Special | MoveCategory::Status => DamageCategory::Special,
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

fn weather_damages(weather: Weather, pokemon: &Pokemon) -> bool {
    match weather {
        Weather::Hail => {
            pokemon.primary_type() != crate::PokemonType::Ice
                && pokemon.secondary_type() != Some(crate::PokemonType::Ice)
        }
        Weather::Sandstorm => {
            pokemon.primary_type() != crate::PokemonType::Rock
                && pokemon.secondary_type() != Some(crate::PokemonType::Rock)
                && pokemon.primary_type() != crate::PokemonType::Ground
                && pokemon.secondary_type() != Some(crate::PokemonType::Ground)
                && pokemon.primary_type() != crate::PokemonType::Steel
                && pokemon.secondary_type() != Some(crate::PokemonType::Steel)
                && pokemon.ability() != Some(Ability::SandVeil)
        }
        Weather::Rain | Weather::Sun => false,
    }
}

fn accuracy_stage_fraction(stage: i8) -> (u8, u8) {
    match stage {
        -6 => (3, 9),
        -5 => (3, 8),
        -4 => (3, 7),
        -3 => (3, 6),
        -2 => (3, 5),
        -1 => (3, 4),
        0 => (3, 3),
        1 => (4, 3),
        2 => (5, 3),
        3 => (6, 3),
        4 => (7, 3),
        5 => (8, 3),
        6 => (9, 3),
        _ => unreachable!("stat stages are clamped to the generation-three range"),
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
