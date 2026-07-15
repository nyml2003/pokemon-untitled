//! Pure PokeAPI CSV parsing and validation.

#![forbid(unsafe_code)]

use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt,
    path::PathBuf,
};

use game_data::{
    AbilityId, AbilityRecord, BaseStats, CurrentDataSet, DamageClass, DataSetMetadata,
    LearnsetEntry, LocalizedName, MoveId, MoveLearnMethod, MoveRecord, PokemonAbility,
    PokemonFormId, PokemonRecord, SpeciesId, TypeId, TypeRecord,
};
use serde::Deserialize;

pub const PINNED_COMMIT: &str = "d638fe7791214a8d3c3282e2a3113eea7cfef288";
const SOURCE_REPOSITORY: &str = "https://github.com/PokeAPI/pokeapi";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportOptions {
    pub locale: String,
    pub source_commit: String,
    pub version_group: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliOptions {
    pub source: PathBuf,
    pub output: PathBuf,
    pub import: ImportOptions,
}

pub fn parse_cli_args(
    args: impl IntoIterator<Item = String>,
) -> Result<Option<CliOptions>, String> {
    let mut source = None;
    let mut output = None;
    let mut version_group = None;
    let mut locale = "zh-Hans".to_owned();
    let mut source_commit = PINNED_COMMIT.to_owned();
    let mut args = args.into_iter();
    while let Some(flag) = args.next() {
        if matches!(flag.as_str(), "--help" | "-h") {
            return Ok(None);
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--source" => source = Some(PathBuf::from(value)),
            "--output" => output = Some(PathBuf::from(value)),
            "--version-group" => version_group = Some(value),
            "--locale" => locale = value,
            "--source-commit" => source_commit = value,
            _ => return Err(format!("unknown argument: {flag}")),
        }
    }
    Ok(Some(CliOptions {
        source: source.ok_or_else(|| "--source is required".to_owned())?,
        output: output.ok_or_else(|| "--output is required".to_owned())?,
        import: ImportOptions {
            locale,
            source_commit,
            version_group: version_group.ok_or_else(|| "--version-group is required".to_owned())?,
        },
    }))
}

impl ImportOptions {
    pub fn emerald() -> Self {
        Self {
            locale: "zh-Hans".into(),
            source_commit: PINNED_COMMIT.into(),
            version_group: "emerald".into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportSource {
    root: PathBuf,
    files: HashMap<String, String>,
}

impl ImportSource {
    pub fn new(root: PathBuf, files: impl IntoIterator<Item = (String, String)>) -> Self {
        Self {
            root,
            files: files.into_iter().collect(),
        }
    }

    fn contents(&self, name: &str) -> Option<&str> {
        self.files.get(name).map(String::as_str)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImportDiagnosticCode {
    MissingInputFile,
    InvalidHeader,
    InvalidField,
    DuplicateId,
    MissingReference,
    MissingStat,
    InvalidTypeSlots,
    MetadataMismatch,
    OutputValidationFailed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportDiagnostic {
    pub code: ImportDiagnosticCode,
    pub file: Option<PathBuf>,
    pub row: Option<usize>,
    pub field: Option<String>,
    pub entity_id: Option<u32>,
    pub message: String,
}

impl ImportDiagnostic {
    fn new(code: ImportDiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            file: None,
            row: None,
            field: None,
            entity_id: None,
            message: message.into(),
        }
    }

    fn file(mut self, file: impl Into<PathBuf>) -> Self {
        self.file = Some(file.into());
        self
    }

    fn row(mut self, row: usize) -> Self {
        self.row = Some(row);
        self
    }

    fn field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    fn entity(mut self, entity_id: u32) -> Self {
        self.entity_id = Some(entity_id);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportFailure {
    diagnostics: Vec<ImportDiagnostic>,
}

impl ImportFailure {
    pub fn one(diagnostic: ImportDiagnostic) -> Self {
        Self {
            diagnostics: vec![diagnostic],
        }
    }

    pub fn diagnostics(&self) -> &[ImportDiagnostic] {
        &self.diagnostics
    }

    pub fn missing_file(path: PathBuf, message: String) -> Self {
        Self::one(ImportDiagnostic::new(ImportDiagnosticCode::MissingInputFile, message).file(path))
    }

    pub fn output(message: String) -> Self {
        Self::one(ImportDiagnostic::new(
            ImportDiagnosticCode::OutputValidationFailed,
            message,
        ))
    }

    pub fn output_at(path: impl Into<PathBuf>, message: String) -> Self {
        Self::one(
            ImportDiagnostic::new(ImportDiagnosticCode::OutputValidationFailed, message).file(path),
        )
    }
}

impl fmt::Display for ImportFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, diagnostic) in self.diagnostics.iter().enumerate() {
            if index > 0 {
                formatter.write_str("\n")?;
            }
            write!(formatter, "{:?}", diagnostic.code)?;
            if let Some(file) = &diagnostic.file {
                write!(formatter, " {}", file.display())?;
            }
            if let Some(row) = diagnostic.row {
                write!(formatter, ":{row}")?;
            }
            if let Some(field) = &diagnostic.field {
                write!(formatter, " field={field}")?;
            }
            if let Some(entity_id) = diagnostic.entity_id {
                write!(formatter, " entity={entity_id}")?;
            }
            write!(formatter, ": {}", diagnostic.message)?;
        }
        Ok(())
    }
}

impl Error for ImportFailure {}

struct Located<T> {
    row: usize,
    value: T,
}

#[derive(Deserialize)]
struct LanguageRow {
    id: u16,
    identifier: String,
}
#[derive(Deserialize)]
struct StatRow {
    id: u16,
    identifier: String,
}
#[derive(Deserialize)]
struct TypeRow {
    id: u16,
    identifier: String,
}
#[derive(Deserialize)]
struct DamageClassRow {
    id: u8,
    identifier: String,
}
#[derive(Deserialize)]
struct VersionGroupRow {
    id: u16,
    identifier: String,
}
#[derive(Deserialize)]
struct MoveMethodRow {
    id: u8,
    identifier: String,
}
#[derive(Deserialize)]
struct PokemonRow {
    id: u32,
    identifier: String,
    species_id: u32,
    is_default: u8,
}
#[derive(Deserialize)]
struct PokemonStatRow {
    pokemon_id: u32,
    stat_id: u16,
    base_stat: u16,
}
#[derive(Deserialize)]
struct PokemonTypeRow {
    pokemon_id: u32,
    type_id: u16,
    slot: u8,
}
#[derive(Deserialize)]
struct AbilityRow {
    id: u16,
    identifier: String,
    generation_id: u8,
    is_main_series: u8,
}
#[derive(Deserialize)]
struct AbilityNameRow {
    ability_id: u16,
    local_language_id: u16,
    name: String,
}
#[derive(Deserialize)]
struct PokemonAbilityRow {
    pokemon_id: u32,
    ability_id: u16,
    is_hidden: u8,
    slot: u8,
}
#[derive(Deserialize)]
struct SpeciesNameRow {
    pokemon_species_id: u32,
    local_language_id: u16,
    name: String,
}
#[derive(Deserialize)]
struct MoveRow {
    id: u32,
    identifier: String,
    type_id: u16,
    power: Option<u16>,
    pp: Option<u8>,
    accuracy: Option<u8>,
    priority: i8,
    damage_class_id: u8,
    effect_id: Option<u16>,
    effect_chance: Option<u8>,
}
#[derive(Deserialize)]
struct PokemonMoveRow {
    pokemon_id: u32,
    version_group_id: u16,
    move_id: u32,
    pokemon_move_method_id: u8,
    level: u8,
    order: Option<u16>,
}
#[derive(Deserialize)]
struct MoveNameRow {
    move_id: u32,
    local_language_id: u16,
    name: String,
}
#[derive(Deserialize)]
struct TypeNameRow {
    type_id: u16,
    local_language_id: u16,
    name: String,
}

fn fail<T>(diagnostic: ImportDiagnostic) -> Result<T, ImportFailure> {
    Err(ImportFailure::one(diagnostic))
}

fn read_csv<T: for<'de> Deserialize<'de>>(
    source: &ImportSource,
    name: &str,
    required_headers: &[&str],
) -> Result<Vec<Located<T>>, ImportFailure> {
    let path = source.path(name);
    let contents = source.contents(name).ok_or_else(|| {
        ImportFailure::missing_file(path.clone(), "input was not provided".into())
    })?;
    let mut reader = csv::Reader::from_reader(contents.as_bytes());
    let headers = reader
        .headers()
        .expect("a UTF-8 in-memory CSV source cannot fail while reading its header");
    if let Some(missing) = required_headers
        .iter()
        .find(|required| !headers.iter().any(|header| header == **required))
    {
        return fail(
            ImportDiagnostic::new(
                ImportDiagnosticCode::InvalidHeader,
                format!("required column is missing: {missing}"),
            )
            .file(&path)
            .field(*missing),
        );
    }
    reader
        .deserialize()
        .enumerate()
        .map(|(index, record)| {
            record
                .map(|value| Located {
                    row: index + 2,
                    value,
                })
                .map_err(|error| {
                    ImportFailure::one(
                        ImportDiagnostic::new(
                            ImportDiagnosticCode::InvalidField,
                            format!("cannot deserialize CSV row: {error}"),
                        )
                        .file(&path)
                        .row(index + 2),
                    )
                })
        })
        .collect()
}

fn validate_source_metadata(source: &ImportSource, commit: &str) -> Result<(), ImportFailure> {
    let path = source.path("SOURCE.md");
    let contents = source.contents("SOURCE.md").ok_or_else(|| {
        ImportFailure::missing_file(path.clone(), "source metadata was not provided".into())
    })?;
    let expected = format!("- Commit: `{commit}`");
    if !contents.lines().any(|line| line == expected) {
        return fail(
            ImportDiagnostic::new(
                ImportDiagnosticCode::MetadataMismatch,
                format!("SOURCE.md does not declare commit {commit}"),
            )
            .file(path)
            .field("commit"),
        );
    }
    Ok(())
}

fn localized(localized: Option<String>, english: Option<String>, fallback: &str) -> LocalizedName {
    let english = english.unwrap_or_else(|| fallback.to_owned());
    LocalizedName {
        localized: localized.unwrap_or_else(|| english.clone()),
        english,
    }
}

fn names<I>(rows: I, language_id: u16) -> HashMap<u32, (Option<String>, Option<String>)>
where
    I: IntoIterator<Item = (u32, u16, String)>,
{
    let mut result: HashMap<u32, (Option<String>, Option<String>)> = HashMap::new();
    for (id, language, name) in rows {
        let entry = result.entry(id).or_default();
        if language == language_id {
            entry.0 = Some(name.clone());
        }
        if language == 9 {
            entry.1 = Some(name);
        }
    }
    result
}

pub fn import_source(
    source: &ImportSource,
    options: &ImportOptions,
) -> Result<CurrentDataSet, ImportFailure> {
    validate_source_metadata(source, &options.source_commit)?;
    if options.locale != "zh-Hans" {
        return fail(
            ImportDiagnostic::new(
                ImportDiagnosticCode::InvalidField,
                format!("unsupported locale: {}", options.locale),
            )
            .field("locale"),
        );
    }

    let languages = read_csv::<LanguageRow>(source, "languages.csv", &["id", "identifier"])?;
    let language_id = languages
        .into_iter()
        .find(|row| row.value.identifier == "zh-hans")
        .map(|row| row.value.id)
        .ok_or_else(|| {
            ImportFailure::one(
                ImportDiagnostic::new(
                    ImportDiagnosticCode::MissingReference,
                    "zh-hans language is missing",
                )
                .file(source.path("languages.csv"))
                .field("identifier"),
            )
        })?;
    let version_groups =
        read_csv::<VersionGroupRow>(source, "version_groups.csv", &["id", "identifier"])?;
    let version_group_id = version_groups
        .into_iter()
        .find(|row| row.value.identifier == options.version_group)
        .map(|row| row.value.id)
        .ok_or_else(|| {
            ImportFailure::one(
                ImportDiagnostic::new(
                    ImportDiagnosticCode::MissingReference,
                    format!("version group is missing: {}", options.version_group),
                )
                .file(source.path("version_groups.csv"))
                .field("identifier"),
            )
        })?;

    let stat_rows = read_csv::<StatRow>(source, "stats.csv", &["id", "identifier"])?;
    let stat_ids: HashMap<_, _> = stat_rows
        .into_iter()
        .map(|row| (row.value.identifier, row.value.id))
        .collect();
    let required_stats = [
        "hp",
        "attack",
        "defense",
        "special-attack",
        "special-defense",
        "speed",
    ];
    if let Some(missing) = required_stats
        .iter()
        .find(|name| !stat_ids.contains_key(**name))
    {
        return fail(
            ImportDiagnostic::new(
                ImportDiagnosticCode::MissingStat,
                format!("required stat is missing: {missing}"),
            )
            .file(source.path("stats.csv"))
            .field("identifier"),
        );
    }

    let type_rows = read_csv::<TypeRow>(source, "types.csv", &["id", "identifier"])?;
    let type_ids: HashSet<_> = type_rows.iter().map(|row| row.value.id).collect();
    let type_names = names(
        read_csv::<TypeNameRow>(
            source,
            "type_names.csv",
            &["type_id", "local_language_id", "name"],
        )?
        .into_iter()
        .map(|row| {
            (
                u32::from(row.value.type_id),
                row.value.local_language_id,
                row.value.name,
            )
        }),
        language_id,
    );
    let types = type_rows
        .into_iter()
        .map(|row| {
            let (localized_name, english_name) = type_names
                .get(&u32::from(row.value.id))
                .cloned()
                .unwrap_or_default();
            TypeRecord {
                id: TypeId(row.value.id),
                identifier: row.value.identifier.clone(),
                display_name: localized(localized_name, english_name, &row.value.identifier),
            }
        })
        .collect::<Vec<_>>();
    let ability_names = names(
        read_csv::<AbilityNameRow>(
            source,
            "ability_names.csv",
            &["ability_id", "local_language_id", "name"],
        )?
        .into_iter()
        .map(|row| {
            (
                u32::from(row.value.ability_id),
                row.value.local_language_id,
                row.value.name,
            )
        }),
        language_id,
    );
    let abilities = read_csv::<AbilityRow>(
        source,
        "abilities.csv",
        &["id", "identifier", "generation_id", "is_main_series"],
    )?
    .into_iter()
    .filter(|row| row.value.generation_id <= 3 && row.value.is_main_series != 0)
    .map(|row| {
        let (localized_name, english_name) = ability_names
            .get(&u32::from(row.value.id))
            .cloned()
            .unwrap_or_default();
        AbilityRecord {
            id: AbilityId(row.value.id),
            identifier: row.value.identifier.clone(),
            display_name: localized(localized_name, english_name, &row.value.identifier),
        }
    })
    .collect::<Vec<_>>();
    let ability_ids: HashSet<_> = abilities.iter().map(|record| record.id.0).collect();
    let damage_classes: HashMap<_, _> =
        read_csv::<DamageClassRow>(source, "move_damage_classes.csv", &["id", "identifier"])?
            .into_iter()
            .map(|row| (row.value.id, row.value.identifier))
            .collect();
    let move_methods: HashMap<_, _> =
        read_csv::<MoveMethodRow>(source, "pokemon_move_methods.csv", &["id", "identifier"])?
            .into_iter()
            .map(|row| (row.value.id, row.value.identifier))
            .collect();

    let species_names = names(
        read_csv::<SpeciesNameRow>(
            source,
            "pokemon_species_names.csv",
            &["pokemon_species_id", "local_language_id", "name"],
        )?
        .into_iter()
        .map(|row| {
            (
                row.value.pokemon_species_id,
                row.value.local_language_id,
                row.value.name,
            )
        }),
        language_id,
    );
    let move_names = names(
        read_csv::<MoveNameRow>(
            source,
            "move_names.csv",
            &["move_id", "local_language_id", "name"],
        )?
        .into_iter()
        .map(|row| {
            (
                row.value.move_id,
                row.value.local_language_id,
                row.value.name,
            )
        }),
        language_id,
    );

    let moves_path = source.path("moves.csv");
    let moves = read_csv::<MoveRow>(
        source,
        "moves.csv",
        &[
            "id",
            "identifier",
            "type_id",
            "power",
            "pp",
            "accuracy",
            "priority",
            "damage_class_id",
            "effect_id",
            "effect_chance",
        ],
    )?
    .into_iter()
    .map(|located| {
        let row = located.value;
        let (localized_name, english_name) = move_names.get(&row.id).cloned().unwrap_or_default();
        let damage_class = match damage_classes.get(&row.damage_class_id).map(String::as_str) {
            Some("physical") => DamageClass::Physical,
            Some("special") => DamageClass::Special,
            Some("status") => DamageClass::Status,
            Some(other) => {
                return fail(
                    ImportDiagnostic::new(
                        ImportDiagnosticCode::InvalidField,
                        format!("unknown damage class: {other}"),
                    )
                    .file(&moves_path)
                    .row(located.row)
                    .field("damage_class_id")
                    .entity(row.id),
                );
            }
            None => {
                return fail(
                    ImportDiagnostic::new(
                        ImportDiagnosticCode::MissingReference,
                        format!("unknown damage class ID: {}", row.damage_class_id),
                    )
                    .file(&moves_path)
                    .row(located.row)
                    .field("damage_class_id")
                    .entity(row.id),
                );
            }
        };
        if !type_ids.contains(&row.type_id) {
            return fail(
                ImportDiagnostic::new(
                    ImportDiagnosticCode::MissingReference,
                    format!("unknown type ID: {}", row.type_id),
                )
                .file(&moves_path)
                .row(located.row)
                .field("type_id")
                .entity(row.id),
            );
        }
        Ok(MoveRecord {
            id: MoveId(row.id),
            identifier: row.identifier.clone(),
            display_name: localized(localized_name, english_name, &row.identifier),
            move_type: TypeId(row.type_id),
            power: row.power.filter(|value| *value != 0),
            accuracy: row.accuracy.filter(|value| *value != 0),
            pp: row.pp.filter(|value| *value != 0),
            priority: row.priority,
            damage_class,
            effect_id: row.effect_id,
            effect_chance: row.effect_chance.filter(|value| *value != 0),
        })
    })
    .collect::<Result<Vec<_>, ImportFailure>>()?;
    let move_ids: HashSet<_> = moves.iter().map(|record| record.id.0).collect();

    let stats_path = source.path("pokemon_stats.csv");
    let mut stats_by_pokemon: HashMap<u32, [Option<u16>; 6]> = HashMap::new();
    for located in read_csv::<PokemonStatRow>(
        source,
        "pokemon_stats.csv",
        &["pokemon_id", "stat_id", "base_stat"],
    )? {
        let row = located.value;
        let slot = required_stats
            .iter()
            .position(|name| stat_ids.get(*name) == Some(&row.stat_id));
        if let Some(slot) = slot {
            let entry = stats_by_pokemon.entry(row.pokemon_id).or_default();
            if entry[slot].replace(row.base_stat).is_some() {
                return fail(
                    ImportDiagnostic::new(
                        ImportDiagnosticCode::DuplicateId,
                        format!("stat {} is repeated", row.stat_id),
                    )
                    .file(&stats_path)
                    .row(located.row)
                    .field("stat_id")
                    .entity(row.pokemon_id),
                );
            }
        }
    }

    let types_path = source.path("pokemon_types.csv");
    let mut types_by_pokemon: HashMap<u32, Vec<(u8, TypeId)>> = HashMap::new();
    for located in read_csv::<PokemonTypeRow>(
        source,
        "pokemon_types.csv",
        &["pokemon_id", "type_id", "slot"],
    )? {
        let row = located.value;
        if !type_ids.contains(&row.type_id) {
            return fail(
                ImportDiagnostic::new(
                    ImportDiagnosticCode::MissingReference,
                    format!("unknown type ID: {}", row.type_id),
                )
                .file(&types_path)
                .row(located.row)
                .field("type_id")
                .entity(row.pokemon_id),
            );
        }
        if !(1..=2).contains(&row.slot) {
            return fail(
                ImportDiagnostic::new(
                    ImportDiagnosticCode::InvalidTypeSlots,
                    format!("invalid type slot: {}", row.slot),
                )
                .file(&types_path)
                .row(located.row)
                .field("slot")
                .entity(row.pokemon_id),
            );
        }
        let entry = types_by_pokemon.entry(row.pokemon_id).or_default();
        if entry.iter().any(|(slot, _)| *slot == row.slot) {
            return fail(
                ImportDiagnostic::new(
                    ImportDiagnosticCode::InvalidTypeSlots,
                    format!("type slot {} is repeated", row.slot),
                )
                .file(&types_path)
                .row(located.row)
                .field("slot")
                .entity(row.pokemon_id),
            );
        }
        entry.push((row.slot, TypeId(row.type_id)));
    }

    let abilities_path = source.path("pokemon_abilities.csv");
    let mut abilities_by_pokemon: HashMap<u32, Vec<PokemonAbility>> = HashMap::new();
    for located in read_csv::<PokemonAbilityRow>(
        source,
        "pokemon_abilities.csv",
        &["pokemon_id", "ability_id", "is_hidden", "slot"],
    )? {
        let row = located.value;
        if !ability_ids.contains(&row.ability_id) {
            continue;
        }
        let entry = abilities_by_pokemon.entry(row.pokemon_id).or_default();
        if entry.iter().any(|ability| ability.slot == row.slot) {
            return fail(
                ImportDiagnostic::new(
                    ImportDiagnosticCode::DuplicateId,
                    format!("ability slot {} is repeated", row.slot),
                )
                .file(&abilities_path)
                .row(located.row)
                .field("slot")
                .entity(row.pokemon_id),
            );
        }
        entry.push(PokemonAbility {
            ability_id: AbilityId(row.ability_id),
            is_hidden: row.is_hidden != 0,
            slot: row.slot,
        });
    }

    let pokemon_path = source.path("pokemon.csv");
    let mut pokemon = read_csv::<PokemonRow>(
        source,
        "pokemon.csv",
        &["id", "identifier", "species_id", "is_default"],
    )?
    .into_iter()
    .map(|located| {
        let row = located.value;
        let stats = stats_by_pokemon.remove(&row.id).ok_or_else(|| {
            ImportFailure::one(
                ImportDiagnostic::new(ImportDiagnosticCode::MissingStat, "pokemon has no stats")
                    .file(&pokemon_path)
                    .row(located.row)
                    .entity(row.id),
            )
        })?;
        let [
            Some(hp),
            Some(attack),
            Some(defense),
            Some(special_attack),
            Some(special_defense),
            Some(speed),
        ] = stats
        else {
            return fail(
                ImportDiagnostic::new(
                    ImportDiagnosticCode::MissingStat,
                    "pokemon does not have all six battle stats",
                )
                .file(&pokemon_path)
                .row(located.row)
                .entity(row.id),
            );
        };
        let mut pokemon_types = types_by_pokemon.remove(&row.id).ok_or_else(|| {
            ImportFailure::one(
                ImportDiagnostic::new(
                    ImportDiagnosticCode::InvalidTypeSlots,
                    "pokemon has no types",
                )
                .file(&pokemon_path)
                .row(located.row)
                .entity(row.id),
            )
        })?;
        pokemon_types.sort_by_key(|(slot, _)| *slot);
        let mut pokemon_abilities = abilities_by_pokemon.remove(&row.id).unwrap_or_default();
        pokemon_abilities.sort_by_key(|ability| ability.slot);
        let (localized_name, english_name) = species_names
            .get(&row.species_id)
            .cloned()
            .unwrap_or_default();
        Ok(PokemonRecord {
            id: PokemonFormId(row.id),
            species_id: SpeciesId(row.species_id),
            identifier: row.identifier.clone(),
            is_default: row.is_default != 0,
            base_stats: BaseStats {
                hp,
                attack,
                defense,
                special_attack,
                special_defense,
                speed,
            },
            types: pokemon_types.into_iter().map(|(_, id)| id).collect(),
            abilities: pokemon_abilities,
            display_name: localized(localized_name, english_name, &row.identifier),
            learnset: Vec::new(),
        })
    })
    .collect::<Result<Vec<_>, ImportFailure>>()?;
    let pokemon_indexes: HashMap<_, _> = pokemon
        .iter()
        .enumerate()
        .map(|(index, record)| (record.id.0, index))
        .collect();

    let learnsets_path = source.path("pokemon_moves.csv");
    for located in read_csv::<PokemonMoveRow>(
        source,
        "pokemon_moves.csv",
        &[
            "pokemon_id",
            "version_group_id",
            "move_id",
            "pokemon_move_method_id",
            "level",
            "order",
        ],
    )? {
        let row = located.value;
        if row.version_group_id != version_group_id {
            continue;
        }
        let Some(&pokemon_index) = pokemon_indexes.get(&row.pokemon_id) else {
            return fail(
                ImportDiagnostic::new(
                    ImportDiagnosticCode::MissingReference,
                    format!("unknown pokemon ID: {}", row.pokemon_id),
                )
                .file(&learnsets_path)
                .row(located.row)
                .field("pokemon_id")
                .entity(row.pokemon_id),
            );
        };
        if !move_ids.contains(&row.move_id) {
            return fail(
                ImportDiagnostic::new(
                    ImportDiagnosticCode::MissingReference,
                    format!("unknown move ID: {}", row.move_id),
                )
                .file(&learnsets_path)
                .row(located.row)
                .field("move_id")
                .entity(row.pokemon_id),
            );
        }
        let method_identifier = move_methods
            .get(&row.pokemon_move_method_id)
            .ok_or_else(|| {
                ImportFailure::one(
                    ImportDiagnostic::new(
                        ImportDiagnosticCode::MissingReference,
                        format!("unknown move method ID: {}", row.pokemon_move_method_id),
                    )
                    .file(&learnsets_path)
                    .row(located.row)
                    .field("pokemon_move_method_id")
                    .entity(row.pokemon_id),
                )
            })?;
        let method = match method_identifier.as_str() {
            "level-up" => MoveLearnMethod::LevelUp,
            "egg" => MoveLearnMethod::Egg,
            "tutor" => MoveLearnMethod::Tutor,
            "machine" => MoveLearnMethod::Machine,
            identifier => MoveLearnMethod::Other(identifier.to_owned()),
        };
        pokemon[pokemon_index].learnset.push(LearnsetEntry {
            move_id: MoveId(row.move_id),
            method,
            level: (row.level != 0).then_some(row.level),
            order: row.order,
        });
    }
    for record in &mut pokemon {
        record.learnset.sort();
        if record.learnset.windows(2).any(|pair| pair[0] == pair[1]) {
            return fail(
                ImportDiagnostic::new(
                    ImportDiagnosticCode::DuplicateId,
                    "learnset contains a duplicate entry",
                )
                .file(&learnsets_path)
                .entity(record.id.0),
            );
        }
    }

    CurrentDataSet::new(
        DataSetMetadata {
            schema_version: "current-data-set-v4".into(),
            source_repository: SOURCE_REPOSITORY.into(),
            source_commit: options.source_commit.clone(),
            generator_version: "game-data-import-0.0.0".into(),
            locale: options.locale.clone(),
            version_group: options.version_group.clone(),
        },
        pokemon,
        moves,
        abilities,
        types,
    )
    .map_err(|error| {
        ImportFailure::one(ImportDiagnostic::new(
            ImportDiagnosticCode::OutputValidationFailed,
            format!("imported data failed validation: {error}"),
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_arguments_are_complete_and_platform_neutral() {
        let parsed = parse_cli_args([
            "--source".into(),
            "source".into(),
            "--output".into(),
            "output.json".into(),
            "--version-group".into(),
            "emerald".into(),
            "--locale".into(),
            "ja".into(),
            "--source-commit".into(),
            "abc".into(),
        ])
        .unwrap()
        .unwrap();
        assert_eq!(parsed.source, PathBuf::from("source"));
        assert_eq!(parsed.output, PathBuf::from("output.json"));
        assert_eq!(parsed.import.locale, "ja");
        assert_eq!(parsed.import.source_commit, "abc");
        assert_eq!(parsed.import.version_group, "emerald");

        assert!(parse_cli_args(["--help".into()]).unwrap().is_none());
        assert!(parse_cli_args(["-h".into()]).unwrap().is_none());
        for (args, expected) in [
            (vec!["--source"], "missing value for --source"),
            (vec!["--unknown", "value"], "unknown argument: --unknown"),
            (vec![], "--source is required"),
            (vec!["--source", "source"], "--output is required"),
            (
                vec!["--source", "source", "--output", "output"],
                "--version-group is required",
            ),
        ] {
            assert_eq!(
                parse_cli_args(args.into_iter().map(str::to_owned)).unwrap_err(),
                expected
            );
        }
    }

    const SNAPSHOT: &[(&str, &str)] = &[
        (
            "SOURCE.md",
            include_str!("../../../../assets/imports/pokeapi-current-data/SOURCE.md"),
        ),
        (
            "languages.csv",
            include_str!("../../../../assets/imports/pokeapi-current-data/languages.csv"),
        ),
        (
            "version_groups.csv",
            include_str!("../../../../assets/imports/pokeapi-current-data/version_groups.csv"),
        ),
        (
            "stats.csv",
            include_str!("../../../../assets/imports/pokeapi-current-data/stats.csv"),
        ),
        (
            "types.csv",
            include_str!("../../../../assets/imports/pokeapi-current-data/types.csv"),
        ),
        (
            "type_names.csv",
            include_str!("../../../../assets/imports/pokeapi-current-data/type_names.csv"),
        ),
        (
            "move_damage_classes.csv",
            include_str!("../../../../assets/imports/pokeapi-current-data/move_damage_classes.csv"),
        ),
        (
            "pokemon_move_methods.csv",
            include_str!(
                "../../../../assets/imports/pokeapi-current-data/pokemon_move_methods.csv"
            ),
        ),
        (
            "abilities.csv",
            include_str!("../../../../assets/imports/pokeapi-current-data/abilities.csv"),
        ),
        (
            "ability_names.csv",
            include_str!("../../../../assets/imports/pokeapi-current-data/ability_names.csv"),
        ),
        (
            "pokemon_abilities.csv",
            include_str!("../../../../assets/imports/pokeapi-current-data/pokemon_abilities.csv"),
        ),
        (
            "pokemon_species_names.csv",
            include_str!(
                "../../../../assets/imports/pokeapi-current-data/pokemon_species_names.csv"
            ),
        ),
        (
            "move_names.csv",
            include_str!("../../../../assets/imports/pokeapi-current-data/move_names.csv"),
        ),
        (
            "moves.csv",
            include_str!("../../../../assets/imports/pokeapi-current-data/moves.csv"),
        ),
        (
            "pokemon_stats.csv",
            include_str!("../../../../assets/imports/pokeapi-current-data/pokemon_stats.csv"),
        ),
        (
            "pokemon_types.csv",
            include_str!("../../../../assets/imports/pokeapi-current-data/pokemon_types.csv"),
        ),
        (
            "pokemon.csv",
            include_str!("../../../../assets/imports/pokeapi-current-data/pokemon.csv"),
        ),
        (
            "pokemon_moves.csv",
            include_str!("../../../../assets/imports/pokeapi-current-data/pokemon_moves.csv"),
        ),
    ];

    fn source() -> ImportSource {
        ImportSource::new(
            PathBuf::from("snapshot"),
            SNAPSHOT
                .iter()
                .map(|(name, contents)| ((*name).to_owned(), (*contents).to_owned())),
        )
    }

    fn changed(name: &str, change: impl Fn(&str) -> String) -> ImportSource {
        ImportSource::new(
            PathBuf::from("snapshot"),
            SNAPSHOT.iter().map(|(file, contents)| {
                let contents = if *file == name {
                    change(contents)
                } else {
                    (*contents).to_owned()
                };
                ((*file).to_owned(), contents)
            }),
        )
    }

    fn without(name: &str) -> ImportSource {
        ImportSource::new(
            PathBuf::from("snapshot"),
            SNAPSHOT
                .iter()
                .filter(|(file, _)| *file != name)
                .map(|(file, contents)| ((*file).to_owned(), (*contents).to_owned())),
        )
    }

    #[track_caller]
    fn code(source: ImportSource, options: ImportOptions) -> ImportDiagnosticCode {
        match import_source(&source, &options) {
            Err(failure) => failure.diagnostics()[0].code,
            Ok(_) => panic!("expected import failure"),
        }
    }

    #[test]
    fn imports_the_complete_snapshot_without_io() {
        let data = import_source(&source(), &ImportOptions::emerald()).unwrap();

        assert!(data.pokemon_iter().next().is_some());
        assert!(data.move_iter().next().is_some());
        assert!(data.type_iter().next().is_some());
        assert_eq!(data.metadata().source_commit, PINNED_COMMIT);
    }

    #[test]
    fn diagnostics_reject_invalid_source_envelopes() {
        assert_eq!(
            code(without("SOURCE.md"), ImportOptions::emerald()),
            ImportDiagnosticCode::MissingInputFile
        );
        assert_eq!(
            code(
                changed("SOURCE.md", |contents| contents
                    .replace(PINNED_COMMIT, "wrong")),
                ImportOptions::emerald(),
            ),
            ImportDiagnosticCode::MetadataMismatch
        );
        let mut options = ImportOptions::emerald();
        options.locale = "en".into();
        assert_eq!(code(source(), options), ImportDiagnosticCode::InvalidField);
        assert_eq!(
            code(without("languages.csv"), ImportOptions::emerald()),
            ImportDiagnosticCode::MissingInputFile
        );
        assert_eq!(
            code(
                changed("languages.csv", |contents| {
                    contents.replacen("identifier", "language_name", 1)
                }),
                ImportOptions::emerald(),
            ),
            ImportDiagnosticCode::InvalidHeader
        );
        assert_eq!(
            code(
                changed("languages.csv", |_| "id,identifier\nnot-a-number,zh-hans\n"
                    .into()),
                ImportOptions::emerald(),
            ),
            ImportDiagnosticCode::InvalidField
        );
        assert_eq!(
            code(
                changed("languages.csv", |_| "id,identifier\n9,en\n".into()),
                ImportOptions::emerald(),
            ),
            ImportDiagnosticCode::MissingReference
        );
        let mut options = ImportOptions::emerald();
        options.version_group = "absent".into();
        assert_eq!(
            code(source(), options),
            ImportDiagnosticCode::MissingReference
        );
        assert_eq!(
            code(
                changed("stats.csv", |contents| {
                    contents.replace("6,,speed,0,4\n", "")
                }),
                ImportOptions::emerald(),
            ),
            ImportDiagnosticCode::MissingStat
        );
    }

    #[test]
    fn diagnostics_reject_invalid_moves_and_stats() {
        assert_eq!(
            code(
                changed("move_damage_classes.csv", |contents| {
                    contents.replace("2,physical", "2,unknown")
                }),
                ImportOptions::emerald(),
            ),
            ImportDiagnosticCode::InvalidField
        );
        assert_eq!(
            code(
                changed("move_damage_classes.csv", |contents| {
                    contents.replace("2,physical\n", "")
                }),
                ImportOptions::emerald(),
            ),
            ImportDiagnosticCode::MissingReference
        );
        assert_eq!(
            code(
                changed("moves.csv", |contents| {
                    contents.replacen("1,pound,1,1,40", "1,pound,1,999,40", 1)
                }),
                ImportOptions::emerald(),
            ),
            ImportDiagnosticCode::MissingReference
        );
        assert_eq!(
            code(
                changed("pokemon_stats.csv", |contents| {
                    format!("{contents}\n1,1,45,0\n")
                }),
                ImportOptions::emerald(),
            ),
            ImportDiagnosticCode::DuplicateId
        );
        assert_eq!(
            code(
                changed("pokemon_stats.csv", |contents| {
                    contents
                        .lines()
                        .filter(|line| !line.starts_with("1,"))
                        .collect::<Vec<_>>()
                        .join("\n")
                }),
                ImportOptions::emerald(),
            ),
            ImportDiagnosticCode::MissingStat
        );
        assert_eq!(
            code(
                changed("pokemon_stats.csv", |contents| {
                    contents
                        .lines()
                        .filter(|line| *line != "1,6,45,0")
                        .collect::<Vec<_>>()
                        .join("\n")
                }),
                ImportOptions::emerald(),
            ),
            ImportDiagnosticCode::MissingStat
        );
        assert_eq!(
            code(
                changed("moves.csv", |contents| {
                    format!("{contents}\n1,pound-copy,1,1,40,35,100,0,10,2,1,,5,1,5\n")
                }),
                ImportOptions::emerald(),
            ),
            ImportDiagnosticCode::OutputValidationFailed
        );
    }

    #[test]
    fn diagnostics_reject_invalid_types_and_learnsets() {
        for (contents, expected) in [
            ("1,999,2", ImportDiagnosticCode::MissingReference),
            ("1,4,3", ImportDiagnosticCode::InvalidTypeSlots),
            ("1,4,1", ImportDiagnosticCode::InvalidTypeSlots),
        ] {
            assert_eq!(
                code(
                    changed("pokemon_types.csv", |original| {
                        format!("{original}\n{contents}\n")
                    }),
                    ImportOptions::emerald(),
                ),
                expected
            );
        }
        assert_eq!(
            code(
                changed("pokemon_types.csv", |contents| {
                    contents
                        .lines()
                        .filter(|line| !line.starts_with("1,"))
                        .collect::<Vec<_>>()
                        .join("\n")
                }),
                ImportOptions::emerald(),
            ),
            ImportDiagnosticCode::InvalidTypeSlots
        );
        for row in ["999999,6,1,1,1,1,", "1,6,999999,1,1,1,", "1,6,1,99,1,1,"] {
            let source = changed("pokemon_moves.csv", |contents| {
                format!("{contents}\n{row}\n")
            });
            assert!(
                import_source(&source, &ImportOptions::emerald()).is_err(),
                "row should fail: {row}"
            );
            assert_eq!(
                code(source, ImportOptions::emerald()),
                ImportDiagnosticCode::MissingReference
            );
        }
        assert_eq!(
            code(
                changed("pokemon_moves.csv", |contents| {
                    format!("{contents}\n1,6,33,1,1,,\n")
                }),
                ImportOptions::emerald(),
            ),
            ImportDiagnosticCode::DuplicateId
        );
    }

    #[test]
    fn diagnostics_and_external_failures_have_complete_text() {
        let failure = import_source(
            &changed("pokemon_types.csv", |contents| format!("{contents}1,4,3\n")),
            &ImportOptions::emerald(),
        )
        .unwrap_err();
        let rendered = failure.to_string();
        assert!(rendered.contains("InvalidTypeSlots"));
        assert!(rendered.contains("pokemon_types.csv"));
        assert!(rendered.contains("field=slot"));
        assert!(rendered.contains("entity=1"));

        let combined = ImportFailure {
            diagnostics: vec![
                ImportFailure::output("serialize".into()).diagnostics[0].clone(),
                ImportFailure::output_at("output.json", "publish".into()).diagnostics[0].clone(),
            ],
        };
        assert_eq!(combined.to_string().lines().count(), 2);
        assert!(combined.to_string().contains("output.json"));
    }
}
