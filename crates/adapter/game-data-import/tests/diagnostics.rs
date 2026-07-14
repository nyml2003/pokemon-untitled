use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use game_data_import::{ImportDiagnosticCode, ImportOptions, generate_to_path, import_directory};

static NEXT_TEMP: AtomicUsize = AtomicUsize::new(0);

struct TempSnapshot(PathBuf);

impl TempSnapshot {
    fn copy() -> Self {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../assets/imports/pokeapi-current-data");
        let path = std::env::temp_dir().join(format!(
            "gen3-game-data-diagnostics-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        for entry in fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                fs::copy(entry.path(), path.join(entry.file_name())).unwrap();
            }
        }
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempSnapshot {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn missing_input_is_structured_and_does_not_replace_output() {
    let snapshot = TempSnapshot::copy();
    fs::remove_file(snapshot.path().join("languages.csv")).unwrap();
    let output = snapshot.path().join("output.json");
    fs::write(&output, b"existing").unwrap();

    let failure =
        generate_to_path(snapshot.path(), &output, &ImportOptions::emerald()).unwrap_err();
    assert_eq!(
        failure.diagnostics()[0].code,
        ImportDiagnosticCode::MissingInputFile
    );
    assert_eq!(
        failure.diagnostics()[0]
            .file
            .as_ref()
            .unwrap()
            .file_name()
            .unwrap(),
        "languages.csv"
    );
    assert_eq!(fs::read(output).unwrap(), b"existing");
}

#[test]
fn missing_required_header_has_a_field_diagnostic() {
    let snapshot = TempSnapshot::copy();
    let path = snapshot.path().join("pokemon.csv");
    let contents = fs::read_to_string(&path).unwrap();
    fs::write(&path, contents.replacen("species_id", "species_ref", 1)).unwrap();

    let failure = import_directory(snapshot.path(), &ImportOptions::emerald()).unwrap_err();
    let diagnostic = &failure.diagnostics()[0];
    assert_eq!(diagnostic.code, ImportDiagnosticCode::InvalidHeader);
    assert_eq!(diagnostic.field.as_deref(), Some("species_id"));
}

#[test]
fn duplicate_type_slot_reports_the_row_and_pokemon() {
    let snapshot = TempSnapshot::copy();
    let path = snapshot.path().join("pokemon_types.csv");
    writeln!(OpenOptions::new().append(true).open(path).unwrap(), "1,4,1").unwrap();

    let failure = import_directory(snapshot.path(), &ImportOptions::emerald()).unwrap_err();
    let diagnostic = &failure.diagnostics()[0];
    assert_eq!(diagnostic.code, ImportDiagnosticCode::InvalidTypeSlots);
    assert_eq!(diagnostic.field.as_deref(), Some("slot"));
    assert_eq!(diagnostic.entity_id, Some(1));
    assert!(diagnostic.row.is_some());
}
