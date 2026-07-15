//! Development-time PokeAPI importer filesystem adapter.

#![forbid(unsafe_code)]

use std::{fs, path::Path};

pub use game_data_import_core::{
    CliOptions, ImportDiagnostic, ImportDiagnosticCode, ImportFailure, ImportOptions, ImportSource,
    PINNED_COMMIT, import_source, parse_cli_args,
};

const INPUT_FILES: &[&str] = &[
    "SOURCE.md",
    "languages.csv",
    "version_groups.csv",
    "stats.csv",
    "types.csv",
    "type_names.csv",
    "move_damage_classes.csv",
    "pokemon_move_methods.csv",
    "pokemon_species_names.csv",
    "move_names.csv",
    "moves.csv",
    "pokemon_stats.csv",
    "pokemon_types.csv",
    "pokemon.csv",
    "pokemon_moves.csv",
    "abilities.csv",
    "ability_names.csv",
    "pokemon_abilities.csv",
];

pub fn import_directory(
    directory: &Path,
    options: &ImportOptions,
) -> Result<game_data::CurrentDataSet, ImportFailure> {
    let mut files = Vec::with_capacity(INPUT_FILES.len());
    for name in INPUT_FILES {
        let path = directory.join(name);
        let contents = fs::read_to_string(&path).map_err(|error| {
            ImportFailure::missing_file(path, format!("cannot read input: {error}"))
        })?;
        files.push(((*name).to_owned(), contents));
    }
    import_source(&ImportSource::new(directory.to_path_buf(), files), options)
}

pub fn generate_to_path(
    source: &Path,
    output: &Path,
    options: &ImportOptions,
) -> Result<(), ImportFailure> {
    let dataset = import_directory(source, options)?;
    let bytes = serde_json::to_vec_pretty(&dataset).map_err(|error| {
        ImportFailure::output(format!("cannot serialize generated data: {error}"))
    })?;
    game_data::CurrentDataSet::from_json(&bytes).map_err(|error| {
        ImportFailure::output(format!("generated output failed validation: {error}"))
    })?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            ImportFailure::output_at(parent, format!("cannot create output directory: {error}"))
        })?;
    }
    let temporary = output.with_extension("json.tmp");
    fs::write(&temporary, bytes).map_err(|error| {
        ImportFailure::output_at(
            &temporary,
            format!("cannot write temporary output: {error}"),
        )
    })?;
    if output.exists() {
        fs::remove_file(output).map_err(|error| {
            ImportFailure::output_at(output, format!("cannot replace existing output: {error}"))
        })?;
    }
    fs::rename(&temporary, output).map_err(|error| {
        ImportFailure::output_at(output, format!("cannot publish generated output: {error}"))
    })?;
    Ok(())
}
