use crate::{PokemonType, model::Pokemon};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DamageCategory {
    Physical,
    Special,
}

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

pub(crate) fn calculate_damage(
    attacker: &Pokemon,
    defender: &Pokemon,
    power: u16,
    move_type: Option<PokemonType>,
    category: DamageCategory,
    critical: bool,
    random_percent: u8,
) -> u64 {
    let (attack, defense) = match category {
        DamageCategory::Physical => (attacker.stats().attack(), defender.stats().defense()),
        DamageCategory::Special => (
            attacker.stats().special_attack(),
            defender.stats().special_defense(),
        ),
    };

    // Level, power, and stats are bounded by u8/u16 inputs, so this chain is
    // below u64::MAX even after every generation-three modifier is applied.
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
    damage = damage * u64::from(random_percent) / 100;
    damage.max(1)
}
