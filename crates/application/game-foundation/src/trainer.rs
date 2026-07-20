use std::{collections::BTreeSet, fmt};

use serde::{Deserialize, Serialize};

use crate::{GameIdError, TrainerId};

pub const TRAINER_CATALOG_FORMAT: &str = "trainer-v1";
const MAX_TRAINER_NAME_BYTES: usize = 64;
const MAX_TRAINER_SCRIPT_BYTES: usize = 4_096;
const MAX_TRAINER_POKEMON: usize = 6;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TrainerPokemon {
    species: String,
    level: u8,
}

impl TrainerPokemon {
    pub fn new(species: impl Into<String>, level: u8) -> Result<Self, TrainerError> {
        let species = species.into();
        if species.trim().is_empty() {
            return Err(TrainerError::EmptyPokemonSpecies);
        }
        if !(1..=100).contains(&level) {
            return Err(TrainerError::InvalidPokemonLevel(level));
        }
        Ok(Self { species, level })
    }

    pub fn species(&self) -> &str {
        &self.species
    }

    pub const fn level(&self) -> u8 {
        self.level
    }

    fn validate(&self) -> Result<(), TrainerError> {
        if self.species.trim().is_empty() {
            return Err(TrainerError::EmptyPokemonSpecies);
        }
        if !(1..=100).contains(&self.level) {
            return Err(TrainerError::InvalidPokemonLevel(self.level));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TrainerDefinition {
    id: TrainerId,
    name: String,
    pokemon: Vec<TrainerPokemon>,
    script: String,
}

impl TrainerDefinition {
    pub fn new(
        id: TrainerId,
        name: impl Into<String>,
        pokemon: Vec<TrainerPokemon>,
        script: impl Into<String>,
    ) -> Result<Self, TrainerError> {
        let name = name.into();
        let script = script.into();
        validate_name(&name)?;
        validate_pokemon(&pokemon)?;
        for member in &pokemon {
            member.validate()?;
        }
        validate_script(&script)?;
        Ok(Self {
            id,
            name,
            pokemon,
            script,
        })
    }

    pub fn id(&self) -> &TrainerId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn pokemon(&self) -> &[TrainerPokemon] {
        &self.pokemon
    }

    pub fn script(&self) -> &str {
        &self.script
    }

    fn validate(&self) -> Result<(), TrainerError> {
        validate_name(&self.name)?;
        validate_pokemon(&self.pokemon)?;
        for pokemon in &self.pokemon {
            pokemon.validate()?;
        }
        validate_script(&self.script)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TrainerCatalog {
    format_version: String,
    trainers: Vec<TrainerDefinition>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum TrainerEditCommand {
    SetName {
        trainer: TrainerId,
        name: String,
    },
    SetScript {
        trainer: TrainerId,
        script: String,
    },
    AddPokemon {
        trainer: TrainerId,
        pokemon: TrainerPokemon,
    },
    ReplacePokemon {
        trainer: TrainerId,
        slot: usize,
        pokemon: TrainerPokemon,
    },
    RemovePokemon {
        trainer: TrainerId,
        slot: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrainerError {
    Id(GameIdError),
    Json(String),
    UnsupportedFormat(String),
    EmptyCatalog,
    DuplicateTrainer(TrainerId),
    UnknownTrainer(TrainerId),
    EmptyName,
    NameTooLong(usize),
    EmptyPokemonSpecies,
    InvalidPokemonLevel(u8),
    EmptyPokemonRoster,
    PokemonRosterTooLarge(usize),
    PokemonSlotMissing { trainer: TrainerId, slot: usize },
    EmptyScript,
    ScriptTooLong(usize),
}

impl fmt::Display for TrainerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Id(error) => write!(formatter, "invalid trainer identifier: {error:?}"),
            Self::Json(error) => write!(formatter, "trainer catalog JSON error: {error}"),
            Self::UnsupportedFormat(format) => {
                write!(formatter, "unsupported trainer catalog format: {format}")
            }
            Self::EmptyCatalog => formatter.write_str("trainer catalog must not be empty"),
            Self::DuplicateTrainer(trainer) => {
                write!(formatter, "duplicate trainer: {}", trainer.as_str())
            }
            Self::UnknownTrainer(trainer) => {
                write!(formatter, "unknown trainer: {}", trainer.as_str())
            }
            Self::EmptyName => formatter.write_str("trainer name must not be empty"),
            Self::NameTooLong(length) => {
                write!(formatter, "trainer name is too long: {length} bytes")
            }
            Self::EmptyPokemonSpecies => {
                formatter.write_str("trainer Pokemon species must not be empty")
            }
            Self::InvalidPokemonLevel(level) => {
                write!(formatter, "trainer Pokemon level is invalid: {level}")
            }
            Self::EmptyPokemonRoster => {
                formatter.write_str("trainer must have at least one Pokemon")
            }
            Self::PokemonRosterTooLarge(length) => {
                write!(formatter, "trainer has too many Pokemon: {length}")
            }
            Self::PokemonSlotMissing { trainer, slot } => {
                write!(
                    formatter,
                    "trainer {} has no Pokemon at slot {slot}",
                    trainer.as_str()
                )
            }
            Self::EmptyScript => formatter.write_str("trainer script must not be empty"),
            Self::ScriptTooLong(length) => {
                write!(formatter, "trainer script is too long: {length} bytes")
            }
        }
    }
}

impl std::error::Error for TrainerError {}

impl From<GameIdError> for TrainerError {
    fn from(value: GameIdError) -> Self {
        Self::Id(value)
    }
}

impl TrainerCatalog {
    pub fn new(mut trainers: Vec<TrainerDefinition>) -> Result<Self, TrainerError> {
        if trainers.is_empty() {
            return Err(TrainerError::EmptyCatalog);
        }
        let mut ids = BTreeSet::new();
        for trainer in &trainers {
            trainer.validate()?;
            if !ids.insert(trainer.id.clone()) {
                return Err(TrainerError::DuplicateTrainer(trainer.id.clone()));
            }
        }
        trainers.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(Self {
            format_version: TRAINER_CATALOG_FORMAT.to_owned(),
            trainers,
        })
    }

    pub fn standard() -> Result<Self, TrainerError> {
        Self::new(vec![TrainerDefinition::new(
            TrainerId::new("route-rival")?,
            "路线训练家 小遥",
            vec![TrainerPokemon::new("Zigzagoon", 5)?],
            "前方是训练家的道路。准备好就来对战吧。",
        )?])
    }

    pub fn from_json(json: &str) -> Result<Self, TrainerError> {
        let catalog = serde_json::from_str::<Self>(json)
            .map_err(|error| TrainerError::Json(error.to_string()))?;
        if catalog.format_version != TRAINER_CATALOG_FORMAT {
            return Err(TrainerError::UnsupportedFormat(catalog.format_version));
        }
        Self::new(catalog.trainers)
    }

    pub fn to_json_pretty(&self) -> Result<String, TrainerError> {
        serde_json::to_string_pretty(self).map_err(|error| TrainerError::Json(error.to_string()))
    }

    pub fn trainers(&self) -> &[TrainerDefinition] {
        &self.trainers
    }

    pub fn trainer(&self, id: &TrainerId) -> Option<&TrainerDefinition> {
        self.trainers.iter().find(|trainer| trainer.id() == id)
    }

    pub fn transition(&self, command: TrainerEditCommand) -> Result<Self, TrainerError> {
        let mut next = self.clone();
        match command {
            TrainerEditCommand::SetName { trainer, name } => {
                next.replace_trainer(trainer, |current| {
                    TrainerDefinition::new(
                        current.id.clone(),
                        name,
                        current.pokemon.clone(),
                        current.script.clone(),
                    )
                })?;
            }
            TrainerEditCommand::SetScript { trainer, script } => {
                next.replace_trainer(trainer, |current| {
                    TrainerDefinition::new(
                        current.id.clone(),
                        current.name.clone(),
                        current.pokemon.clone(),
                        script,
                    )
                })?;
            }
            TrainerEditCommand::AddPokemon { trainer, pokemon } => {
                next.replace_trainer(trainer, |current| {
                    let mut roster = current.pokemon.clone();
                    roster.push(pokemon);
                    TrainerDefinition::new(
                        current.id.clone(),
                        current.name.clone(),
                        roster,
                        current.script.clone(),
                    )
                })?;
            }
            TrainerEditCommand::ReplacePokemon {
                trainer,
                slot,
                pokemon,
            } => {
                next.replace_trainer(trainer, |current| {
                    let mut roster = current.pokemon.clone();
                    let Some(existing) = roster.get_mut(slot) else {
                        return Err(TrainerError::PokemonSlotMissing {
                            trainer: current.id.clone(),
                            slot,
                        });
                    };
                    *existing = pokemon;
                    TrainerDefinition::new(
                        current.id.clone(),
                        current.name.clone(),
                        roster,
                        current.script.clone(),
                    )
                })?;
            }
            TrainerEditCommand::RemovePokemon { trainer, slot } => {
                next.replace_trainer(trainer, |current| {
                    let mut roster = current.pokemon.clone();
                    if slot >= roster.len() {
                        return Err(TrainerError::PokemonSlotMissing {
                            trainer: current.id.clone(),
                            slot,
                        });
                    }
                    roster.remove(slot);
                    TrainerDefinition::new(
                        current.id.clone(),
                        current.name.clone(),
                        roster,
                        current.script.clone(),
                    )
                })?;
            }
        }
        next.validate()?;
        Ok(next)
    }

    fn replace_trainer(
        &mut self,
        id: TrainerId,
        update: impl FnOnce(&TrainerDefinition) -> Result<TrainerDefinition, TrainerError>,
    ) -> Result<(), TrainerError> {
        let Some(index) = self.trainers.iter().position(|trainer| trainer.id() == &id) else {
            return Err(TrainerError::UnknownTrainer(id));
        };
        let Some(current) = self.trainers.get(index).cloned() else {
            return Err(TrainerError::UnknownTrainer(id));
        };
        let replacement = update(&current)?;
        let Some(target) = self.trainers.get_mut(index) else {
            return Err(TrainerError::UnknownTrainer(id));
        };
        *target = replacement;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), TrainerError> {
        if self.format_version != TRAINER_CATALOG_FORMAT {
            return Err(TrainerError::UnsupportedFormat(self.format_version.clone()));
        }
        let _ = Self::new(self.trainers.clone())?;
        Ok(())
    }
}

fn validate_name(name: &str) -> Result<(), TrainerError> {
    if name.trim().is_empty() {
        return Err(TrainerError::EmptyName);
    }
    if name.len() > MAX_TRAINER_NAME_BYTES {
        return Err(TrainerError::NameTooLong(name.len()));
    }
    Ok(())
}

fn validate_pokemon(pokemon: &[TrainerPokemon]) -> Result<(), TrainerError> {
    if pokemon.is_empty() {
        return Err(TrainerError::EmptyPokemonRoster);
    }
    if pokemon.len() > MAX_TRAINER_POKEMON {
        return Err(TrainerError::PokemonRosterTooLarge(pokemon.len()));
    }
    Ok(())
}

fn validate_script(script: &str) -> Result<(), TrainerError> {
    if script.trim().is_empty() {
        return Err(TrainerError::EmptyScript);
    }
    if script.len() > MAX_TRAINER_SCRIPT_BYTES {
        return Err(TrainerError::ScriptTooLong(script.len()));
    }
    Ok(())
}
