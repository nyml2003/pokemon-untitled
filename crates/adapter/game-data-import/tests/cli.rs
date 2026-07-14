use std::{fs, path::PathBuf, process::Command};

use game_data::{CurrentDataSet, MoveId, PokemonFormId};

#[test]
fn imports_the_pinned_snapshot_and_emits_queryable_data() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = manifest.join("../../../assets/imports/pokeapi-current-data");
    let output = std::env::temp_dir().join(format!(
        "gen3-game-data-import-{}-current-data-set-v1.json",
        std::process::id()
    ));

    let status = Command::new(env!("CARGO_BIN_EXE_game-data-import"))
        .args([
            "--source",
            source.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--version-group",
            "emerald",
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let bytes = fs::read(&output).unwrap();
    let data = CurrentDataSet::from_json(&bytes).unwrap();
    assert_eq!(data.pokemon_iter().count(), 1_351);
    assert_eq!(data.move_iter().count(), 937);
    assert_eq!(data.metadata().version_group, "emerald");
    assert!(data.can_learn(PokemonFormId(1), MoveId(33)));
    assert!(data.can_learn(PokemonFormId(1), MoveId(22)));
    assert_eq!(
        data.pokemon(PokemonFormId(1))
            .unwrap()
            .display_name
            .localized,
        "妙蛙种子"
    );
    assert_eq!(
        data.move_by_id(MoveId(1)).unwrap().display_name.localized,
        "拍击"
    );

    fs::remove_file(output).unwrap();
}
