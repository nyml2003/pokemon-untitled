//! Pure authored Pokemon catalog editor state and typed edit commands.

#![forbid(unsafe_code)]

use std::{collections::BTreeSet, error::Error, fmt};

use editor_application::{EditorCore, EditorDiagnostic};
use serde::{Deserialize, Deserializer, Serialize};

pub const POKEMON_CATALOG_FORMAT: &str = "pokemon-v1";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct PokemonId(String);

impl PokemonId {
    pub fn new(value: impl Into<String>) -> Result<Self, PokemonEditorError> {
        let value = value.into();
        if value.is_empty() || value.len() > 64 {
            return Err(PokemonEditorError::InvalidId(value));
        }
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(PokemonEditorError::InvalidId(value));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for PokemonId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PokemonDefinition {
    id: PokemonId,
    national_dex: u16,
    name: String,
    types: Vec<String>,
    base_hp: u16,
}

impl PokemonDefinition {
    pub fn new(
        id: PokemonId,
        national_dex: u16,
        name: impl Into<String>,
        types: Vec<String>,
        base_hp: u16,
    ) -> Result<Self, PokemonEditorError> {
        let definition = Self {
            id,
            national_dex,
            name: name.into(),
            types,
            base_hp,
        };
        definition.validate()?;
        Ok(definition)
    }

    pub fn id(&self) -> &PokemonId {
        &self.id
    }

    pub const fn national_dex(&self) -> u16 {
        self.national_dex
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn types(&self) -> &[String] {
        &self.types
    }

    pub const fn base_hp(&self) -> u16 {
        self.base_hp
    }

    fn validate(&self) -> Result<(), PokemonEditorError> {
        let _ = PokemonId::new(self.id.as_str())?;
        if !(1..=386).contains(&self.national_dex) {
            return Err(PokemonEditorError::InvalidNationalDex(self.national_dex));
        }
        if self.name.trim().is_empty() || self.name.len() > 64 {
            return Err(PokemonEditorError::InvalidName);
        }
        if !(1..=2).contains(&self.types.len())
            || self.types.iter().any(|value| value.trim().is_empty())
        {
            return Err(PokemonEditorError::InvalidTypes);
        }
        if !(1..=255).contains(&self.base_hp) {
            return Err(PokemonEditorError::InvalidBaseHp(self.base_hp));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PokemonCatalog {
    format_version: String,
    pokemon: Vec<PokemonDefinition>,
}

impl PokemonCatalog {
    pub fn new(mut pokemon: Vec<PokemonDefinition>) -> Result<Self, PokemonEditorError> {
        if pokemon.is_empty() {
            return Err(PokemonEditorError::EmptyCatalog);
        }
        let mut ids = BTreeSet::new();
        let mut dex = BTreeSet::new();
        for definition in &pokemon {
            definition.validate()?;
            if !ids.insert(definition.id.clone()) {
                return Err(PokemonEditorError::DuplicateId(definition.id.clone()));
            }
            if !dex.insert(definition.national_dex) {
                return Err(PokemonEditorError::DuplicateNationalDex(
                    definition.national_dex,
                ));
            }
        }
        pokemon.sort_by_key(|definition| definition.national_dex);
        Ok(Self {
            format_version: POKEMON_CATALOG_FORMAT.to_owned(),
            pokemon,
        })
    }

    pub fn standard() -> Result<Self, PokemonEditorError> {
        Self::new(vec![PokemonDefinition::new(
            PokemonId::new("zigzagoon")?,
            263,
            "Zigzagoon",
            vec![String::from("Normal")],
            38,
        )?])
    }

    pub fn from_json(json: &str) -> Result<Self, PokemonEditorError> {
        let catalog = serde_json::from_str::<Self>(json)
            .map_err(|error| PokemonEditorError::Json(error.to_string()))?;
        if catalog.format_version != POKEMON_CATALOG_FORMAT {
            return Err(PokemonEditorError::UnsupportedFormat(
                catalog.format_version,
            ));
        }
        Self::new(catalog.pokemon)
    }

    pub fn to_json_pretty(&self) -> Result<String, PokemonEditorError> {
        serde_json::to_string_pretty(self)
            .map_err(|error| PokemonEditorError::Json(error.to_string()))
    }

    pub fn pokemon(&self) -> &[PokemonDefinition] {
        &self.pokemon
    }

    pub fn pokemon_by_id(&self, id: &PokemonId) -> Option<&PokemonDefinition> {
        self.pokemon.iter().find(|definition| definition.id() == id)
    }

    pub fn transition(&self, command: PokemonEditCommand) -> Result<Self, PokemonEditorError> {
        let mut next = self.clone();
        let id = command.id().clone();
        let Some(index) = next
            .pokemon
            .iter()
            .position(|definition| definition.id() == &id)
        else {
            return Err(PokemonEditorError::UnknownPokemon(id));
        };
        let Some(current) = next.pokemon.get(index).cloned() else {
            return Err(PokemonEditorError::UnknownPokemon(id));
        };
        let replacement = match command {
            PokemonEditCommand::SetName { name, .. } => PokemonDefinition::new(
                current.id,
                current.national_dex,
                name,
                current.types,
                current.base_hp,
            )?,
            PokemonEditCommand::SetTypes { types, .. } => PokemonDefinition::new(
                current.id,
                current.national_dex,
                current.name,
                types,
                current.base_hp,
            )?,
            PokemonEditCommand::SetBaseHp { base_hp, .. } => PokemonDefinition::new(
                current.id,
                current.national_dex,
                current.name,
                current.types,
                base_hp,
            )?,
        };
        let Some(target) = next.pokemon.get_mut(index) else {
            return Err(PokemonEditorError::UnknownPokemon(id));
        };
        *target = replacement;
        Self::new(next.pokemon)
    }

    pub fn validate(&self) -> Result<(), PokemonEditorError> {
        if self.format_version != POKEMON_CATALOG_FORMAT {
            return Err(PokemonEditorError::UnsupportedFormat(
                self.format_version.clone(),
            ));
        }
        let _ = Self::new(self.pokemon.clone())?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PokemonEditCommand {
    SetName {
        pokemon: PokemonId,
        name: String,
    },
    SetTypes {
        pokemon: PokemonId,
        types: Vec<String>,
    },
    SetBaseHp {
        pokemon: PokemonId,
        base_hp: u16,
    },
}

impl PokemonEditCommand {
    fn id(&self) -> &PokemonId {
        match self {
            Self::SetName { pokemon, .. }
            | Self::SetTypes { pokemon, .. }
            | Self::SetBaseHp { pokemon, .. } => pokemon,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PokemonEditorCommand {
    Inspect,
    Validate,
    Edit(PokemonEditCommand),
    Save,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PokemonEditorState {
    catalog: PokemonCatalog,
    dirty: bool,
}

impl PokemonEditorState {
    pub fn catalog(&self) -> &PokemonCatalog {
        &self.catalog
    }

    pub const fn dirty(&self) -> bool {
        self.dirty
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum PokemonEditorResult {
    State(PokemonEditorState),
    Diagnostics(Vec<EditorDiagnostic>),
    SaveRequested,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PokemonEditorModel {
    catalog: PokemonCatalog,
    dirty: bool,
}

impl PokemonEditorModel {
    pub fn new(catalog: PokemonCatalog) -> Result<Self, PokemonEditorError> {
        catalog.validate()?;
        Ok(Self {
            catalog,
            dirty: false,
        })
    }

    pub fn execute(
        &self,
        command: PokemonEditorCommand,
    ) -> Result<(Self, PokemonEditorResult), PokemonEditorError> {
        match command {
            PokemonEditorCommand::Inspect => {
                Ok((self.clone(), PokemonEditorResult::State(self.inspect())))
            }
            PokemonEditorCommand::Validate => Ok((
                self.clone(),
                PokemonEditorResult::Diagnostics(self.validate()),
            )),
            PokemonEditorCommand::Edit(command) => {
                let model = self.clone().transition(command)?;
                Ok((model.clone(), PokemonEditorResult::State(model.inspect())))
            }
            PokemonEditorCommand::Save => Ok((self.clone(), PokemonEditorResult::SaveRequested)),
        }
    }

    pub fn saved(mut self) -> Self {
        self.dirty = false;
        self
    }

    pub fn catalog(&self) -> &PokemonCatalog {
        &self.catalog
    }
}

impl EditorCore for PokemonEditorModel {
    type Command = PokemonEditCommand;
    type Snapshot = PokemonEditorState;
    type Error = PokemonEditorError;

    fn inspect(&self) -> Self::Snapshot {
        PokemonEditorState {
            catalog: self.catalog.clone(),
            dirty: self.dirty,
        }
    }

    fn validate(&self) -> Vec<EditorDiagnostic> {
        match self.catalog.validate() {
            Ok(()) => Vec::new(),
            Err(error) => vec![EditorDiagnostic::new("pokemon.invalid", error.to_string())],
        }
    }

    fn transition(mut self, command: Self::Command) -> Result<Self, Self::Error> {
        self.catalog = self.catalog.transition(command)?;
        self.dirty = true;
        Ok(self)
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PokemonEditorError {
    Json(String),
    InvalidId(String),
    EmptyCatalog,
    DuplicateId(PokemonId),
    DuplicateNationalDex(u16),
    UnknownPokemon(PokemonId),
    UnsupportedFormat(String),
    InvalidNationalDex(u16),
    InvalidName,
    InvalidTypes,
    InvalidBaseHp(u16),
}

impl fmt::Display for PokemonEditorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "Pokemon catalog JSON error: {error}"),
            Self::InvalidId(id) => write!(formatter, "invalid Pokemon ID: {id}"),
            Self::EmptyCatalog => formatter.write_str("Pokemon catalog must not be empty"),
            Self::DuplicateId(id) => write!(formatter, "duplicate Pokemon ID: {}", id.as_str()),
            Self::DuplicateNationalDex(number) => {
                write!(formatter, "duplicate national dex number: {number}")
            }
            Self::UnknownPokemon(id) => write!(formatter, "unknown Pokemon: {}", id.as_str()),
            Self::UnsupportedFormat(format) => {
                write!(formatter, "unsupported Pokemon catalog format: {format}")
            }
            Self::InvalidNationalDex(number) => {
                write!(formatter, "invalid national dex number: {number}")
            }
            Self::InvalidName => {
                formatter.write_str("Pokemon name must be non-empty and at most 64 bytes")
            }
            Self::InvalidTypes => {
                formatter.write_str("Pokemon requires one or two non-empty types")
            }
            Self::InvalidBaseHp(value) => write!(formatter, "invalid Pokemon base HP: {value}"),
        }
    }
}

impl Error for PokemonEditorError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_are_validated_and_leave_the_prior_catalog_unchanged_on_error()
    -> Result<(), PokemonEditorError> {
        let catalog = PokemonCatalog::standard()?;
        let pokemon = PokemonId::new("zigzagoon")?;
        let edited = catalog.transition(PokemonEditCommand::SetBaseHp {
            pokemon: pokemon.clone(),
            base_hp: 40,
        })?;
        assert_eq!(
            edited
                .pokemon_by_id(&pokemon)
                .map(PokemonDefinition::base_hp),
            Some(40)
        );
        assert_eq!(
            catalog
                .pokemon_by_id(&pokemon)
                .map(PokemonDefinition::base_hp),
            Some(38)
        );
        assert!(
            catalog
                .transition(PokemonEditCommand::SetBaseHp {
                    pokemon,
                    base_hp: 0
                })
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn catalog_json_rejects_invalid_pokemon_ids() {
        let result = PokemonCatalog::from_json(
            r#"{
                "format_version": "pokemon-v1",
                "pokemon": [{
                    "id": "../zigzagoon",
                    "national_dex": 263,
                    "name": "Zigzagoon",
                    "types": ["Normal"],
                    "base_hp": 38
                }]
            }"#,
        );
        assert!(matches!(result, Err(PokemonEditorError::Json(_))));
    }
}
