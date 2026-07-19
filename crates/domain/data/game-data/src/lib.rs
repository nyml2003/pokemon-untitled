//! Immutable, offline game data loaded from a generated PokeAPI snapshot.

#![forbid(unsafe_code)]

use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

macro_rules! id_type {
    ($name:ident, $inner:ty) => {
        #[derive(
            Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub $inner);
    };
}

id_type!(PokemonFormId, u32);
id_type!(SpeciesId, u32);
id_type!(MoveId, u32);
id_type!(TypeId, u16);
id_type!(AbilityId, u16);

const POKEDEX_MAGIC: &[u8; 4] = b"PKDX";
const POKEDEX_VERSION: u16 = 1;
pub const GEN3_FIRST_DEX: u16 = 1;
pub const GEN3_LAST_DEX: u16 = 386;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PokedexType {
    pub id: TypeId,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PokedexEntry {
    pub national_dex: u16,
    pub form_id: PokemonFormId,
    pub localized_name: String,
    pub english_name: String,
    pub types: Vec<PokedexType>,
    pub base_stats: BaseStats,
    pub front_asset: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PokedexData {
    entries: Vec<PokedexEntry>,
}

impl PokedexData {
    pub fn embedded_gen3() -> Result<Self, PokedexLoadError> {
        Self::from_bytes(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../assets/source/data/game/pokedex/gen3.v1.bin"
        )))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PokedexLoadError> {
        let mut reader = PokedexReader::new(bytes);
        if reader.take(4)? != POKEDEX_MAGIC {
            return Err(PokedexLoadError::InvalidMagic);
        }
        if reader.u16()? != POKEDEX_VERSION {
            return Err(PokedexLoadError::UnsupportedVersion);
        }
        let count = usize::from(reader.u16()?);
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let national_dex = reader.u16()?;
            let form_id = PokemonFormId(reader.u32()?);
            let base_stats = BaseStats {
                hp: reader.u16()?,
                attack: reader.u16()?,
                defense: reader.u16()?,
                special_attack: reader.u16()?,
                special_defense: reader.u16()?,
                speed: reader.u16()?,
            };
            let type_count = usize::from(reader.u8()?);
            if !(1..=2).contains(&type_count) {
                return Err(PokedexLoadError::InvalidTypeCount(type_count));
            }
            let mut types = Vec::with_capacity(type_count);
            for _ in 0..type_count {
                types.push(PokedexType {
                    id: TypeId(reader.u16()?),
                    name: reader.text()?,
                });
            }
            entries.push(PokedexEntry {
                national_dex,
                form_id,
                localized_name: reader.text()?,
                english_name: reader.text()?,
                types,
                base_stats,
                front_asset: reader.text()?,
            });
        }
        if !reader.is_finished() {
            return Err(PokedexLoadError::TrailingBytes);
        }
        if entries.len() != usize::from(GEN3_LAST_DEX - GEN3_FIRST_DEX + 1)
            || entries
                .iter()
                .zip(GEN3_FIRST_DEX..=GEN3_LAST_DEX)
                .any(|(entry, expected_dex)| {
                    entry.national_dex != expected_dex
                        || entry.front_asset
                            != format!("pokemon/{:04}/form/00/normal/front/00", entry.national_dex)
                })
        {
            return Err(PokedexLoadError::InvalidEntries);
        }
        Ok(Self { entries })
    }

    pub fn entries(&self) -> &[PokedexEntry] {
        &self.entries
    }
}

struct PokedexReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> PokedexReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], PokedexLoadError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(PokedexLoadError::Truncated)?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or(PokedexLoadError::Truncated)?;
        self.offset = end;
        Ok(bytes)
    }

    fn u8(&mut self) -> Result<u8, PokedexLoadError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, PokedexLoadError> {
        let bytes = self
            .take(2)?
            .try_into()
            .map_err(|_| PokedexLoadError::Truncated)?;
        Ok(u16::from_le_bytes(bytes))
    }

    fn u32(&mut self) -> Result<u32, PokedexLoadError> {
        let bytes = self
            .take(4)?
            .try_into()
            .map_err(|_| PokedexLoadError::Truncated)?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn text(&mut self) -> Result<String, PokedexLoadError> {
        let length = usize::from(self.u8()?);
        let bytes = self.take(length)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| PokedexLoadError::InvalidText)
    }

    const fn is_finished(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PokedexLoadError {
    Truncated,
    InvalidMagic,
    UnsupportedVersion,
    InvalidText,
    InvalidTypeCount(usize),
    TrailingBytes,
    InvalidEntries,
}

impl fmt::Display for PokedexLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid Pokedex binary: {self:?}")
    }
}

impl Error for PokedexLoadError {}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DataSetMetadata {
    pub schema_version: String,
    pub source_repository: String,
    pub source_commit: String,
    pub generator_version: String,
    pub locale: String,
    pub version_group: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BaseStats {
    pub hp: u16,
    pub attack: u16,
    pub defense: u16,
    pub special_attack: u16,
    pub special_defense: u16,
    pub speed: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LocalizedName {
    pub localized: String,
    pub english: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PokemonRecord {
    pub id: PokemonFormId,
    pub species_id: SpeciesId,
    pub identifier: String,
    pub is_default: bool,
    pub base_stats: BaseStats,
    pub types: Vec<TypeId>,
    pub abilities: Vec<PokemonAbility>,
    pub display_name: LocalizedName,
    pub learnset: Vec<LearnsetEntry>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PokemonAbility {
    pub ability_id: AbilityId,
    pub is_hidden: bool,
    pub slot: u8,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AbilityRecord {
    pub id: AbilityId,
    pub identifier: String,
    pub display_name: LocalizedName,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MoveLearnMethod {
    LevelUp,
    Egg,
    Tutor,
    Machine,
    Other(String),
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct LearnsetEntry {
    pub move_id: MoveId,
    pub method: MoveLearnMethod,
    pub level: Option<u8>,
    pub order: Option<u16>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TypeRecord {
    pub id: TypeId,
    pub identifier: String,
    pub display_name: LocalizedName,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DamageClass {
    Physical,
    Special,
    Status,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MoveRecord {
    pub id: MoveId,
    pub identifier: String,
    pub display_name: LocalizedName,
    pub move_type: TypeId,
    pub power: Option<u16>,
    pub accuracy: Option<u8>,
    pub pp: Option<u8>,
    pub priority: i8,
    pub damage_class: DamageClass,
    pub effect_id: Option<u16>,
    pub effect_chance: Option<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CurrentDataSet {
    metadata: DataSetMetadata,
    pokemon: Vec<PokemonRecord>,
    moves: Vec<MoveRecord>,
    abilities: Vec<AbilityRecord>,
    types: Vec<TypeRecord>,
}

impl CurrentDataSet {
    pub fn new(
        metadata: DataSetMetadata,
        pokemon: Vec<PokemonRecord>,
        moves: Vec<MoveRecord>,
        abilities: Vec<AbilityRecord>,
        types: Vec<TypeRecord>,
    ) -> Result<Self, DataLoadError> {
        let data = Self {
            metadata,
            pokemon,
            moves,
            abilities,
            types,
        };
        data.validate()?;
        Ok(data)
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, DataLoadError> {
        let data: Self = serde_json::from_slice(bytes)
            .map_err(|error| DataLoadError::MalformedData(error.to_string()))?;
        data.validate()?;
        Ok(data)
    }

    pub fn embedded() -> Result<Self, DataLoadError> {
        Self::from_json(include_bytes!(
            "../../../../../assets/source/data/game/current-dataset/v2.json"
        ))
    }

    pub fn metadata(&self) -> &DataSetMetadata {
        &self.metadata
    }

    pub fn pokemon(&self, id: PokemonFormId) -> Option<&PokemonRecord> {
        self.pokemon
            .binary_search_by_key(&id, |record| record.id)
            .ok()
            .map(|index| &self.pokemon[index])
    }

    pub fn move_by_id(&self, id: MoveId) -> Option<&MoveRecord> {
        self.moves
            .binary_search_by_key(&id, |record| record.id)
            .ok()
            .map(|index| &self.moves[index])
    }

    pub fn type_by_id(&self, id: TypeId) -> Option<&TypeRecord> {
        self.types
            .binary_search_by_key(&id, |record| record.id)
            .ok()
            .map(|index| &self.types[index])
    }

    pub fn ability_by_id(&self, id: AbilityId) -> Option<&AbilityRecord> {
        self.abilities
            .binary_search_by_key(&id, |record| record.id)
            .ok()
            .map(|index| &self.abilities[index])
    }

    pub fn learnset(&self, id: PokemonFormId) -> Option<&[LearnsetEntry]> {
        self.pokemon(id).map(|record| record.learnset.as_slice())
    }

    pub fn can_learn(&self, pokemon: PokemonFormId, battle_move: MoveId) -> bool {
        self.learnset(pokemon)
            .is_some_and(|entries| entries.iter().any(|entry| entry.move_id == battle_move))
    }

    pub fn can_learn_at_level(
        &self,
        pokemon: PokemonFormId,
        battle_move: MoveId,
        level: u8,
    ) -> bool {
        self.learnset(pokemon).is_some_and(|entries| {
            entries.iter().any(|entry| {
                entry.move_id == battle_move
                    && match entry.method {
                        MoveLearnMethod::LevelUp => {
                            entry.level.is_none_or(|required| required <= level)
                        }
                        _ => true,
                    }
            })
        })
    }

    pub fn pokemon_iter(&self) -> impl Iterator<Item = &PokemonRecord> {
        self.pokemon.iter()
    }

    pub fn move_iter(&self) -> impl Iterator<Item = &MoveRecord> {
        self.moves.iter()
    }

    pub fn type_iter(&self) -> impl Iterator<Item = &TypeRecord> {
        self.types.iter()
    }

    pub fn ability_iter(&self) -> impl Iterator<Item = &AbilityRecord> {
        self.abilities.iter()
    }

    fn validate(&self) -> Result<(), DataLoadError> {
        if self.metadata.schema_version != "current-data-set-v4" {
            return Err(DataLoadError::UnsupportedSchema(
                self.metadata.schema_version.clone(),
            ));
        }
        if self.metadata.version_group.trim().is_empty() {
            return Err(DataLoadError::InvalidRecord(
                "metadata version group is empty".into(),
            ));
        }
        validate_sorted("pokemon", self.pokemon.iter().map(|record| record.id.0))?;
        validate_sorted("moves", self.moves.iter().map(|record| record.id.0))?;
        validate_sorted(
            "abilities",
            self.abilities.iter().map(|record| record.id.0 as u32),
        )?;
        validate_sorted("types", self.types.iter().map(|record| record.id.0 as u32))?;
        for pokemon in &self.pokemon {
            if pokemon.types.is_empty() || pokemon.types.len() > 2 {
                return Err(DataLoadError::InvalidRecord(format!(
                    "pokemon {} has {} types",
                    pokemon.id.0,
                    pokemon.types.len()
                )));
            }
            if pokemon.types.len() == 2 && pokemon.types[0] == pokemon.types[1] {
                return Err(DataLoadError::InvalidRecord(format!(
                    "pokemon {} repeats a type",
                    pokemon.id.0
                )));
            }
            if pokemon
                .types
                .iter()
                .any(|id| self.type_by_id(*id).is_none())
            {
                return Err(DataLoadError::InvalidRecord(format!(
                    "pokemon {} references an unknown type",
                    pokemon.id.0
                )));
            }
            let mut previous_ability_slot = None;
            for ability in &pokemon.abilities {
                if self.ability_by_id(ability.ability_id).is_none() {
                    return Err(DataLoadError::InvalidRecord(format!(
                        "pokemon {} references an unknown ability",
                        pokemon.id.0
                    )));
                }
                if previous_ability_slot.is_some_and(|previous| ability.slot <= previous) {
                    return Err(DataLoadError::InvalidRecord(format!(
                        "pokemon {} abilities are not strictly sorted by slot",
                        pokemon.id.0
                    )));
                }
                previous_ability_slot = Some(ability.slot);
            }
            let mut previous = None;
            for entry in &pokemon.learnset {
                if self.move_by_id(entry.move_id).is_none() {
                    return Err(DataLoadError::InvalidRecord(format!(
                        "pokemon {} learnset references unknown move {}",
                        pokemon.id.0, entry.move_id.0
                    )));
                }
                if previous.as_ref().is_some_and(|previous| entry <= previous) {
                    return Err(DataLoadError::InvalidRecord(format!(
                        "pokemon {} learnset is not strictly sorted",
                        pokemon.id.0
                    )));
                }
                previous = Some(entry.clone());
            }
        }
        for move_record in &self.moves {
            if self.type_by_id(move_record.move_type).is_none() {
                return Err(DataLoadError::InvalidRecord(format!(
                    "move {} references an unknown type",
                    move_record.id.0
                )));
            }
            if move_record
                .accuracy
                .is_some_and(|accuracy| !(1..=100).contains(&accuracy))
            {
                return Err(DataLoadError::InvalidRecord(format!(
                    "move {} has invalid accuracy",
                    move_record.id.0
                )));
            }
            if move_record
                .effect_chance
                .is_some_and(|chance| !(1..=100).contains(&chance))
            {
                return Err(DataLoadError::InvalidRecord(format!(
                    "move {} has invalid effect chance",
                    move_record.id.0
                )));
            }
        }
        Ok(())
    }
}

fn validate_sorted<T: Ord + Copy>(
    kind: &str,
    ids: impl Iterator<Item = T>,
) -> Result<(), DataLoadError> {
    let mut previous = None;
    for id in ids {
        if previous.is_some_and(|previous| id <= previous) {
            return Err(DataLoadError::InvalidRecord(format!(
                "{kind} IDs are not strictly sorted"
            )));
        }
        previous = Some(id);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataLoadError {
    MalformedData(String),
    UnsupportedSchema(String),
    InvalidRecord(String),
}

impl fmt::Display for DataLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedData(error) => write!(formatter, "malformed game data: {error}"),
            Self::UnsupportedSchema(version) => {
                write!(formatter, "unsupported game data schema: {version}")
            }
            Self::InvalidRecord(error) => write!(formatter, "invalid game data record: {error}"),
        }
    }
}

impl Error for DataLoadError {}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
