use battle_domain::{
    Ability, BattleStat, BattleUnit, MajorStatus, MoveCategory, PokemonType, TypeEffectiveness,
    Weather,
};

/// 攻击属性相对于一只宝可梦全部属性的倍率。
pub(crate) fn type_effectiveness(attack: PokemonType, defender: &BattleUnit) -> TypeEffectiveness {
    let mut result = TypeEffectiveness::Normal;
    for defense in defender.species.types.iter() {
        let factor = single_type_factor(attack, *defense);
        result = combine(result, factor);
    }
    result
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SingleTypeFactor {
    Immune,
    Half,
    Normal,
    Double,
}

fn combine(current: TypeEffectiveness, factor: SingleTypeFactor) -> TypeEffectiveness {
    use SingleTypeFactor::{Double, Half, Immune, Normal};
    match (current, factor) {
        (_, Immune) => TypeEffectiveness::Immune,
        (TypeEffectiveness::Immune, _) => TypeEffectiveness::Immune,
        (TypeEffectiveness::Quarter, Double) | (TypeEffectiveness::Double, Double) => {
            TypeEffectiveness::Quadruple
        }
        (TypeEffectiveness::Quadruple, Double) | (TypeEffectiveness::Quadruple, Normal) => {
            TypeEffectiveness::Quadruple
        }
        (TypeEffectiveness::Double, Half) | (TypeEffectiveness::Half, Double) => {
            TypeEffectiveness::Normal
        }
        (TypeEffectiveness::Double, Normal) | (TypeEffectiveness::Normal, Double) => {
            TypeEffectiveness::Double
        }
        (TypeEffectiveness::Half, Half) => TypeEffectiveness::Quarter,
        (TypeEffectiveness::Half, Normal) | (TypeEffectiveness::Normal, Half) => {
            TypeEffectiveness::Half
        }
        (TypeEffectiveness::Normal, Normal) => TypeEffectiveness::Normal,
        (current, _) => current,
    }
}

fn single_type_factor(attack: PokemonType, defense: PokemonType) -> SingleTypeFactor {
    use PokemonType::*;
    use SingleTypeFactor::{Double, Half, Immune, Normal as Neutral};
    match (attack, defense) {
        (Normal, Ghost)
        | (Fighting, Ghost)
        | (Poison, Steel)
        | (Ground, Flying)
        | (Electric, Ground)
        | (Psychic, Dark)
        | (Ghost, Normal) => Immune,
        (Normal, Rock | Steel)
        | (Fire, Fire | Water | Rock | Dragon)
        | (Water, Water | Grass | Dragon)
        | (Electric, Electric | Grass | Dragon)
        | (Grass, Fire | Grass | Poison | Flying | Bug | Dragon | Steel)
        | (Ice, Fire | Water | Ice | Steel)
        | (Fighting, Poison | Flying | Psychic | Bug)
        | (Poison, Poison | Ground | Rock | Ghost)
        | (Ground, Grass | Bug)
        | (Flying, Electric | Rock | Steel)
        | (Psychic, Psychic | Steel)
        | (Bug, Fire | Fighting | Poison | Flying | Ghost | Steel)
        | (Rock, Fighting | Ground | Steel)
        | (Ghost, Dark | Steel)
        | (Dragon, Steel)
        | (Dark, Fighting | Dark | Steel)
        | (Steel, Fire | Water | Electric | Steel) => Half,
        (Fire, Grass | Ice | Bug | Steel)
        | (Water, Fire | Ground | Rock)
        | (Electric, Water | Flying)
        | (Grass, Water | Ground | Rock)
        | (Ice, Grass | Ground | Flying | Dragon)
        | (Fighting, Normal | Ice | Rock | Dark | Steel)
        | (Poison, Grass)
        | (Ground, Fire | Electric | Poison | Rock | Steel)
        | (Flying, Grass | Fighting | Bug)
        | (Psychic, Fighting | Poison)
        | (Bug, Grass | Psychic | Dark)
        | (Rock, Fire | Ice | Flying | Bug)
        | (Ghost, Psychic | Ghost)
        | (Dragon, Dragon)
        | (Dark, Psychic | Ghost)
        | (Steel, Ice | Rock) => Double,
        _ => Neutral,
    }
}

/// 返回应用攻击特性与烧伤后的物理攻击，不包含能力阶级修正。
pub(crate) fn physical_attack(unit: &BattleUnit) -> u16 {
    let state = &unit.state;
    let attack = match (state.ability.first().copied(), state.major_status) {
        (Some(Ability::Guts), Some(_)) => state.stats.attack.saturating_mul(3) / 2,
        (_, Some(MajorStatus::Burn)) => state.stats.attack / 2,
        _ => state.stats.attack,
    };
    match state.ability.first().copied() {
        Some(Ability::HugePower | Ability::PurePower) => attack.saturating_mul(2),
        Some(Ability::Hustle) => attack.saturating_mul(3) / 2,
        _ => attack,
    }
}

/// 返回应用攻击特性和攻击阶级后的物理攻击值。
pub(crate) fn effective_attack(unit: &BattleUnit) -> u16 {
    stage_modified_stat(
        physical_attack(unit),
        unit.state.stages.get(BattleStat::Attack),
    )
}

/// 返回应用防御特性和防御阶级后的物理防御值。
pub(crate) fn effective_defense(unit: &BattleUnit) -> u16 {
    let defense = if unit.state.ability.contains(&Ability::MarvelScale)
        && unit.state.major_status.is_some()
    {
        unit.state.stats.defense.saturating_mul(3) / 2
    } else {
        unit.state.stats.defense
    };
    stage_modified_stat(defense, unit.state.stages.get(BattleStat::Defense))
}

pub(crate) fn effective_special_attack(unit: &BattleUnit) -> u16 {
    stage_modified_stat(
        unit.state.stats.special_attack,
        unit.state.stages.get(BattleStat::SpecialAttack),
    )
}

pub(crate) fn effective_special_defense(unit: &BattleUnit) -> u16 {
    stage_modified_stat(
        unit.state.stats.special_defense,
        unit.state.stages.get(BattleStat::SpecialDefense),
    )
}

/// 返回应用麻痹与速度阶级后的有效速度。
pub(crate) fn effective_speed(unit: &BattleUnit) -> u16 {
    let speed = match unit.state.major_status {
        Some(MajorStatus::Paralysis) => (unit.state.stats.speed / 4).max(1),
        _ => unit.state.stats.speed,
    };
    stage_modified_stat(speed, unit.state.stages.get(BattleStat::Speed))
}

fn stage_modified_stat(value: u16, stage: i8) -> u16 {
    let value = u32::from(value);
    let adjusted = if stage >= 0 {
        value * u32::from(2 + stage as u8) / 2
    } else {
        value * 2 / u32::from(2 + (-stage) as u8)
    };
    adjusted.max(1) as u16
}

/// 按第三世代公式计算招式伤害。
#[allow(clippy::too_many_arguments)]
pub(crate) fn calculate_damage(
    attacker: &BattleUnit,
    defender: &BattleUnit,
    power: u16,
    move_type: Option<PokemonType>,
    category: MoveCategory,
    critical: bool,
    random_percent: u8,
    weather: Option<Weather>,
) -> u64 {
    let (attack, defense) = damage_stats(attacker, defender, category, critical);
    let level_factor = u64::from(attacker.state.level) * 2 / 5 + 2;
    let mut damage =
        level_factor * u64::from(power) * u64::from(attack) / u64::from(defense) / 50 + 2;
    if critical {
        damage *= 2;
    }
    if let Some(attack_type) = move_type {
        if attacker.species.types.contains(&attack_type) {
            damage = damage * 3 / 2;
        }
        let effectiveness = type_effectiveness(attack_type, defender);
        damage = match effectiveness {
            TypeEffectiveness::Immune => return 0,
            TypeEffectiveness::Quarter => damage / 4,
            TypeEffectiveness::Half => damage / 2,
            TypeEffectiveness::Normal => damage,
            TypeEffectiveness::Double => damage * 2,
            TypeEffectiveness::Quadruple => damage * 4,
        };
    }
    if let Some(weather) = weather {
        damage =
            match (weather, move_type) {
                (Weather::Rain, Some(PokemonType::Water))
                | (Weather::Sun, Some(PokemonType::Fire)) => damage * 3 / 2,
                (Weather::Rain, Some(PokemonType::Fire))
                | (Weather::Sun, Some(PokemonType::Water)) => damage / 2,
                _ => damage,
            };
    }
    if low_hp_type_boost_applies(attacker, move_type) {
        damage = damage * 3 / 2;
    }
    if thick_fat_applies(defender, move_type) {
        damage /= 2;
    }
    damage = damage * u64::from(random_percent) / 100;
    damage.max(1)
}

pub(crate) fn low_hp_type_boost_applies(
    attacker: &BattleUnit,
    move_type: Option<PokemonType>,
) -> bool {
    attacker.state.current_hp * 3 <= attacker.state.max_hp
        && matches!(
            (attacker.state.ability.first().copied(), move_type),
            (Some(Ability::Blaze), Some(PokemonType::Fire))
                | (Some(Ability::Overgrow), Some(PokemonType::Grass))
                | (Some(Ability::Swarm), Some(PokemonType::Bug))
                | (Some(Ability::Torrent), Some(PokemonType::Water))
        )
}

pub(crate) fn thick_fat_applies(defender: &BattleUnit, move_type: Option<PokemonType>) -> bool {
    defender.state.ability.contains(&Ability::ThickFat)
        && matches!(move_type, Some(PokemonType::Fire | PokemonType::Ice))
}

fn damage_stats(
    attacker: &BattleUnit,
    defender: &BattleUnit,
    category: MoveCategory,
    critical: bool,
) -> (u16, u16) {
    let (attack_stat, defense_stat) = match category {
        MoveCategory::Physical => (BattleStat::Attack, BattleStat::Defense),
        MoveCategory::Special => (BattleStat::SpecialAttack, BattleStat::SpecialDefense),
        MoveCategory::Status => (BattleStat::Attack, BattleStat::Defense),
    };
    let ignore_stages = critical
        && attacker.state.stages.get(attack_stat) <= defender.state.stages.get(defense_stat);
    if ignore_stages {
        return match category {
            MoveCategory::Physical => (physical_attack(attacker), defender.state.stats.defense),
            MoveCategory::Special => (
                attacker.state.stats.special_attack,
                defender.state.stats.special_defense,
            ),
            MoveCategory::Status => (0, 1),
        };
    }
    match category {
        MoveCategory::Physical => (effective_attack(attacker), effective_defense(defender)),
        MoveCategory::Special => (
            effective_special_attack(attacker),
            effective_special_defense(defender),
        ),
        MoveCategory::Status => (0, 1),
    }
}
