use crate::{
    Ability, Accuracy, BattleStat, MoveCategory, PokemonType, Weather, WeatherAccuracyModifier,
    WeatherMoveModifier, model::Pokemon,
};

/// 第三世代按属性划分的伤害类别。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DamageCategory {
    Physical,
    Special,
}

/// 返回属性在第三世代规则中的物理或特殊伤害类别。
pub const fn damage_category(move_type: PokemonType) -> DamageCategory {
    match move_type {
        PokemonType::Normal
        | PokemonType::Fighting
        | PokemonType::Flying
        | PokemonType::Poison
        | PokemonType::Ground
        | PokemonType::Rock
        | PokemonType::Bug
        | PokemonType::Ghost
        | PokemonType::Steel => DamageCategory::Physical,
        PokemonType::Fire
        | PokemonType::Water
        | PokemonType::Grass
        | PokemonType::Electric
        | PokemonType::Psychic
        | PokemonType::Ice
        | PokemonType::Dragon
        | PokemonType::Dark => DamageCategory::Special,
    }
}

/// 攻击属性相对于一只宝可梦全部属性的倍率。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypeEffectiveness {
    Immune,
    Quarter,
    Half,
    Normal,
    Double,
    Quadruple,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SingleTypeFactor {
    Immune,
    Half,
    Normal,
    Double,
}

/// 计算攻击属性对一个主属性和可选副属性的总倍率。
pub const fn type_effectiveness(
    attack: PokemonType,
    primary: PokemonType,
    secondary: Option<PokemonType>,
) -> TypeEffectiveness {
    let first = single_type_factor(attack, primary);
    let second = match secondary {
        Some(defense) => single_type_factor(attack, defense),
        None => SingleTypeFactor::Normal,
    };
    combine_factors(first, second)
}

pub(crate) const fn weather_adjusted_accuracy(
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

pub(crate) const fn weather_adjusted_move(
    modifier: Option<WeatherMoveModifier>,
    power: u16,
    move_type: PokemonType,
    category: MoveCategory,
    weather: Option<Weather>,
) -> (u16, PokemonType, MoveCategory) {
    match (modifier, weather) {
        (Some(WeatherMoveModifier::WeatherBall), Some(Weather::Hail)) => {
            (power * 2, PokemonType::Ice, MoveCategory::Special)
        }
        (Some(WeatherMoveModifier::WeatherBall), Some(Weather::Rain)) => {
            (power * 2, PokemonType::Water, MoveCategory::Special)
        }
        (Some(WeatherMoveModifier::WeatherBall), Some(Weather::Sandstorm)) => {
            (power * 2, PokemonType::Rock, MoveCategory::Physical)
        }
        (Some(WeatherMoveModifier::WeatherBall), Some(Weather::Sun)) => {
            (power * 2, PokemonType::Fire, MoveCategory::Special)
        }
        _ => (power, move_type, category),
    }
}

const fn combine_factors(first: SingleTypeFactor, second: SingleTypeFactor) -> TypeEffectiveness {
    use SingleTypeFactor::{Double, Half, Immune, Normal};
    match (first, second) {
        (Immune, _) | (_, Immune) => TypeEffectiveness::Immune,
        (Half, Half) => TypeEffectiveness::Quarter,
        (Double, Double) => TypeEffectiveness::Quadruple,
        (Half, Double) | (Double, Half) | (Normal, Normal) => TypeEffectiveness::Normal,
        (Half, Normal) | (Normal, Half) => TypeEffectiveness::Half,
        (Double, Normal) | (Normal, Double) => TypeEffectiveness::Double,
    }
}

pub(crate) const fn single_type_factor(
    attack: PokemonType,
    defense: PokemonType,
) -> SingleTypeFactor {
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn calculate_damage(
    attacker: &Pokemon,
    defender: &Pokemon,
    power: u16,
    move_type: Option<PokemonType>,
    category: DamageCategory,
    critical: bool,
    random_percent: u8,
    weather: Option<Weather>,
) -> u64 {
    let (attack, defense) = damage_stats(attacker, defender, category, critical);

    // 等级、威力和能力值均受 u8/u16 限制，因此所有第三世代修正后的中间值仍小于 u64::MAX。
    let level_factor = u64::from(attacker.level()) * 2 / 5 + 2;
    let mut damage =
        level_factor * u64::from(power) * u64::from(attack) / u64::from(defense) / 50 + 2;

    if critical {
        damage *= 2;
    }
    if let Some(attack_type) = move_type {
        if attacker.primary_type() == attack_type || attacker.secondary_type() == Some(attack_type)
        {
            damage = damage * 3 / 2;
        }
        for defense_type in [Some(defender.primary_type()), defender.secondary_type()]
            .into_iter()
            .flatten()
        {
            damage = match single_type_factor(attack_type, defense_type) {
                SingleTypeFactor::Immune => return 0,
                SingleTypeFactor::Half => damage / 2,
                SingleTypeFactor::Normal => damage,
                SingleTypeFactor::Double => damage * 2,
            };
        }
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
    attacker: &Pokemon,
    move_type: Option<PokemonType>,
) -> bool {
    attacker.current_hp() * 3 <= attacker.max_hp()
        && matches!(
            (attacker.ability(), move_type),
            (Some(Ability::Blaze), Some(PokemonType::Fire))
                | (Some(Ability::Overgrow), Some(PokemonType::Grass))
                | (Some(Ability::Swarm), Some(PokemonType::Bug))
                | (Some(Ability::Torrent), Some(PokemonType::Water))
        )
}

pub(crate) fn thick_fat_applies(defender: &Pokemon, move_type: Option<PokemonType>) -> bool {
    defender.ability() == Some(Ability::ThickFat)
        && matches!(move_type, Some(PokemonType::Fire | PokemonType::Ice))
}

fn damage_stats(
    attacker: &Pokemon,
    defender: &Pokemon,
    category: DamageCategory,
    critical: bool,
) -> (u16, u16) {
    let (attack_stat, defense_stat) = match category {
        DamageCategory::Physical => (BattleStat::Attack, BattleStat::Defense),
        DamageCategory::Special => (BattleStat::SpecialAttack, BattleStat::SpecialDefense),
    };
    let ignore_stages =
        critical && attacker.stages().get(attack_stat) <= defender.stages().get(defense_stat);
    if ignore_stages {
        return match category {
            DamageCategory::Physical => (attacker.physical_attack(), defender.stats().defense()),
            DamageCategory::Special => (
                attacker.stats().special_attack(),
                defender.stats().special_defense(),
            ),
        };
    }
    match category {
        DamageCategory::Physical => (attacker.effective_attack(), defender.effective_defense()),
        DamageCategory::Special => (
            attacker.effective_special_attack(),
            defender.effective_special_defense(),
        ),
    }
}
