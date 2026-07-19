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
        include_str!("../../../../../assets/imports/pokeapi-current-data/SOURCE.md"),
    ),
    (
        "languages.csv",
        include_str!("../../../../../assets/imports/pokeapi-current-data/languages.csv"),
    ),
    (
        "version_groups.csv",
        include_str!("../../../../../assets/imports/pokeapi-current-data/version_groups.csv"),
    ),
    (
        "stats.csv",
        include_str!("../../../../../assets/imports/pokeapi-current-data/stats.csv"),
    ),
    (
        "types.csv",
        include_str!("../../../../../assets/imports/pokeapi-current-data/types.csv"),
    ),
    (
        "type_names.csv",
        include_str!("../../../../../assets/imports/pokeapi-current-data/type_names.csv"),
    ),
    (
        "move_damage_classes.csv",
        include_str!("../../../../../assets/imports/pokeapi-current-data/move_damage_classes.csv"),
    ),
    (
        "pokemon_move_methods.csv",
        include_str!("../../../../../assets/imports/pokeapi-current-data/pokemon_move_methods.csv"),
    ),
    (
        "abilities.csv",
        include_str!("../../../../../assets/imports/pokeapi-current-data/abilities.csv"),
    ),
    (
        "ability_names.csv",
        include_str!("../../../../../assets/imports/pokeapi-current-data/ability_names.csv"),
    ),
    (
        "pokemon_abilities.csv",
        include_str!("../../../../../assets/imports/pokeapi-current-data/pokemon_abilities.csv"),
    ),
    (
        "pokemon_species_names.csv",
        include_str!(
            "../../../../../assets/imports/pokeapi-current-data/pokemon_species_names.csv"
        ),
    ),
    (
        "move_names.csv",
        include_str!("../../../../../assets/imports/pokeapi-current-data/move_names.csv"),
    ),
    (
        "moves.csv",
        include_str!("../../../../../assets/imports/pokeapi-current-data/moves.csv"),
    ),
    (
        "pokemon_stats.csv",
        include_str!("../../../../../assets/imports/pokeapi-current-data/pokemon_stats.csv"),
    ),
    (
        "pokemon_types.csv",
        include_str!("../../../../../assets/imports/pokeapi-current-data/pokemon_types.csv"),
    ),
    (
        "pokemon.csv",
        include_str!("../../../../../assets/imports/pokeapi-current-data/pokemon.csv"),
    ),
    (
        "pokemon_moves.csv",
        include_str!("../../../../../assets/imports/pokeapi-current-data/pokemon_moves.csv"),
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
