//! Immutable battle ruleset contracts and executable Gen3 baseline validation.
//!
//! This crate owns the release-facing ruleset identity and decides whether a
//! data-backed form, move, ability, and type can enter the current engine.
//! It does not read files, construct windows, or own a battle session.

#![forbid(unsafe_code)]

use std::fmt;

use battle_domain::{
    Ability, Battle, BattleCommand, BattleError, BattleEvent, BattlePhase, PokemonType, Side, Team,
};
use game_data::{
    CurrentDataSet, DamageClass, MoveId, MoveRecord, PokemonFormId, SpeciesId, TypeId,
};

pub const LEGACY_GEN3_RULESET_ID: &str = "legacy-gen3";
pub const LEGACY_GEN3_RULESET_REVISION: u16 = 1;
pub const FIRST_ALLOWED_SPECIES: u32 = 1;
pub const LAST_ALLOWED_SPECIES: u32 = 386;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BattleRulesetId(String);

impl BattleRulesetId {
    pub fn new(value: impl Into<String>) -> Result<Self, RulesetError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(RulesetError::EmptyRulesetId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BattleRulesetRef {
    id: BattleRulesetId,
    revision: u16,
}

impl BattleRulesetRef {
    pub fn new(id: BattleRulesetId, revision: u16) -> Result<Self, RulesetError> {
        if revision == 0 {
            return Err(RulesetError::InvalidRevision(revision));
        }
        Ok(Self { id, revision })
    }

    pub fn id(&self) -> &BattleRulesetId {
        &self.id
    }

    pub const fn revision(&self) -> u16 {
        self.revision
    }

    pub fn storage_key(&self) -> String {
        format!("{}@{}", self.id.as_str(), self.revision)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BattleRuleset {
    reference: BattleRulesetRef,
    reference_target: &'static str,
}

impl BattleRuleset {
    pub fn legacy_gen3_r1() -> Result<Self, RulesetError> {
        Ok(Self {
            reference: BattleRulesetRef::new(
                BattleRulesetId::new(LEGACY_GEN3_RULESET_ID)?,
                LEGACY_GEN3_RULESET_REVISION,
            )?,
            reference_target: "project legacy Gen3 deterministic baseline",
        })
    }

    pub fn reference(&self) -> &BattleRulesetRef {
        &self.reference
    }

    pub fn reference_target(&self) -> &str {
        self.reference_target
    }

    pub fn supports_species(&self, species: SpeciesId) -> bool {
        (FIRST_ALLOWED_SPECIES..=LAST_ALLOWED_SPECIES).contains(&species.0)
    }

    pub fn validate_form(
        &self,
        data: &CurrentDataSet,
        form: PokemonFormId,
    ) -> Result<(), RulesetError> {
        let record = data
            .pokemon(form)
            .ok_or(RulesetError::UnknownPokemon(form))?;
        if !record.is_default {
            return Err(RulesetError::UnsupportedForm(form));
        }
        if !self.supports_species(record.species_id) {
            return Err(RulesetError::UnsupportedSpecies(record.species_id));
        }
        for type_id in &record.types {
            self.validate_type(data, *type_id)?;
        }
        Ok(())
    }

    pub fn validate_move(
        &self,
        data: &CurrentDataSet,
        form: PokemonFormId,
        move_id: MoveId,
        level: u8,
    ) -> Result<(), RulesetError> {
        self.validate_form(data, form)?;
        if !data.can_learn_at_level(form, move_id, level) {
            return Err(RulesetError::MoveNotLearnable {
                form,
                move_id,
                level,
            });
        }
        let record = data
            .move_by_id(move_id)
            .ok_or(RulesetError::UnknownMove(move_id))?;
        self.validate_move_record(data, record)
    }

    pub fn resolve_default_form(
        &self,
        data: &CurrentDataSet,
        species: &str,
    ) -> Result<PokemonFormId, RulesetError> {
        let form = data
            .pokemon_iter()
            .find(|record| {
                record.is_default
                    && (record.identifier.eq_ignore_ascii_case(species)
                        || record.display_name.english.eq_ignore_ascii_case(species))
            })
            .map(|record| record.id)
            .ok_or_else(|| RulesetError::UnknownSpeciesName(species.to_owned()))?;
        self.validate_form(data, form)?;
        Ok(form)
    }

    pub fn validate_member(
        &self,
        data: &CurrentDataSet,
        species: &str,
        level: u8,
    ) -> Result<RulesetMember, RulesetError> {
        let form = self.resolve_default_form(data, species)?;
        let ability = self.select_ability(data, form)?;
        let move_id = data
            .learnset(form)
            .into_iter()
            .flatten()
            .map(|entry| entry.move_id)
            .find(|move_id| self.validate_move(data, form, *move_id, level).is_ok())
            .ok_or(RulesetError::NoSupportedMove { form, level })?;
        Ok(RulesetMember {
            form,
            ability,
            move_id,
        })
    }

    pub fn validate_move_record(
        &self,
        data: &CurrentDataSet,
        record: &MoveRecord,
    ) -> Result<(), RulesetError> {
        self.validate_type(data, record.move_type)?;
        if record.pp.is_none_or(|pp| pp == 0) {
            return Err(RulesetError::UnsupportedMove(record.id));
        }
        let effect_supported =
            matches!(record.effect_id, None | Some(1)) || supported_effect_id(record.effect_id);
        let executable = match record.damage_class {
            DamageClass::Physical | DamageClass::Special => {
                record.power.is_some() && effect_supported
            }
            DamageClass::Status => effect_supported && supported_effect_id(record.effect_id),
        };
        if executable {
            Ok(())
        } else {
            Err(RulesetError::UnsupportedMove(record.id))
        }
    }

    pub fn select_ability(
        &self,
        data: &CurrentDataSet,
        form: PokemonFormId,
    ) -> Result<Ability, RulesetError> {
        self.validate_form(data, form)?;
        let record = data
            .pokemon(form)
            .ok_or(RulesetError::UnknownPokemon(form))?;
        record
            .abilities
            .iter()
            .filter(|entry| !entry.is_hidden)
            .filter_map(|entry| data.ability_by_id(entry.ability_id))
            .find_map(|entry| map_ability_identifier(&entry.identifier))
            .ok_or(RulesetError::NoSupportedAbility(form))
    }

    pub fn replay(
        &self,
        team_one: Team,
        team_two: Team,
        seed: u64,
        commands: &[BattleCommand],
    ) -> Result<BattleReplay, ReplayError> {
        let mut battle = Battle::new(team_one, team_two, seed)?;
        let mut events = Vec::new();
        for command in commands {
            events.extend(battle.submit(*command)?.events().iter().cloned());
        }
        Ok(BattleReplay {
            ruleset: self.reference.clone(),
            commands: commands.to_vec(),
            events,
            phase: battle.phase(),
            team_one: battle.team(Side::One).clone(),
            team_two: battle.team(Side::Two).clone(),
        })
    }

    fn validate_type(&self, data: &CurrentDataSet, type_id: TypeId) -> Result<(), RulesetError> {
        let record = data
            .type_by_id(type_id)
            .ok_or(RulesetError::UnknownType(type_id))?;
        map_type_identifier(&record.identifier)
            .map(|_| ())
            .ok_or_else(|| RulesetError::UnsupportedType {
                id: type_id,
                identifier: record.identifier.clone(),
            })
    }
}

pub fn map_type_identifier(identifier: &str) -> Option<PokemonType> {
    match identifier {
        "normal" => Some(PokemonType::Normal),
        "fighting" => Some(PokemonType::Fighting),
        "flying" => Some(PokemonType::Flying),
        "poison" => Some(PokemonType::Poison),
        "ground" => Some(PokemonType::Ground),
        "rock" => Some(PokemonType::Rock),
        "bug" => Some(PokemonType::Bug),
        "ghost" => Some(PokemonType::Ghost),
        "steel" => Some(PokemonType::Steel),
        "fire" => Some(PokemonType::Fire),
        "water" => Some(PokemonType::Water),
        "grass" => Some(PokemonType::Grass),
        "electric" => Some(PokemonType::Electric),
        "psychic" => Some(PokemonType::Psychic),
        "ice" => Some(PokemonType::Ice),
        "dragon" => Some(PokemonType::Dragon),
        "dark" => Some(PokemonType::Dark),
        _ => None,
    }
}

pub fn map_ability_identifier(identifier: &str) -> Option<Ability> {
    match identifier {
        "air-lock" => Some(Ability::AirLock),
        "arena-trap" => Some(Ability::ArenaTrap),
        "battle-armor" => Some(Ability::BattleArmor),
        "blaze" => Some(Ability::Blaze),
        "chlorophyll" => Some(Ability::Chlorophyll),
        "clear-body" => Some(Ability::ClearBody),
        "cloud-nine" => Some(Ability::CloudNine),
        "compound-eyes" => Some(Ability::CompoundEyes),
        "drizzle" => Some(Ability::Drizzle),
        "drought" => Some(Ability::Drought),
        "early-bird" => Some(Ability::EarlyBird),
        "flash-fire" => Some(Ability::FlashFire),
        "guts" => Some(Ability::Guts),
        "huge-power" => Some(Ability::HugePower),
        "hyper-cutter" => Some(Ability::HyperCutter),
        "hustle" => Some(Ability::Hustle),
        "immunity" => Some(Ability::Immunity),
        "intimidate" => Some(Ability::Intimidate),
        "inner-focus" => Some(Ability::InnerFocus),
        "keen-eye" => Some(Ability::KeenEye),
        "insomnia" => Some(Ability::Insomnia),
        "levitate" => Some(Ability::Levitate),
        "limber" => Some(Ability::Limber),
        "liquid-ooze" => Some(Ability::LiquidOoze),
        "magma-armor" => Some(Ability::MagmaArmor),
        "marvel-scale" => Some(Ability::MarvelScale),
        "natural-cure" => Some(Ability::NaturalCure),
        "overgrow" => Some(Ability::Overgrow),
        "pressure" => Some(Ability::Pressure),
        "pure-power" => Some(Ability::PurePower),
        "rain-dish" => Some(Ability::RainDish),
        "rock-head" => Some(Ability::RockHead),
        "sand-stream" => Some(Ability::SandStream),
        "sand-veil" => Some(Ability::SandVeil),
        "serene-grace" => Some(Ability::SereneGrace),
        "shell-armor" => Some(Ability::ShellArmor),
        "shed-skin" => Some(Ability::ShedSkin),
        "shield-dust" => Some(Ability::ShieldDust),
        "shadow-tag" => Some(Ability::ShadowTag),
        "synchronize" => Some(Ability::Synchronize),
        "speed-boost" => Some(Ability::SpeedBoost),
        "swift-swim" => Some(Ability::SwiftSwim),
        "swarm" => Some(Ability::Swarm),
        "thick-fat" => Some(Ability::ThickFat),
        "torrent" => Some(Ability::Torrent),
        "vital-spirit" => Some(Ability::VitalSpirit),
        "volt-absorb" => Some(Ability::VoltAbsorb),
        "water-absorb" => Some(Ability::WaterAbsorb),
        "water-veil" => Some(Ability::WaterVeil),
        "white-smoke" => Some(Ability::WhiteSmoke),
        _ => None,
    }
}

const fn supported_effect_id(effect_id: Option<u16>) -> bool {
    matches!(
        effect_id,
        Some(
            2 | 3
                | 4
                | 5
                | 6
                | 7
                | 11
                | 12
                | 17
                | 19
                | 20
                | 21
                | 24
                | 25
                | 26
                | 32
                | 33
                | 34
                | 38
                | 42
                | 49
                | 52
                | 53
                | 54
                | 55
                | 59
                | 60
                | 61
                | 62
                | 63
                | 68
                | 69
                | 70
                | 71
                | 72
                | 73
                | 74
                | 80
                | 88
                | 112
                | 116
                | 131
                | 137
                | 138
                | 144
                | 159
                | 165
                | 168
                | 194
                | 199
                | 212
                | 278
                | 317
                | 328
        )
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RulesetMember {
    form: PokemonFormId,
    ability: Ability,
    move_id: MoveId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BattleReplay {
    ruleset: BattleRulesetRef,
    commands: Vec<BattleCommand>,
    events: Vec<BattleEvent>,
    phase: BattlePhase,
    team_one: Team,
    team_two: Team,
}

impl BattleReplay {
    pub fn ruleset(&self) -> &BattleRulesetRef {
        &self.ruleset
    }

    pub fn commands(&self) -> &[BattleCommand] {
        &self.commands
    }

    pub fn events(&self) -> &[BattleEvent] {
        &self.events
    }

    pub const fn phase(&self) -> BattlePhase {
        self.phase
    }

    pub fn team(&self, side: Side) -> &Team {
        match side {
            Side::One => &self.team_one,
            Side::Two => &self.team_two,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayError {
    Battle(BattleError),
}

impl From<BattleError> for ReplayError {
    fn from(value: BattleError) -> Self {
        Self::Battle(value)
    }
}

impl RulesetMember {
    pub const fn form(&self) -> PokemonFormId {
        self.form
    }

    pub const fn ability(&self) -> Ability {
        self.ability
    }

    pub const fn move_id(&self) -> MoveId {
        self.move_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RulesetError {
    EmptyRulesetId,
    InvalidRevision(u16),
    UnknownPokemon(PokemonFormId),
    UnsupportedForm(PokemonFormId),
    UnsupportedSpecies(SpeciesId),
    UnknownSpeciesName(String),
    UnknownMove(MoveId),
    UnsupportedMove(MoveId),
    MoveNotLearnable {
        form: PokemonFormId,
        move_id: MoveId,
        level: u8,
    },
    UnknownType(TypeId),
    UnsupportedType {
        id: TypeId,
        identifier: String,
    },
    NoSupportedAbility(PokemonFormId),
    NoSupportedMove {
        form: PokemonFormId,
        level: u8,
    },
}

impl fmt::Display for RulesetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRulesetId => formatter.write_str("ruleset ID must not be empty"),
            Self::InvalidRevision(revision) => {
                write!(formatter, "invalid ruleset revision: {revision}")
            }
            Self::UnknownPokemon(form) => write!(formatter, "unknown Pokemon form: {}", form.0),
            Self::UnsupportedForm(form) => {
                write!(formatter, "unsupported Pokemon form: {}", form.0)
            }
            Self::UnsupportedSpecies(species) => {
                write!(formatter, "unsupported species: {}", species.0)
            }
            Self::UnknownSpeciesName(species) => write!(formatter, "unknown species: {species}"),
            Self::UnknownMove(move_id) => write!(formatter, "unknown move: {}", move_id.0),
            Self::UnsupportedMove(move_id) => write!(formatter, "unsupported move: {}", move_id.0),
            Self::MoveNotLearnable {
                form,
                move_id,
                level,
            } => write!(
                formatter,
                "form {} cannot learn move {} at level {level}",
                form.0, move_id.0
            ),
            Self::UnknownType(type_id) => write!(formatter, "unknown type: {}", type_id.0),
            Self::UnsupportedType { id, identifier } => {
                write!(formatter, "unsupported type {}: {identifier}", id.0)
            }
            Self::NoSupportedAbility(form) => {
                write!(formatter, "form {} has no supported ability", form.0)
            }
            Self::NoSupportedMove { form, level } => {
                write!(
                    formatter,
                    "form {} has no supported move at level {level}",
                    form.0
                )
            }
        }
    }
}

impl std::error::Error for RulesetError {}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
