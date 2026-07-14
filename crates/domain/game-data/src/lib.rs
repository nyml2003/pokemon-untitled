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
            "/../../../assets/source/data/game/pokedex/gen3.v1.bin"
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
            || entries.iter().enumerate().any(|(index, entry)| {
                entry.national_dex != GEN3_FIRST_DEX + u16::try_from(index).unwrap()
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
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().expect("two bytes"),
        ))
    }

    fn u32(&mut self) -> Result<u32, PokedexLoadError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("four bytes"),
        ))
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
    pub display_name: LocalizedName,
    pub learnset: Vec<LearnsetEntry>,
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
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CurrentDataSet {
    metadata: DataSetMetadata,
    pokemon: Vec<PokemonRecord>,
    moves: Vec<MoveRecord>,
    types: Vec<TypeRecord>,
}

impl CurrentDataSet {
    pub fn new(
        metadata: DataSetMetadata,
        pokemon: Vec<PokemonRecord>,
        moves: Vec<MoveRecord>,
        types: Vec<TypeRecord>,
    ) -> Result<Self, DataLoadError> {
        let data = Self {
            metadata,
            pokemon,
            moves,
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
            "../../../../assets/source/data/game/current-dataset/v2.json"
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

    fn validate(&self) -> Result<(), DataLoadError> {
        if self.metadata.schema_version != "current-data-set-v2" {
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
mod tests {
    use super::{
        CurrentDataSet, DamageClass, DataLoadError, GEN3_FIRST_DEX, GEN3_LAST_DEX, MoveId,
        PokedexData, PokemonFormId, TypeId,
    };

    #[test]
    fn embedded_pokedex_covers_the_canonical_gen3_fronts() {
        let pokedex = PokedexData::embedded_gen3().unwrap();
        assert_eq!(pokedex.entries().len(), 386);
        assert_eq!(pokedex.entries()[0].national_dex, GEN3_FIRST_DEX);
        assert_eq!(
            pokedex.entries().last().unwrap().national_dex,
            GEN3_LAST_DEX
        );
        for entry in pokedex.entries() {
            assert_eq!(
                entry.front_asset,
                format!("pokemon/{:04}/form/00/normal/front/00", entry.national_dex)
            );
            assert!(!entry.localized_name.is_empty());
            assert!(!entry.types.is_empty());
        }
    }

    fn fixture() -> Vec<u8> {
        serde_json::to_vec(&CurrentDataSet {
            metadata: super::DataSetMetadata {
                schema_version: "current-data-set-v2".into(),
                source_repository: "test".into(),
                source_commit: "test".into(),
                generator_version: "test".into(),
                locale: "zh-Hans".into(),
                version_group: "emerald".into(),
            },
            pokemon: vec![super::PokemonRecord {
                id: PokemonFormId(1),
                species_id: super::SpeciesId(1),
                identifier: "bulbasaur".into(),
                is_default: true,
                base_stats: super::BaseStats {
                    hp: 45,
                    attack: 49,
                    defense: 49,
                    special_attack: 65,
                    special_defense: 65,
                    speed: 45,
                },
                types: vec![TypeId(12), TypeId(4)],
                display_name: super::LocalizedName {
                    localized: "妙蛙种子".into(),
                    english: "Bulbasaur".into(),
                },
                learnset: vec![super::LearnsetEntry {
                    move_id: MoveId(1),
                    method: super::MoveLearnMethod::LevelUp,
                    level: Some(1),
                    order: Some(1),
                }],
            }],
            moves: vec![super::MoveRecord {
                id: MoveId(1),
                identifier: "pound".into(),
                display_name: super::LocalizedName {
                    localized: "拍击".into(),
                    english: "Pound".into(),
                },
                move_type: TypeId(1),
                power: Some(40),
                accuracy: Some(100),
                pp: Some(35),
                priority: 0,
                damage_class: DamageClass::Physical,
            }],
            types: vec![
                super::TypeRecord {
                    id: TypeId(1),
                    identifier: "normal".into(),
                    display_name: super::LocalizedName {
                        localized: "一般".into(),
                        english: "Normal".into(),
                    },
                },
                super::TypeRecord {
                    id: TypeId(4),
                    identifier: "poison".into(),
                    display_name: super::LocalizedName {
                        localized: "毒".into(),
                        english: "Poison".into(),
                    },
                },
                super::TypeRecord {
                    id: TypeId(12),
                    identifier: "grass".into(),
                    display_name: super::LocalizedName {
                        localized: "草".into(),
                        english: "Grass".into(),
                    },
                },
            ],
        })
        .unwrap()
    }

    fn invalid(mut change: impl FnMut(&mut serde_json::Value)) -> DataLoadError {
        let mut value: serde_json::Value = serde_json::from_slice(&fixture()).unwrap();
        change(&mut value);
        CurrentDataSet::from_json(&serde_json::to_vec(&value).unwrap()).unwrap_err()
    }

    #[test]
    fn loads_and_queries_sorted_records() {
        let data = CurrentDataSet::from_json(&fixture()).unwrap();
        assert_eq!(
            data.pokemon(PokemonFormId(1))
                .unwrap()
                .display_name
                .localized,
            "妙蛙种子"
        );
        assert_eq!(data.move_by_id(MoveId(1)).unwrap().power, Some(40));
        assert!(data.can_learn(PokemonFormId(1), MoveId(1)));
        assert!(data.can_learn_at_level(PokemonFormId(1), MoveId(1), 1));
        assert!(!data.can_learn_at_level(PokemonFormId(1), MoveId(1), 0));
    }

    #[test]
    fn rejects_unknown_schema() {
        let mut bytes = fixture();
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        value["metadata"]["schema_version"] = serde_json::Value::String("v9".into());
        bytes = serde_json::to_vec(&value).unwrap();
        assert!(matches!(
            CurrentDataSet::from_json(&bytes),
            Err(DataLoadError::UnsupportedSchema(_))
        ));
    }

    #[test]
    fn rejects_every_cross_record_and_metadata_violation() {
        let errors = [
            invalid(|value| {
                let pokemon = value["pokemon"][0].clone();
                value["pokemon"] = serde_json::json!([pokemon.clone(), pokemon]);
            }),
            invalid(|value| value["metadata"]["version_group"] = "".into()),
            invalid(|value| value["pokemon"][0]["types"] = serde_json::json!([])),
            invalid(|value| value["pokemon"][0]["types"] = serde_json::json!([1, 4, 12])),
            invalid(|value| value["pokemon"][0]["types"] = serde_json::json!([4, 4])),
            invalid(|value| value["pokemon"][0]["types"] = serde_json::json!([999])),
            invalid(|value| value["pokemon"][0]["learnset"][0]["move_id"] = 999.into()),
            invalid(|value| {
                let entry = value["pokemon"][0]["learnset"][0].clone();
                value["pokemon"][0]["learnset"] = serde_json::json!([entry.clone(), entry]);
            }),
            invalid(|value| value["moves"][0]["move_type"] = 999.into()),
            invalid(|value| value["moves"][0]["accuracy"] = 0.into()),
        ];
        for error in errors {
            assert!(matches!(error, DataLoadError::InvalidRecord(_)));
            assert!(!error.to_string().is_empty());
        }

        for error in [
            DataLoadError::MalformedData("bad json".into()),
            DataLoadError::UnsupportedSchema("v9".into()),
        ] {
            assert!(!error.to_string().is_empty());
        }
    }

    #[test]
    fn embedded_data_matches_the_pinned_snapshot() {
        let data = CurrentDataSet::embedded().unwrap();
        assert_eq!(
            data.metadata().source_commit,
            "d638fe7791214a8d3c3282e2a3113eea7cfef288"
        );
        assert_eq!(data.metadata().version_group, "emerald");
        assert_eq!(data.pokemon_iter().count(), 1_351);
        assert_eq!(data.move_iter().count(), 937);
        assert_eq!(data.type_iter().count(), 21);
        assert!(data.can_learn(PokemonFormId(1), MoveId(33)));
        assert!(data.can_learn(PokemonFormId(1), MoveId(22)));
        assert!(!data.can_learn_at_level(PokemonFormId(1), MoveId(22), 9));
        assert!(data.can_learn_at_level(PokemonFormId(1), MoveId(22), 10));
    }
}
