use crate::enums::PokemonType;
use crate::id::{BattleUnitId, MoveId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationError {
    InvalidTeamSlot { index: usize },
    InvalidMoveSlot { index: usize },
    EmptyPokemonId,
    EmptyMoveId,
    EmptyPokemonName,
    EmptyMoveName,
    EmptyMoveType,
    EmptySpeciesType,
    InvalidLevel { level: u8 },
    DuplicatePokemonType { primary_type: PokemonType },
    ZeroMaxHp,
    CurrentHpExceedsMax { current: u32, max: u32 },
    ZeroStat { stat: &'static str },
    ZeroMovePower,
    InvalidAccuracy { value: u8 },
    InvalidEffectChance { value: u8 },
    InvalidStageChange,
    EmptyStageChanges,
    InvalidHealFraction { numerator: u8, denominator: u8 },
    ZeroMaxPp,
    CurrentPpExceedsMax { current: u8, max: u8 },
    InvalidMoveCount { count: usize },
    InvalidTeamSize { count: usize },
    DuplicateMoveId { id: MoveId },
    DuplicatePokemonId { id: BattleUnitId },
}
