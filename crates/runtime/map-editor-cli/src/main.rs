#![forbid(unsafe_code)]

use std::{
    collections::BTreeSet,
    env,
    error::Error,
    fs,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
};

use game_fs_assets::{load_catalog, read_optional_text, read_tile_sources};
use map_assets::{build_tile_assets, project_from_json_or_default};
use map_editor_cli::{CliResponse, execute, save_requested};
use map_editor_core::{EditorModel, EditorVirtualCommand};

fn main() -> Result<(), Box<dyn Error>> {
    let project_path = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(default_project_path);
    let ids = load_atomic_ids(&asset_root())?;
    let project = load_project(&project_path, &ids)?;
    let mut model = EditorModel::new(project, ids);
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();

    for line in stdin.lock().lines() {
        let response = match line {
            Ok(line) if line.trim().is_empty() => continue,
            Ok(line) => match serde_json::from_str::<EditorVirtualCommand>(&line) {
                Ok(command) => match execute(&model, command) {
                    Ok((next, response)) => {
                        model = next;
                        if save_requested(&response) {
                            match save_project(&project_path, &model) {
                                Ok(()) => model = model.saved(),
                                Err(error) => model = model.with_error(error),
                            }
                        }
                        response
                    }
                    Err(error) => CliResponse::Error {
                        message: error.to_string(),
                    },
                },
                Err(error) => CliResponse::Error {
                    message: format!("invalid command JSON: {error}"),
                },
            },
            Err(error) => CliResponse::Error {
                message: format!("cannot read command: {error}"),
            },
        };
        serde_json::to_writer(&mut stdout, &response)?;
        stdout.write_all(b"\n")?;
        stdout.flush()?;
    }
    Ok(())
}

fn load_atomic_ids(root: &Path) -> Result<Vec<map_project::AtomicTileId>, Box<dyn Error>> {
    let catalog = load_catalog(root)?;
    Ok(build_tile_assets(read_tile_sources(root, &catalog)?)?.ids)
}

fn load_project(
    path: &Path,
    ids: &[map_project::AtomicTileId],
) -> Result<map_project::MapProject, Box<dyn Error>> {
    let json = read_optional_text(path)?;
    Ok(project_from_json_or_default(json.as_deref(), ids)?)
}

fn save_project(path: &Path, model: &EditorModel) -> Result<(), Box<dyn Error>> {
    let known = model.atomic_ids.iter().cloned().collect::<BTreeSet<_>>();
    let json = model.project.to_json_pretty(&known)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, json)?;
    Ok(())
}

fn default_project_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../maps/demo-map.json")
}

fn asset_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../assets")
}
