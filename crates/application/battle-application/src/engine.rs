use std::cmp::Ordering;

use battle_domain::{
    Ability, Accuracy, Action, Battle, BattleCommand, BattleError, BattleEvent, BattleOutcome,
    BattlePhase, BattleStat, BattleState, BattleUnit, DamageSource, EffectTarget,
    IllegalActionReason, MajorStatus, MajorStatusKind, Move, MoveCategory, MoveEffect, MoveSlot,
    PendingCommand, PokemonType, Side, StageChanges, StatStages, SubmitOutcome, TeamSlot,
    TypeEffectiveness, UsedMove, VolatileStatus, Weather, WeatherAccuracyModifier, WeatherState,
};

use super::rules;

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

/// 查询指定阵营当前可提交的动作。
pub fn legal_actions(battle: &Battle, side: Side) -> Vec<Action> {
    if battle.pending[side_index(side)].is_some() {
        return Vec::new();
    }
    match battle.phase {
        BattlePhase::Finished(_) => Vec::new(),
        BattlePhase::ForcedReplacement(_) => {
            if battle.phase.requires_replacement(side) {
                legal_switches(battle, side)
            } else {
                Vec::new()
            }
        }
        BattlePhase::Turn => {
            let active = battle.active(side);
            let mut actions = Vec::new();
            for (index, battle_move) in active.state.moves.iter().enumerate() {
                if battle_move.current_pp() > 0 {
                    actions.push(Action::UseMove(MoveSlot::from_valid_index(index)));
                }
            }
            if actions.is_empty() {
                actions.push(Action::Struggle);
            }
            if !is_trapped(battle, side) {
                actions.extend(legal_switches(battle, side));
                actions.push(Action::Run);
            }
            actions
        }
    }
}

/// 原子地提交一条命令，双方命令齐备时结算回合。
pub fn submit(battle: &mut Battle, command: BattleCommand) -> Result<SubmitOutcome, BattleError> {
    let mut candidate = battle.clone();
    let start = candidate.events.len();
    let waiting_for_opponent = submit_in_place(&mut candidate, command)?;
    let outcome = SubmitOutcome {
        events: candidate.events[start..].to_vec(),
        phase: candidate.phase,
        waiting_for_opponent,
    };
    *battle = candidate;
    Ok(outcome)
}

fn submit_in_place(battle: &mut Battle, command: BattleCommand) -> Result<bool, BattleError> {
    if let BattlePhase::Finished(outcome) = battle.phase {
        return Err(BattleError::BattleAlreadyFinished { outcome });
    }
    let index = side_index(command.side());
    if battle.pending[index].is_some() {
        return Err(BattleError::CommandAlreadySubmitted {
            side: command.side(),
        });
    }
    validate_action(battle, command.side(), command.action())?;
    battle.pending[index] = Some(PendingCommand {
        command,
        replacement: match command.action() {
            Action::Switch(slot) => Some(slot),
            Action::UseMove(_) | Action::Run | Action::Struggle => None,
        },
    });
    let commands_ready = commands_ready(battle);
    if commands_ready {
        publish_pending_commands(battle);
        if matches!(battle.phase, BattlePhase::ForcedReplacement(_)) {
            resolve_replacements(battle)?;
        } else {
            resolve_turn(battle)?;
        }
    }
    Ok(!commands_ready)
}

fn publish_pending_commands(battle: &mut Battle) {
    for pending in battle.pending.iter().flatten() {
        battle.events.push(BattleEvent::CommandAccepted {
            side: pending.command.side(),
            action: pending.command.action(),
        });
    }
}

fn validate_action(battle: &Battle, side: Side, action: Action) -> Result<(), BattleError> {
    if legal_actions(battle, side).contains(&action) {
        return Ok(());
    }
    let reason = if matches!(battle.phase, BattlePhase::ForcedReplacement(_)) {
        if !battle.phase.requires_replacement(side) {
            IllegalActionReason::WrongPhase
        } else {
            switch_error(battle, side, action)
        }
    } else {
        match action {
            Action::UseMove(slot) => {
                if battle.active(side).state.moves.get(slot.index()).is_some() {
                    IllegalActionReason::MoveHasNoPp
                } else {
                    IllegalActionReason::MoveDoesNotExist
                }
            }
            Action::Struggle => IllegalActionReason::StruggleNotRequired,
            Action::Switch(_) | Action::Run => switch_error(battle, side, action),
        }
    };
    Err(BattleError::ActionNotLegal {
        side,
        action,
        reason,
    })
}

fn switch_error(battle: &Battle, side: Side, action: Action) -> IllegalActionReason {
    match action {
        Action::Switch(slot) => {
            if slot == battle.active_slot(side) {
                IllegalActionReason::SwitchToActive
            } else if battle.team(side).member(slot).is_fainted() {
                IllegalActionReason::SwitchTargetFainted
            } else if is_trapped(battle, side) {
                IllegalActionReason::SwitchPrevented
            } else {
                IllegalActionReason::StateInconsistent
            }
        }
        Action::Run if is_trapped(battle, side) => IllegalActionReason::SwitchPrevented,
        Action::UseMove(_) | Action::Struggle | Action::Run => IllegalActionReason::WrongPhase,
    }
}

fn commands_ready(battle: &Battle) -> bool {
    let one_required = battle.phase.requires_replacement(Side::One);
    let two_required = battle.phase.requires_replacement(Side::Two);
    let one_ready = !one_required || battle.pending[0].is_some();
    let two_ready = !two_required || battle.pending[1].is_some();
    let replacement_phase = one_required || two_required;
    if replacement_phase {
        one_ready && two_ready
    } else {
        battle.pending.iter().all(Option::is_some)
    }
}

fn resolve_turn(battle: &mut Battle) -> Result<(), BattleError> {
    let one = battle.pending[0]
        .take()
        .ok_or(BattleError::StateInconsistent {
            detail: "turn is missing side one command",
        })?
        .command;
    let two = battle.pending[1]
        .take()
        .ok_or(BattleError::StateInconsistent {
            detail: "turn is missing side two command",
        })?
        .command;
    battle
        .events
        .push(BattleEvent::TurnStarted { turn: battle.turn });
    battle.flinched = [false; 2];
    battle.protected = [false; 2];
    let order = action_order(battle, one, two);
    resolve_action(battle, order[0])?;
    if !matches!(battle.phase, BattlePhase::Finished(_)) {
        resolve_action(battle, order[1])?;
    }
    battle.turn = battle.turn.saturating_add(1);
    if !matches!(battle.phase, BattlePhase::Finished(_)) {
        resolve_end_of_turn(battle);
    }
    if !matches!(battle.phase, BattlePhase::Finished(_)) {
        update_phase_after_turn(battle);
    }
    Ok(())
}

fn action_order(battle: &mut Battle, one: BattleCommand, two: BattleCommand) -> [BattleCommand; 2] {
    match compare_actions(battle, one, two) {
        Ordering::Greater => [one, two],
        Ordering::Less => [two, one],
        Ordering::Equal if battle.rng.next_bool() => [one, two],
        Ordering::Equal => [two, one],
    }
}

fn compare_actions(battle: &Battle, one: BattleCommand, two: BattleCommand) -> Ordering {
    action_class(one.action())
        .cmp(&action_class(two.action()))
        .then_with(|| action_priority(battle, one).cmp(&action_priority(battle, two)))
        .then_with(|| effective_speed(battle, one.side()).cmp(&effective_speed(battle, two.side())))
}

fn action_priority(battle: &Battle, command: BattleCommand) -> i8 {
    match command.action() {
        Action::UseMove(slot) => battle.active(command.side()).state.moves[slot.index()].priority(),
        Action::Switch(_) | Action::Run | Action::Struggle => 0,
    }
}

fn resolve_action(battle: &mut Battle, command: BattleCommand) -> Result<(), BattleError> {
    if battle.active(command.side()).is_fainted()
        || battle.active(command.side().opponent()).is_fainted()
    {
        return Ok(());
    }
    if !can_act(battle, command.side()) {
        return Ok(());
    }
    match command.action() {
        Action::Switch(to) => switch(battle, command.side(), to),
        Action::UseMove(slot) => use_regular_move(battle, command.side(), slot)?,
        Action::Run => run(battle, command.side()),
        Action::Struggle => use_struggle(battle, command.side()),
    }
    Ok(())
}

fn run(battle: &mut Battle, side: Side) {
    let outcome = BattleOutcome::Escaped(side);
    battle.phase = BattlePhase::Finished(outcome);
    battle.events.push(BattleEvent::BattleFinished { outcome });
}

fn switch(battle: &mut Battle, side: Side, to: TeamSlot) {
    let from = battle.active_slot(side);
    let leaving = battle.active(side).clone();
    battle.flash_fire[side_index(side)] = false;
    let leaving_state = &mut battle.teams[side_index(side)].members[from.index()].state;
    reset_switch_modifiers(leaving_state);
    if leaving.state.ability.contains(&Ability::NaturalCure)
        && let Some(status) = cure_major_status(leaving_state)
    {
        battle.events.push(BattleEvent::StatusCured {
            side,
            pokemon: leaving.id().clone(),
            status,
        });
    }
    battle.active[side_index(side)] = to;
    let pokemon = battle.active(side);
    battle.events.push(BattleEvent::Switched {
        side,
        from,
        to,
        pokemon: pokemon.id().clone(),
        current_hp: pokemon.state.current_hp,
    });
    activate_entry_ability(battle, side);
}

fn activate_entry_ability(battle: &mut Battle, side: Side) {
    let pokemon = battle.active(side).clone();
    let Some(&ability) = pokemon.state.ability.first() else {
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
    battle.events.push(BattleEvent::AbilityActivated {
        side,
        pokemon: pokemon.id().clone(),
        ability,
    });
    match ability {
        Ability::AirLock | Ability::CloudNine => {}
        Ability::Intimidate => {
            let target_side = side.opponent();
            let target_slot = battle.active_slot(target_side);
            let target = battle.active(target_side).id().clone();
            if let Some(block) =
                ability_blocks_opponent_stat_drop(battle.active(target_side), BattleStat::Attack)
            {
                battle.events.push(BattleEvent::AbilityActivated {
                    side: target_side,
                    pokemon: target,
                    ability: block,
                });
                return;
            }
            let previous = battle
                .active(target_side)
                .state
                .stages
                .get(BattleStat::Attack);
            if let Some(stage) = change_stage(
                &mut battle.teams[side_index(target_side)].members[target_slot.index()].state,
                BattleStat::Attack,
                -1,
            ) {
                battle.events.push(BattleEvent::StatStageChanged {
                    side: target_side,
                    pokemon: target,
                    stat: BattleStat::Attack,
                    change: stage - previous,
                    stage,
                });
            }
        }
        Ability::Drizzle => start_weather(battle, Weather::Rain, None),
        Ability::Drought => start_weather(battle, Weather::Sun, None),
        Ability::SandStream => start_weather(battle, Weather::Sandstorm, None),
        _ => (),
    }
}

fn use_regular_move(battle: &mut Battle, side: Side, slot: MoveSlot) -> Result<(), BattleError> {
    let attacker_slot = battle.active_slot(side);
    let attacker = battle.active(side).clone();
    let battle_move = attacker
        .state
        .moves
        .get(slot.index())
        .ok_or(BattleError::StateInconsistent {
            detail: "submitted move slot is missing from the active pokemon",
        })?
        .clone();
    let used_move = UsedMove::Move {
        slot,
        id: battle_move.id().clone(),
    };
    battle.events.push(BattleEvent::MoveUsed {
        side,
        pokemon: attacker.id().clone(),
        used_move: used_move.clone(),
    });
    let target_side = side.opponent();
    let pp_cost = if first_effect(&battle_move).targets_opponent()
        && battle
            .active(target_side)
            .state
            .ability
            .contains(&Ability::Pressure)
    {
        battle.events.push(BattleEvent::AbilityActivated {
            side: target_side,
            pokemon: battle.active(target_side).id().clone(),
            ability: Ability::Pressure,
        });
        2
    } else {
        1
    };
    let remaining = {
        let battle_move = &mut battle.teams[side_index(side)].members[attacker_slot.index()]
            .state
            .moves[slot.index()];
        for _ in 0..pp_cost {
            battle_move.spend_pp();
        }
        battle_move.current_pp()
    };
    battle.events.push(BattleEvent::PpSpent {
        side,
        pokemon: attacker.id().clone(),
        move_slot: slot,
        remaining,
    });
    if !matches!(first_effect(&battle_move), MoveEffect::ProtectUser) {
        reset_protect_streak(
            &mut battle.teams[side_index(side)].members[attacker_slot.index()].state,
        );
    }
    let (power, move_type, category) = weather_adjusted_move(
        battle_move.weather_move(),
        battle_move.power(),
        battle_move.move_types().first().copied(),
        battle_move.category(),
        effective_weather(battle),
    );
    if category == MoveCategory::Status {
        resolve_status_move(
            battle,
            side,
            &attacker,
            move_type,
            accuracy_for_move(battle, &battle_move, &attacker, category),
            first_effect(&battle_move),
        );
    } else {
        let accuracy = accuracy_for_move(battle, &battle_move, &attacker, category);
        resolve_hit(
            battle,
            side,
            attacker,
            used_move,
            power,
            Some(move_type),
            category,
            accuracy,
            false,
            first_effect(&battle_move),
        );
    }
    Ok(())
}

fn use_struggle(battle: &mut Battle, side: Side) {
    let attacker = battle.active(side).clone();
    reset_protect_streak(
        &mut battle.teams[side_index(side)].members[battle.active_slot(side).index()].state,
    );
    battle.events.push(BattleEvent::MoveUsed {
        side,
        pokemon: attacker.id().clone(),
        used_move: UsedMove::Struggle,
    });
    resolve_hit(
        battle,
        side,
        attacker,
        UsedMove::Struggle,
        50,
        None,
        MoveCategory::Physical,
        Accuracy::AlwaysHit,
        true,
        MoveEffect::None,
    );
}

fn resolve_status_move(
    battle: &mut Battle,
    side: Side,
    attacker: &BattleUnit,
    move_type: PokemonType,
    accuracy: Accuracy,
    effect: MoveEffect,
) {
    let target_side = side.opponent();
    let target = battle.active(target_side).clone();
    activate_accuracy_ability(battle, side, attacker, MoveCategory::Status, accuracy);
    let hit = check_accuracy(battle, accuracy, attacker, &target);
    if !hit {
        battle.events.push(BattleEvent::Missed {
            side,
            target_side,
            target: target.id().clone(),
        });
        return;
    }
    if let Some(ability) = ability_blocks_move(&target, move_type) {
        activate_move_blocking_ability(battle, target_side, &target, ability);
        battle.events.push(BattleEvent::Effectiveness {
            side,
            target_side,
            target: target.id().clone(),
            effectiveness: TypeEffectiveness::Immune,
        });
        return;
    }
    if effect.targets_opponent() && battle.protected[side_index(target_side)] {
        battle.events.push(BattleEvent::MoveBlocked {
            side,
            target_side,
            target: target.id().clone(),
        });
        return;
    }
    if effect.targets_opponent() && substitute_hp(&target).is_some() {
        battle.events.push(BattleEvent::SubstituteBlocked {
            side,
            target_side,
            target: target.id().clone(),
        });
        return;
    }
    apply_move_effect(battle, side, target_side, effect, false);
}

#[allow(clippy::too_many_arguments)]
fn resolve_hit(
    battle: &mut Battle,
    side: Side,
    attacker: BattleUnit,
    used_move: UsedMove,
    power: u16,
    move_type: Option<PokemonType>,
    category: MoveCategory,
    accuracy: Accuracy,
    recoil: bool,
    effect: MoveEffect,
) {
    let target_side = side.opponent();
    let target_slot = battle.active_slot(target_side);
    let target = battle.active(target_side).clone();
    activate_accuracy_ability(battle, side, &attacker, category, accuracy);
    let hit = check_accuracy(battle, accuracy, &attacker, &target);
    if !hit {
        battle.events.push(BattleEvent::Missed {
            side,
            target_side,
            target: target.id().clone(),
        });
        return;
    }
    if battle.protected[side_index(target_side)] {
        battle.events.push(BattleEvent::MoveBlocked {
            side,
            target_side,
            target: target.id().clone(),
        });
        return;
    }
    if let Some(ability) = move_type.and_then(|kind| ability_blocks_move(&target, kind)) {
        activate_move_blocking_ability(battle, target_side, &target, ability);
        battle.events.push(BattleEvent::Effectiveness {
            side,
            target_side,
            target: target.id().clone(),
            effectiveness: TypeEffectiveness::Immune,
        });
        if matches!(ability, Ability::WaterAbsorb | Ability::VoltAbsorb) {
            let amount = (u64::from(target.state.max_hp) / 4).max(1);
            let actual = heal(
                &mut battle.teams[side_index(target_side)].members[target_slot.index()].state,
                amount,
            );
            if actual > 0 {
                battle.events.push(BattleEvent::Healed {
                    side: target_side,
                    pokemon: target.id().clone(),
                    amount: actual,
                    current_hp: battle.active(target_side).state.current_hp,
                });
            }
        }
        return;
    }
    let fixed_damage = effect.fixed_damage_for(attacker.state.level);
    let critical_roll = fixed_damage.is_none() && battle.rng.range_inclusive(1, 16) == 1;
    let critical = match (critical_roll, target.state.ability.first().copied()) {
        (true, Some(ability @ (Ability::BattleArmor | Ability::ShellArmor))) => {
            battle.events.push(BattleEvent::AbilityActivated {
                side: target_side,
                pokemon: target.id().clone(),
                ability,
            });
            false
        }
        _ => critical_roll,
    };
    let effectiveness = match move_type {
        Some(attack_type) => rules::type_effectiveness(attack_type, &target),
        None => TypeEffectiveness::Normal,
    };
    if critical && effectiveness != TypeEffectiveness::Immune {
        battle.events.push(BattleEvent::Critical {
            side,
            target_side,
            target: target.id().clone(),
        });
    }
    battle.events.push(BattleEvent::Effectiveness {
        side,
        target_side,
        target: target.id().clone(),
        effectiveness,
    });
    let flash_fire_boosted = fixed_damage.is_none()
        && move_type == Some(PokemonType::Fire)
        && battle.flash_fire[side_index(side)];
    if flash_fire_boosted {
        battle.events.push(BattleEvent::AbilityActivated {
            side,
            pokemon: attacker.id().clone(),
            ability: Ability::FlashFire,
        });
    }
    if let Some(ability) = attacker.state.ability.first().copied()
        && fixed_damage.is_none()
        && effectiveness != TypeEffectiveness::Immune
        && category == MoveCategory::Physical
        && physical_attack_ability_is_active(&attacker)
    {
        battle.events.push(BattleEvent::AbilityActivated {
            side,
            pokemon: attacker.id().clone(),
            ability,
        });
    }
    if fixed_damage.is_none()
        && effectiveness != TypeEffectiveness::Immune
        && category == MoveCategory::Physical
        && defense_ability_is_active(&target)
    {
        battle.events.push(BattleEvent::AbilityActivated {
            side: target_side,
            pokemon: target.id().clone(),
            ability: Ability::MarvelScale,
        });
    }
    if let Some(ability) = attacker.state.ability.first().copied()
        && fixed_damage.is_none()
        && effectiveness != TypeEffectiveness::Immune
        && rules::low_hp_type_boost_applies(&attacker, move_type)
    {
        battle.events.push(BattleEvent::AbilityActivated {
            side,
            pokemon: attacker.id().clone(),
            ability,
        });
    }
    if fixed_damage.is_none()
        && effectiveness != TypeEffectiveness::Immune
        && rules::thick_fat_applies(&target, move_type)
    {
        battle.events.push(BattleEvent::AbilityActivated {
            side: target_side,
            pokemon: target.id().clone(),
            ability: Ability::ThickFat,
        });
    }
    let damage = match fixed_damage {
        Some(_) if effectiveness == TypeEffectiveness::Immune => 0,
        Some(damage) => damage,
        None => rules::calculate_damage(
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
            battle.rng.range_inclusive(85, 100) as u8,
            effective_weather(battle),
        ),
    };
    if let Some((actual, remaining_hp, broke)) = damage_substitute(
        &mut battle.teams[side_index(target_side)].members[target_slot.index()].state,
        damage,
    ) {
        battle.events.push(BattleEvent::SubstituteDamaged {
            side: target_side,
            pokemon: target.id().clone(),
            amount: actual,
            remaining_hp,
        });
        if broke {
            battle.events.push(BattleEvent::SubstituteBroke {
                side: target_side,
                pokemon: target.id().clone(),
            });
        }
        if recoil {
            apply_struggle_recoil(battle, side, &attacker, actual);
        } else {
            apply_damaging_move_effect(battle, side, target_side, &attacker, effect, actual, true);
        }
        return;
    }
    let actual = apply_damage(
        &mut battle.teams[side_index(target_side)].members[target_slot.index()].state,
        damage,
    );
    battle.events.push(BattleEvent::Damage {
        source: DamageSource::Move {
            side,
            pokemon: attacker.id().clone(),
            used_move,
        },
        target_side,
        target: target.id().clone(),
        amount: actual,
        remaining_hp: battle.active(target_side).state.current_hp,
    });
    if battle.active(target_side).is_fainted() {
        battle.events.push(BattleEvent::Fainted {
            side: target_side,
            pokemon: target.id().clone(),
        });
    }
    if actual > 0 {
        apply_damaging_move_effect(battle, side, target_side, &attacker, effect, actual, false);
        if !battle.active(target_side).is_fainted() {
            apply_move_effect(battle, side, target_side, effect, true);
        }
    }
    if recoil {
        apply_struggle_recoil(battle, side, &attacker, actual);
    }
}

fn can_act(battle: &mut Battle, side: Side) -> bool {
    let slot = battle.active_slot(side);
    let pokemon = battle.active(side).clone();
    if battle.flinched[side_index(side)] {
        battle.events.push(BattleEvent::Flinched {
            side,
            pokemon: pokemon.id().clone(),
        });
        return false;
    }
    match pokemon.state.major_status {
        Some(MajorStatus::Sleep { .. }) => {
            let remaining = advance_sleep(
                &mut battle.teams[side_index(side)].members[slot.index()].state,
                1,
            );
            let remaining = if pokemon.state.ability.contains(&Ability::EarlyBird)
                && remaining.is_some_and(|r| r > 0)
            {
                advance_sleep(
                    &mut battle.teams[side_index(side)].members[slot.index()].state,
                    1,
                )
            } else {
                remaining
            };
            if remaining == Some(0) {
                battle.events.push(BattleEvent::StatusCured {
                    side,
                    pokemon: pokemon.id().clone(),
                    status: MajorStatusKind::Sleep,
                });
                true
            } else if let Some(remaining) = remaining {
                battle.events.push(BattleEvent::StatusPreventsAction {
                    side,
                    pokemon: pokemon.id().clone(),
                    status: MajorStatus::Sleep {
                        turns_remaining: remaining,
                    },
                });
                false
            } else {
                false
            }
        }
        Some(MajorStatus::Freeze) => {
            battle.events.push(BattleEvent::StatusPreventsAction {
                side,
                pokemon: pokemon.id().clone(),
                status: MajorStatus::Freeze,
            });
            false
        }
        Some(MajorStatus::Paralysis) if battle.rng.range_inclusive(1, 4) == 1 => {
            battle.events.push(BattleEvent::StatusPreventsAction {
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
    battle: &mut Battle,
    side: Side,
    target_side: Side,
    effect: MoveEffect,
    damaging_secondary: bool,
) {
    if damaging_secondary
        && effect.is_non_damaging_secondary_effect()
        && effect.targets_opponent()
        && let Some(ability) = ability_blocks_secondary_effect(battle.active(target_side))
    {
        battle.events.push(BattleEvent::AbilityActivated {
            side: target_side,
            pokemon: battle.active(target_side).id().clone(),
            ability,
        });
        return;
    }
    match effect {
        MoveEffect::None => {}
        MoveEffect::InflictMajorStatus { status, chance } => {
            let chance = secondary_effect_chance(battle, side, chance, damaging_secondary);
            if battle.rng.range_inclusive(1, 100) > u64::from(chance) {
                return;
            }
            let applied = match status {
                MajorStatusKind::Burn => MajorStatus::Burn,
                MajorStatusKind::Freeze => MajorStatus::Freeze,
                MajorStatusKind::Paralysis => MajorStatus::Paralysis,
                MajorStatusKind::Poison => MajorStatus::Poison,
                MajorStatusKind::BadlyPoisoned => MajorStatus::BadlyPoisoned { stage: 1 },
                MajorStatusKind::Sleep => MajorStatus::Sleep {
                    turns_remaining: battle.rng.range_inclusive(1, 3) as u8,
                },
            };
            let target = battle.active(target_side).id().clone();
            let ability = if battle.active(target_side).state.major_status.is_none() {
                ability_blocks_status(battle.active(target_side), status)
            } else {
                None
            };
            let target_slot = battle.active_slot(target_side);
            if inflict_major_status(
                &mut battle.teams[side_index(target_side)].members[target_slot.index()].state,
                applied,
            ) {
                battle.events.push(BattleEvent::StatusApplied {
                    side: target_side,
                    pokemon: target.clone(),
                    status: applied,
                });
                apply_synchronize(battle, side, target_side, target, status);
            } else {
                if let Some(ability) = ability {
                    battle.events.push(BattleEvent::AbilityActivated {
                        side: target_side,
                        pokemon: target.clone(),
                        ability,
                    });
                }
                battle.events.push(BattleEvent::StatusFailed {
                    side,
                    target_side,
                    target,
                    status,
                });
            }
        }
        MoveEffect::ChangeStages { target, changes } => {
            apply_stage_changes(battle, side, target_side, target, changes);
        }
        MoveEffect::ChangeStagesWithChance {
            target,
            changes,
            chance,
        } => {
            let chance = secondary_effect_chance(battle, side, chance, damaging_secondary);
            if battle.rng.range_inclusive(1, 100) <= u64::from(chance) {
                apply_stage_changes(battle, side, target_side, target, changes);
            }
        }
        MoveEffect::HealUser {
            numerator,
            denominator,
        } => {
            let slot = battle.active_slot(side);
            let pokemon = battle.active(side).id().clone();
            let amount = (u64::from(battle.active(side).state.max_hp) * u64::from(numerator)
                / u64::from(denominator))
            .max(1);
            let actual = heal(
                &mut battle.teams[side_index(side)].members[slot.index()].state,
                amount,
            );
            if actual == 0 {
                battle.events.push(BattleEvent::EffectFailed {
                    side,
                    target_side: side,
                    target: pokemon,
                });
            } else {
                battle.events.push(BattleEvent::Healed {
                    side,
                    pokemon,
                    amount: actual,
                    current_hp: battle.active(side).state.current_hp,
                });
            }
        }
        MoveEffect::DrainUser { .. }
        | MoveEffect::FixedDamage(_)
        | MoveEffect::FlinchTarget { .. }
        | MoveEffect::RecoilUser { .. } => {}
        MoveEffect::CopyTargetStages => {
            copy_target_stages(battle, side, target_side);
        }
        MoveEffect::Haze => {
            for affected_side in [side, target_side] {
                let slot = battle.active_slot(affected_side);
                let pokemon = battle.active(affected_side).id().clone();
                for stat in BattleStat::ALL {
                    let previous = battle.active(affected_side).state.stages.get(stat);
                    if previous == 0 {
                        continue;
                    }
                    if change_stage(
                        &mut battle.teams[side_index(affected_side)].members[slot.index()].state,
                        stat,
                        -previous,
                    )
                    .is_some()
                    {
                        battle.events.push(BattleEvent::StatStageChanged {
                            side: affected_side,
                            pokemon: pokemon.clone(),
                            stat,
                            change: -previous,
                            stage: 0,
                        });
                    }
                }
            }
        }
        MoveEffect::Rest => {
            let slot = battle.active_slot(side);
            let pokemon = battle.active(side).id().clone();
            let Some((healed, previous_status)) =
                rest(&mut battle.teams[side_index(side)].members[slot.index()].state)
            else {
                battle.events.push(BattleEvent::EffectFailed {
                    side,
                    target_side: side,
                    target: pokemon,
                });
                return;
            };
            if let Some(status) = previous_status {
                battle.events.push(BattleEvent::StatusCured {
                    side,
                    pokemon: pokemon.clone(),
                    status: status.kind(),
                });
            }
            if healed > 0 {
                battle.events.push(BattleEvent::Healed {
                    side,
                    pokemon: pokemon.clone(),
                    amount: healed,
                    current_hp: battle.active(side).state.current_hp,
                });
            }
            battle.events.push(BattleEvent::StatusApplied {
                side,
                pokemon,
                status: MajorStatus::Sleep { turns_remaining: 3 },
            });
        }
        MoveEffect::Refresh => {
            let slot = battle.active_slot(side);
            let pokemon = battle.active(side).id().clone();
            match refresh(&mut battle.teams[side_index(side)].members[slot.index()].state) {
                Some(status) => battle.events.push(BattleEvent::StatusCured {
                    side,
                    pokemon,
                    status,
                }),
                None => battle.events.push(BattleEvent::EffectFailed {
                    side,
                    target_side: side,
                    target: pokemon,
                }),
            }
        }
        MoveEffect::CreateSubstitute => {
            let slot = battle.active_slot(side);
            let pokemon = battle.active(side).id().clone();
            if let Some(substitute_hp) =
                create_substitute(&mut battle.teams[side_index(side)].members[slot.index()].state)
            {
                battle.events.push(BattleEvent::SubstituteCreated {
                    side,
                    pokemon,
                    substitute_hp,
                    current_hp: battle.active(side).state.current_hp,
                });
            } else {
                battle.events.push(BattleEvent::EffectFailed {
                    side,
                    target_side: side,
                    target: pokemon,
                });
            }
        }
        MoveEffect::ProtectUser => {
            let slot = battle.active_slot(side);
            let pokemon = battle.active(side).id().clone();
            let streak = protect_streak(&battle.active(side).state);
            let denominator = 1_u64 << u32::from(streak.min(6));
            let succeeds = denominator == 1 || battle.rng.range_inclusive(1, denominator) == 1;
            if succeeds {
                record_protect_success(
                    &mut battle.teams[side_index(side)].members[slot.index()].state,
                );
                battle.protected[side_index(side)] = true;
                battle
                    .events
                    .push(BattleEvent::ProtectionActivated { side, pokemon });
            } else {
                reset_protect_streak(
                    &mut battle.teams[side_index(side)].members[slot.index()].state,
                );
                battle
                    .events
                    .push(BattleEvent::ProtectionFailed { side, pokemon });
            }
        }
        MoveEffect::StartWeather(weather) => {
            start_weather(battle, weather, Some(5));
        }
    }
}

fn apply_synchronize(
    battle: &mut Battle,
    source_side: Side,
    target_side: Side,
    target: battle_domain::BattleUnitId,
    status: MajorStatusKind,
) {
    let reflected = match status {
        MajorStatusKind::Burn => MajorStatus::Burn,
        MajorStatusKind::Paralysis => MajorStatus::Paralysis,
        MajorStatusKind::Poison => MajorStatus::Poison,
        _ => return,
    };
    if !battle
        .active(target_side)
        .state
        .ability
        .contains(&Ability::Synchronize)
        || battle.active(source_side).state.major_status.is_some()
        || !inflict_major_status(
            &mut battle.teams[side_index(source_side)].members
                [battle.active_slot(source_side).index()]
            .state,
            reflected,
        )
    {
        return;
    }
    let source = battle.active(source_side).id().clone();
    battle.events.push(BattleEvent::AbilityActivated {
        side: target_side,
        pokemon: target,
        ability: Ability::Synchronize,
    });
    battle.events.push(BattleEvent::StatusApplied {
        side: source_side,
        pokemon: source,
        status: reflected,
    });
}

fn apply_stage_changes(
    battle: &mut Battle,
    side: Side,
    target_side: Side,
    target: EffectTarget,
    changes: StageChanges,
) {
    let affected_side = match target {
        EffectTarget::User => side,
        EffectTarget::Opponent => target_side,
    };
    let pokemon = battle.active(affected_side).id().clone();
    let mut changed = false;
    for stat in BattleStat::ALL {
        let amount = changes.get(stat);
        if amount == 0 {
            continue;
        }
        let blocked_by = (target == EffectTarget::Opponent && amount < 0)
            .then(|| ability_blocks_opponent_stat_drop(battle.active(affected_side), stat))
            .flatten();
        if let Some(ability) = blocked_by {
            battle.events.push(BattleEvent::AbilityActivated {
                side: affected_side,
                pokemon: pokemon.clone(),
                ability,
            });
            continue;
        }
        let previous = battle.active(affected_side).state.stages.get(stat);
        if let Some(stage) = change_stage(
            &mut battle.teams[side_index(affected_side)].members
                [battle.active_slot(affected_side).index()]
            .state,
            stat,
            amount,
        ) {
            changed = true;
            battle.events.push(BattleEvent::StatStageChanged {
                side: affected_side,
                pokemon: pokemon.clone(),
                stat,
                change: stage - previous,
                stage,
            });
        }
    }
    if !changed {
        battle.events.push(BattleEvent::EffectFailed {
            side,
            target_side: affected_side,
            target: pokemon,
        });
    }
}

fn copy_target_stages(battle: &mut Battle, side: Side, target_side: Side) {
    let slot = battle.active_slot(side);
    let pokemon = battle.active(side).id().clone();
    let target_stages = battle.active(target_side).state.stages;
    let mut changed = false;
    for stat in BattleStat::ALL {
        let previous = battle.active(side).state.stages.get(stat);
        let target_stage = target_stages.get(stat);
        let change = target_stage - previous;
        if change == 0 {
            continue;
        }
        let Some(stage) = change_stage(
            &mut battle.teams[side_index(side)].members[slot.index()].state,
            stat,
            change,
        ) else {
            continue;
        };
        changed = true;
        battle.events.push(BattleEvent::StatStageChanged {
            side,
            pokemon: pokemon.clone(),
            stat,
            change,
            stage,
        });
    }
    if !changed {
        battle.events.push(BattleEvent::EffectFailed {
            side,
            target_side: side,
            target: pokemon,
        });
    }
}

fn check_accuracy(
    battle: &mut Battle,
    accuracy: Accuracy,
    attacker: &BattleUnit,
    target: &BattleUnit,
) -> bool {
    let Accuracy::Percent(base) = accuracy else {
        return true;
    };
    let (accuracy_numerator, accuracy_denominator) =
        accuracy_stage_fraction(attacker.state.stages.get(BattleStat::Accuracy));
    let (evasion_numerator, evasion_denominator) =
        accuracy_stage_fraction(target.state.stages.get(BattleStat::Evasion));
    let chance = (u64::from(base) * u64::from(accuracy_numerator) * u64::from(evasion_denominator)
        / (u64::from(accuracy_denominator) * u64::from(evasion_numerator)))
    .min(100);
    battle.rng.range_inclusive(1, 100) <= chance
}

fn accuracy_for_move(
    battle: &Battle,
    battle_move: &Move,
    attacker: &BattleUnit,
    category: MoveCategory,
) -> Accuracy {
    let weather_accuracy = weather_adjusted_accuracy(
        battle_move.weather_accuracy(),
        battle_move.accuracy(),
        effective_weather(battle),
    );
    match accuracy_ability(attacker, category, weather_accuracy) {
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
        _ => weather_accuracy,
    }
}

fn activate_accuracy_ability(
    battle: &mut Battle,
    side: Side,
    attacker: &BattleUnit,
    category: MoveCategory,
    accuracy: Accuracy,
) {
    let Some(ability) = accuracy_ability(attacker, category, accuracy) else {
        return;
    };
    battle.events.push(BattleEvent::AbilityActivated {
        side,
        pokemon: attacker.id().clone(),
        ability,
    });
}

fn activate_move_blocking_ability(
    battle: &mut Battle,
    side: Side,
    pokemon: &BattleUnit,
    ability: Ability,
) {
    battle.events.push(BattleEvent::AbilityActivated {
        side,
        pokemon: pokemon.id().clone(),
        ability,
    });
    if ability == Ability::FlashFire {
        battle.flash_fire[side_index(side)] = true;
    }
}

fn start_weather(battle: &mut Battle, weather: Weather, turns_remaining: Option<u8>) {
    battle.weather = Some(match turns_remaining {
        Some(turns) => WeatherState::with_turns(weather, turns),
        None => WeatherState::permanent(weather),
    });
    battle.events.push(BattleEvent::WeatherStarted {
        weather,
        turns_remaining,
    });
}

fn resolve_end_of_turn(battle: &mut Battle) {
    for side in [Side::One, Side::Two] {
        if battle.active(side).is_fainted() {
            continue;
        }
        let Some(
            status @ (MajorStatus::Burn | MajorStatus::Poison | MajorStatus::BadlyPoisoned { .. }),
        ) = battle.active(side).state.major_status
        else {
            continue;
        };
        let slot = battle.active_slot(side);
        let pokemon = battle.active(side).clone();
        let damage = match status {
            MajorStatus::BadlyPoisoned { stage } => {
                u64::from((pokemon.state.max_hp / 16).max(1)) * u64::from(stage)
            }
            MajorStatus::Burn | MajorStatus::Poison => u64::from((pokemon.state.max_hp / 8).max(1)),
            MajorStatus::Freeze | MajorStatus::Paralysis | MajorStatus::Sleep { .. } => continue,
        };
        let actual = apply_damage(
            &mut battle.teams[side_index(side)].members[slot.index()].state,
            damage,
        );
        battle.events.push(BattleEvent::Damage {
            source: DamageSource::Status {
                side,
                pokemon: pokemon.id().clone(),
                status,
            },
            target_side: side,
            target: pokemon.id().clone(),
            amount: actual,
            remaining_hp: battle.active(side).state.current_hp,
        });
        if battle.active(side).is_fainted() {
            battle.events.push(BattleEvent::Fainted {
                side,
                pokemon: pokemon.id().clone(),
            });
        }
        if matches!(status, MajorStatus::BadlyPoisoned { .. })
            && let Some(stage) = advance_badly_poison(
                &mut battle.teams[side_index(side)].members[slot.index()].state,
            )
        {
            battle.events.push(BattleEvent::StatusAdvanced {
                side,
                pokemon: pokemon.id().clone(),
                status: MajorStatus::BadlyPoisoned { stage },
            });
        }
    }
    resolve_weather_end_of_turn(battle);
    resolve_speed_boost_end_of_turn(battle);
    resolve_shed_skin_end_of_turn(battle);
}

fn resolve_speed_boost_end_of_turn(battle: &mut Battle) {
    for side in [Side::One, Side::Two] {
        if battle.active(side).is_fainted()
            || !battle
                .active(side)
                .state
                .ability
                .contains(&Ability::SpeedBoost)
        {
            continue;
        }
        let slot = battle.active_slot(side);
        let pokemon = battle.active(side).id().clone();
        let previous = battle.active(side).state.stages.get(BattleStat::Speed);
        let Some(stage) = change_stage(
            &mut battle.teams[side_index(side)].members[slot.index()].state,
            BattleStat::Speed,
            1,
        ) else {
            continue;
        };
        battle.events.push(BattleEvent::AbilityActivated {
            side,
            pokemon: pokemon.clone(),
            ability: Ability::SpeedBoost,
        });
        battle.events.push(BattleEvent::StatStageChanged {
            side,
            pokemon,
            stat: BattleStat::Speed,
            change: stage - previous,
            stage,
        });
    }
}

fn resolve_shed_skin_end_of_turn(battle: &mut Battle) {
    const SHED_SKIN_CHANCE: u64 = 30;
    for side in [Side::One, Side::Two] {
        if battle.active(side).is_fainted()
            || !battle
                .active(side)
                .state
                .ability
                .contains(&Ability::ShedSkin)
            || battle.active(side).state.major_status.is_none()
            || battle.rng.range_inclusive(1, 100) > SHED_SKIN_CHANCE
        {
            continue;
        }
        let slot = battle.active_slot(side);
        let pokemon = battle.active(side).id().clone();
        let Some(status) =
            cure_major_status(&mut battle.teams[side_index(side)].members[slot.index()].state)
        else {
            continue;
        };
        battle.events.push(BattleEvent::AbilityActivated {
            side,
            pokemon: pokemon.clone(),
            ability: Ability::ShedSkin,
        });
        battle.events.push(BattleEvent::StatusCured {
            side,
            pokemon,
            status,
        });
    }
}

fn resolve_weather_end_of_turn(battle: &mut Battle) {
    let Some(state) = battle.weather else {
        return;
    };
    if effective_weather(battle).is_some() {
        for side in [Side::One, Side::Two] {
            if battle.active(side).is_fainted()
                || !weather_damages(state.weather(), battle.active(side))
            {
                continue;
            }
            let slot = battle.active_slot(side);
            let pokemon = battle.active(side).clone();
            let amount = u64::from((pokemon.state.max_hp / 16).max(1));
            let actual = apply_damage(
                &mut battle.teams[side_index(side)].members[slot.index()].state,
                amount,
            );
            battle.events.push(BattleEvent::Damage {
                source: DamageSource::Weather {
                    weather: state.weather(),
                },
                target_side: side,
                target: pokemon.id().clone(),
                amount: actual,
                remaining_hp: battle.active(side).state.current_hp,
            });
            if battle.active(side).is_fainted() {
                battle.events.push(BattleEvent::Fainted {
                    side,
                    pokemon: pokemon.id().clone(),
                });
            }
        }
        resolve_weather_abilities_end_of_turn(battle, state.weather());
    }
    let Some(state) = battle.weather.as_mut() else {
        return;
    };
    let remaining = elapse_weather(state);
    match remaining {
        Some(0) => {
            let weather = state.weather();
            battle.weather = None;
            battle.events.push(BattleEvent::WeatherEnded { weather });
        }
        Some(remaining) => {
            battle.events.push(BattleEvent::WeatherUpdated {
                weather: state.weather(),
                turns_remaining: remaining,
            });
        }
        None => {}
    }
}

fn resolve_weather_abilities_end_of_turn(battle: &mut Battle, weather: Weather) {
    if weather != Weather::Rain {
        return;
    }
    for side in [Side::One, Side::Two] {
        if battle.active(side).is_fainted()
            || !battle
                .active(side)
                .state
                .ability
                .contains(&Ability::RainDish)
        {
            continue;
        }
        let slot = battle.active_slot(side);
        let pokemon = battle.active(side).clone();
        let amount = u64::from((pokemon.state.max_hp / 16).max(1));
        let actual = heal(
            &mut battle.teams[side_index(side)].members[slot.index()].state,
            amount,
        );
        if actual == 0 {
            continue;
        }
        battle.events.push(BattleEvent::AbilityActivated {
            side,
            pokemon: pokemon.id().clone(),
            ability: Ability::RainDish,
        });
        battle.events.push(BattleEvent::Healed {
            side,
            pokemon: pokemon.id().clone(),
            amount: actual,
            current_hp: battle.active(side).state.current_hp,
        });
    }
}

fn apply_damaging_move_effect(
    battle: &mut Battle,
    side: Side,
    target_side: Side,
    attacker: &BattleUnit,
    effect: MoveEffect,
    dealt: u32,
    hit_substitute: bool,
) {
    match effect {
        MoveEffect::DrainUser {
            numerator,
            denominator,
        } if !hit_substitute => {
            let amount = (u64::from(dealt) * u64::from(numerator) / u64::from(denominator)).max(1);
            let target = battle.active(target_side).clone();
            if target.state.ability.contains(&Ability::LiquidOoze) {
                battle.events.push(BattleEvent::AbilityActivated {
                    side: target_side,
                    pokemon: target.id().clone(),
                    ability: Ability::LiquidOoze,
                });
                let slot = battle.active_slot(side);
                let actual = apply_damage(
                    &mut battle.teams[side_index(side)].members[slot.index()].state,
                    amount,
                );
                battle.events.push(BattleEvent::Damage {
                    source: DamageSource::Ability {
                        side: target_side,
                        pokemon: target.id().clone(),
                        ability: Ability::LiquidOoze,
                    },
                    target_side: side,
                    target: attacker.id().clone(),
                    amount: actual,
                    remaining_hp: battle.active(side).state.current_hp,
                });
                if battle.active(side).is_fainted() {
                    battle.events.push(BattleEvent::Fainted {
                        side,
                        pokemon: attacker.id().clone(),
                    });
                }
            } else {
                let slot = battle.active_slot(side);
                let actual = heal(
                    &mut battle.teams[side_index(side)].members[slot.index()].state,
                    amount,
                );
                if actual > 0 {
                    battle.events.push(BattleEvent::Healed {
                        side,
                        pokemon: attacker.id().clone(),
                        amount: actual,
                        current_hp: battle.active(side).state.current_hp,
                    });
                }
            }
        }
        MoveEffect::RecoilUser {
            numerator,
            denominator,
        } => {
            if attacker.state.ability.contains(&Ability::RockHead) {
                battle.events.push(BattleEvent::AbilityActivated {
                    side,
                    pokemon: attacker.id().clone(),
                    ability: Ability::RockHead,
                });
            } else {
                apply_recoil(battle, side, attacker, dealt, numerator, denominator);
            }
        }
        MoveEffect::FixedDamage(_) => {}
        MoveEffect::FlinchTarget { chance } if !hit_substitute => {
            if let Some(ability) = ability_blocks_secondary_effect(battle.active(target_side)) {
                battle.events.push(BattleEvent::AbilityActivated {
                    side: target_side,
                    pokemon: battle.active(target_side).id().clone(),
                    ability,
                });
                return;
            }
            let chance = secondary_effect_chance(battle, side, chance, true);
            if battle.rng.range_inclusive(1, 100) > u64::from(chance) {
                return;
            }
            let target = battle.active(target_side);
            if target.state.ability.contains(&Ability::InnerFocus) {
                battle.events.push(BattleEvent::AbilityActivated {
                    side: target_side,
                    pokemon: target.id().clone(),
                    ability: Ability::InnerFocus,
                });
            } else {
                battle.flinched[side_index(target_side)] = true;
            }
        }
        _ => {}
    }
}

fn secondary_effect_chance(
    battle: &mut Battle,
    side: Side,
    chance: u8,
    damaging_secondary: bool,
) -> u8 {
    if !damaging_secondary
        || chance == 100
        || !battle
            .active(side)
            .state
            .ability
            .contains(&Ability::SereneGrace)
    {
        return chance;
    }
    battle.events.push(BattleEvent::AbilityActivated {
        side,
        pokemon: battle.active(side).id().clone(),
        ability: Ability::SereneGrace,
    });
    chance.saturating_mul(2).min(100)
}

fn apply_struggle_recoil(battle: &mut Battle, side: Side, attacker: &BattleUnit, dealt: u32) {
    apply_recoil(battle, side, attacker, dealt, 1, 4);
}

fn apply_recoil(
    battle: &mut Battle,
    side: Side,
    attacker: &BattleUnit,
    dealt: u32,
    numerator: u8,
    denominator: u8,
) {
    let slot = battle.active_slot(side);
    let recoil = (u64::from(dealt) * u64::from(numerator) / u64::from(denominator)).max(1);
    let actual = apply_damage(
        &mut battle.teams[side_index(side)].members[slot.index()].state,
        recoil,
    );
    battle.events.push(BattleEvent::Damage {
        source: DamageSource::Recoil {
            side,
            pokemon: attacker.id().clone(),
        },
        target_side: side,
        target: attacker.id().clone(),
        amount: actual,
        remaining_hp: battle.active(side).state.current_hp,
    });
    if battle.active(side).is_fainted() {
        battle.events.push(BattleEvent::Fainted {
            side,
            pokemon: attacker.id().clone(),
        });
    }
}

fn update_phase_after_turn(battle: &mut Battle) {
    let one_living = battle.teams[0]
        .members
        .iter()
        .any(|unit| !unit.is_fainted());
    let two_living = battle.teams[1]
        .members
        .iter()
        .any(|unit| !unit.is_fainted());
    let outcome = match (one_living, two_living) {
        (false, false) => Some(BattleOutcome::Draw),
        (true, false) => Some(BattleOutcome::Winner(Side::One)),
        (false, true) => Some(BattleOutcome::Winner(Side::Two)),
        (true, true) => None,
    };
    if let Some(outcome) = outcome {
        battle.phase = BattlePhase::Finished(outcome);
        battle.events.push(BattleEvent::BattleFinished { outcome });
        return;
    }
    let side_one = battle.active(Side::One).is_fainted();
    let side_two = battle.active(Side::Two).is_fainted();
    let replacements = match (side_one, side_two) {
        (true, true) => Some(battle_domain::ReplacementSides::Both),
        (true, false) => Some(battle_domain::ReplacementSides::One),
        (false, true) => Some(battle_domain::ReplacementSides::Two),
        (false, false) => None,
    };
    if let Some(replacements) = replacements {
        battle.phase = BattlePhase::ForcedReplacement(replacements);
        if side_one {
            battle
                .events
                .push(BattleEvent::ForcedReplacement { side: Side::One });
        }
        if side_two {
            battle
                .events
                .push(BattleEvent::ForcedReplacement { side: Side::Two });
        }
    } else {
        battle.phase = BattlePhase::Turn;
    }
}

fn resolve_replacements(battle: &mut Battle) -> Result<(), BattleError> {
    for side in [Side::One, Side::Two] {
        if battle.phase.requires_replacement(side) {
            let pending =
                battle.pending[side_index(side)]
                    .take()
                    .ok_or(BattleError::StateInconsistent {
                        detail: "required replacement is missing a command",
                    })?;
            let to = pending.replacement.ok_or(BattleError::StateInconsistent {
                detail: "replacement command is missing a switch target",
            })?;
            switch(battle, side, to);
        }
    }
    battle.pending = [None, None];
    battle.phase = BattlePhase::Turn;
    Ok(())
}

fn legal_switches(battle: &Battle, side: Side) -> Vec<Action> {
    let active = battle.active_slot(side);
    battle
        .team(side)
        .members
        .iter()
        .enumerate()
        .filter_map(|(index, unit)| {
            let slot = TeamSlot::from_valid_index(index);
            (slot != active && !unit.is_fainted()).then_some(Action::Switch(slot))
        })
        .collect()
}

fn is_trapped(battle: &Battle, side: Side) -> bool {
    let target = battle.active(side);
    let opponent = battle.active(side.opponent());
    match opponent.state.ability.first().copied() {
        Some(Ability::ShadowTag) => !target.state.ability.contains(&Ability::ShadowTag),
        Some(Ability::ArenaTrap) => {
            !target.species.types.contains(&PokemonType::Flying)
                && !target.state.ability.contains(&Ability::Levitate)
        }
        _ => false,
    }
}

fn effective_speed(battle: &Battle, side: Side) -> u16 {
    let unit = battle.active(side);
    let base = rules::effective_speed(unit);
    let weather = effective_weather(battle);
    let abilities = &unit.state.ability;
    match (
        abilities.contains(&Ability::Chlorophyll),
        abilities.contains(&Ability::SwiftSwim),
        weather,
    ) {
        (true, _, Some(Weather::Sun)) | (_, true, Some(Weather::Rain)) => base.saturating_mul(2),
        _ => base,
    }
}

fn effective_weather(battle: &Battle) -> Option<Weather> {
    let weather = battle.weather.map(WeatherState::weather)?;
    let suppressed = [Side::One, Side::Two].into_iter().any(|side| {
        !battle.active(side).is_fainted()
            && battle
                .active(side)
                .state
                .ability
                .iter()
                .any(|ability| matches!(ability, Ability::AirLock | Ability::CloudNine))
    });
    (!suppressed).then_some(weather)
}

fn weather_damages(weather: Weather, unit: &BattleUnit) -> bool {
    match weather {
        Weather::Hail => !unit.species.types.contains(&PokemonType::Ice),
        Weather::Sandstorm => {
            !unit.species.types.contains(&PokemonType::Rock)
                && !unit.species.types.contains(&PokemonType::Ground)
                && !unit.species.types.contains(&PokemonType::Steel)
                && !unit.state.ability.contains(&Ability::SandVeil)
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
        _ => (3, 3),
    }
}

fn weather_adjusted_move(
    modifier: Option<battle_domain::WeatherMoveModifier>,
    power: u16,
    move_type: Option<PokemonType>,
    category: MoveCategory,
    weather: Option<Weather>,
) -> (u16, PokemonType, MoveCategory) {
    let move_type = move_type.unwrap_or(PokemonType::Normal);
    match (modifier, weather) {
        (Some(battle_domain::WeatherMoveModifier::WeatherBall), Some(Weather::Hail)) => {
            (power * 2, PokemonType::Ice, MoveCategory::Special)
        }
        (Some(battle_domain::WeatherMoveModifier::WeatherBall), Some(Weather::Rain)) => {
            (power * 2, PokemonType::Water, MoveCategory::Special)
        }
        (Some(battle_domain::WeatherMoveModifier::WeatherBall), Some(Weather::Sandstorm)) => {
            (power * 2, PokemonType::Rock, MoveCategory::Physical)
        }
        (Some(battle_domain::WeatherMoveModifier::WeatherBall), Some(Weather::Sun)) => {
            (power * 2, PokemonType::Fire, MoveCategory::Special)
        }
        _ => (power, move_type, category),
    }
}

fn weather_adjusted_accuracy(
    modifier: Option<WeatherAccuracyModifier>,
    accuracy: Accuracy,
    weather: Option<Weather>,
) -> Accuracy {
    match (modifier, weather) {
        (Some(WeatherAccuracyModifier::Thunder), Some(Weather::Rain)) => Accuracy::AlwaysHit,
        (Some(WeatherAccuracyModifier::Thunder), Some(Weather::Sun)) => Accuracy::Percent(50),
        _ => accuracy,
    }
}

fn first_effect(battle_move: &Move) -> MoveEffect {
    battle_move
        .effects()
        .first()
        .copied()
        .unwrap_or(MoveEffect::None)
}

fn substitute_hp(unit: &BattleUnit) -> Option<u32> {
    unit.state.volatile_statuses.get(VolatileStatus::Substitute)
}

fn elapse_weather(state: &mut WeatherState) -> Option<u8> {
    let remaining = state.turns_remaining()?;
    let next = remaining.saturating_sub(1);
    *state = if next > 0 {
        WeatherState::with_turns(state.weather(), next)
    } else {
        WeatherState::with_turns(state.weather(), 0)
    };
    Some(next)
}

// ---- 状态变更辅助（操作 BattleState 的公开字段） ----

fn apply_damage(state: &mut BattleState, damage: u64) -> u32 {
    let actual = damage.min(u64::from(state.current_hp)) as u32;
    state.current_hp -= actual;
    actual
}

fn heal(state: &mut BattleState, amount: u64) -> u32 {
    let missing = state.max_hp - state.current_hp;
    let actual = amount.min(u64::from(missing)) as u32;
    state.current_hp += actual;
    actual
}

fn change_stage(state: &mut BattleState, stat: BattleStat, amount: i8) -> Option<i8> {
    let previous = state.stages.get(stat);
    let next = previous.saturating_add(amount).clamp(-6, 6);
    if next == previous {
        return None;
    }
    let _ = state.stages.set(stat, next);
    Some(next)
}

fn advance_sleep(state: &mut BattleState, ticks: u8) -> Option<u8> {
    let MajorStatus::Sleep { turns_remaining } = state.major_status? else {
        return None;
    };
    let next = turns_remaining.saturating_sub(ticks);
    state.major_status = (next > 0).then_some(MajorStatus::Sleep {
        turns_remaining: next,
    });
    Some(next)
}

fn advance_badly_poison(state: &mut BattleState) -> Option<u8> {
    let MajorStatus::BadlyPoisoned { stage } = state.major_status? else {
        return None;
    };
    let next = stage.saturating_add(1);
    state.major_status = Some(MajorStatus::BadlyPoisoned { stage: next });
    Some(next)
}

fn inflict_major_status(state: &mut BattleState, status: MajorStatus) -> bool {
    if state.major_status.is_some() {
        return false;
    }
    state.major_status = Some(status);
    true
}

fn cure_major_status(state: &mut BattleState) -> Option<MajorStatusKind> {
    let status = state.major_status?;
    state.major_status = None;
    Some(status.kind())
}

fn refresh(state: &mut BattleState) -> Option<MajorStatusKind> {
    let status = state.major_status?;
    if !matches!(
        status,
        MajorStatus::BadlyPoisoned { .. }
            | MajorStatus::Burn
            | MajorStatus::Paralysis
            | MajorStatus::Poison
    ) {
        return None;
    }
    state.major_status = None;
    Some(status.kind())
}

fn rest(state: &mut BattleState) -> Option<(u32, Option<MajorStatus>)> {
    if state.current_hp == state.max_hp && state.major_status.is_none() {
        return None;
    }
    let previous_status = state.major_status;
    let healed = state.max_hp - state.current_hp;
    state.current_hp = state.max_hp;
    state.major_status = Some(MajorStatus::Sleep { turns_remaining: 3 });
    Some((healed, previous_status))
}

fn create_substitute(state: &mut BattleState) -> Option<u32> {
    if state
        .volatile_statuses
        .get(VolatileStatus::Substitute)
        .is_some()
    {
        return None;
    }
    let cost = (state.max_hp / 4).max(1);
    if state.current_hp <= cost {
        return None;
    }
    state.current_hp -= cost;
    state
        .volatile_statuses
        .set(VolatileStatus::Substitute, cost);
    Some(cost)
}

fn damage_substitute(state: &mut BattleState, damage: u64) -> Option<(u32, u32, bool)> {
    let hp = state.volatile_statuses.get(VolatileStatus::Substitute)?;
    let actual = damage.min(u64::from(hp)) as u32;
    let remaining = hp - actual;
    if remaining > 0 {
        state
            .volatile_statuses
            .set(VolatileStatus::Substitute, remaining);
    } else {
        state.volatile_statuses.remove(VolatileStatus::Substitute);
    }
    Some((actual, remaining, remaining == 0))
}

fn protect_streak(state: &BattleState) -> u8 {
    state
        .volatile_statuses
        .get(VolatileStatus::ProtectStreak)
        .unwrap_or(0) as u8
}

fn record_protect_success(state: &mut BattleState) {
    let next = protect_streak(state).saturating_add(1);
    state
        .volatile_statuses
        .set(VolatileStatus::ProtectStreak, u32::from(next));
}

fn reset_protect_streak(state: &mut BattleState) {
    state
        .volatile_statuses
        .remove(VolatileStatus::ProtectStreak);
}

fn reset_switch_modifiers(state: &mut BattleState) {
    state.stages = StatStages::neutral();
    state
        .volatile_statuses
        .remove(VolatileStatus::ProtectStreak);
    state.volatile_statuses.remove(VolatileStatus::Substitute);
    if matches!(state.major_status, Some(MajorStatus::BadlyPoisoned { .. })) {
        state.major_status = Some(MajorStatus::BadlyPoisoned { stage: 1 });
    }
}

// ---- 特性判定辅助 ----

fn ability_blocks_status(unit: &BattleUnit, status: MajorStatusKind) -> Option<Ability> {
    unit.state
        .ability
        .iter()
        .find(|ability| {
            matches!(
                (*ability, status),
                (
                    Ability::Immunity,
                    MajorStatusKind::Poison | MajorStatusKind::BadlyPoisoned
                ) | (Ability::Limber, MajorStatusKind::Paralysis)
                    | (Ability::WaterVeil, MajorStatusKind::Burn)
                    | (
                        Ability::Insomnia | Ability::VitalSpirit,
                        MajorStatusKind::Sleep
                    )
                    | (Ability::MagmaArmor, MajorStatusKind::Freeze)
            )
        })
        .copied()
}

fn ability_blocks_move(unit: &BattleUnit, move_type: PokemonType) -> Option<Ability> {
    unit.state
        .ability
        .iter()
        .find(|ability| {
            matches!(
                (*ability, move_type),
                (Ability::Levitate, PokemonType::Ground)
                    | (Ability::FlashFire, PokemonType::Fire)
                    | (Ability::WaterAbsorb, PokemonType::Water)
                    | (Ability::VoltAbsorb, PokemonType::Electric)
            )
        })
        .copied()
}

fn ability_blocks_secondary_effect(unit: &BattleUnit) -> Option<Ability> {
    unit.state
        .ability
        .contains(&Ability::ShieldDust)
        .then_some(Ability::ShieldDust)
}

fn ability_blocks_opponent_stat_drop(unit: &BattleUnit, stat: BattleStat) -> Option<Ability> {
    unit.state
        .ability
        .iter()
        .find(|ability| {
            matches!(
                (*ability, stat),
                (Ability::ClearBody | Ability::WhiteSmoke, _)
                    | (Ability::HyperCutter, BattleStat::Attack)
                    | (Ability::KeenEye, BattleStat::Accuracy)
            )
        })
        .copied()
}

fn accuracy_ability(
    unit: &BattleUnit,
    category: MoveCategory,
    accuracy: Accuracy,
) -> Option<Ability> {
    let abilities = &unit.state.ability;
    if abilities.contains(&Ability::CompoundEyes) && matches!(accuracy, Accuracy::Percent(_)) {
        return Some(Ability::CompoundEyes);
    }
    if abilities.contains(&Ability::Hustle)
        && category == MoveCategory::Physical
        && matches!(accuracy, Accuracy::Percent(_))
    {
        return Some(Ability::Hustle);
    }
    None
}

fn physical_attack_ability_is_active(unit: &BattleUnit) -> bool {
    let abilities = &unit.state.ability;
    abilities.iter().any(|ability| {
        matches!(
            ability,
            Ability::HugePower | Ability::PurePower | Ability::Hustle
        )
    }) || (abilities.contains(&Ability::Guts) && unit.state.major_status.is_some())
}

fn defense_ability_is_active(unit: &BattleUnit) -> bool {
    unit.state.ability.contains(&Ability::MarvelScale) && unit.state.major_status.is_some()
}
