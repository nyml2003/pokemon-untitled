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
use map_project_storage::{FILE_EXTENSION, MapProjectReader, MapProjectWriter};
use map_tile_semantics::TileSemanticsCatalog;

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = env::args_os().skip(1).collect::<Vec<_>>();
    if let Some(command) = arguments.first().and_then(|argument| argument.to_str())
        && matches!(command, "inspect" | "verify" | "pack" | "unpack" | "lint")
    {
        return run_subcommand(command, &arguments[1..]);
    }
    let project_path = arguments
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(default_project_path);
    let root = asset_root();
    let ids = load_atomic_ids(&root)?;
    let known = ids.iter().cloned().collect::<BTreeSet<_>>();
    let semantics = load_semantics(&root, &known)?;
    let project = load_project(&project_path, &ids)?;
    let mut model = EditorModel::with_semantics(project, ids, semantics);
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
                                Ok(()) => {
                                    model = model.saved();
                                    response
                                }
                                Err(error) => {
                                    let message = error.to_string();
                                    model = model.with_error(error);
                                    CliResponse::Error { message }
                                }
                            }
                        } else {
                            response
                        }
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

fn run_subcommand(command: &str, arguments: &[std::ffi::OsString]) -> Result<(), Box<dyn Error>> {
    match command {
        "inspect" => inspect_map(arguments),
        "verify" => verify_map(arguments),
        "pack" => pack_map(arguments),
        "unpack" => unpack_map(arguments),
        "lint" => lint_map(arguments),
        _ => Err(usage("inspect|verify|pack|unpack|lint <arguments>")),
    }
}

fn lint_map(arguments: &[std::ffi::OsString]) -> Result<(), Box<dyn Error>> {
    let (input, json) = match arguments {
        [input] => (PathBuf::from(input), false),
        [input, flag] if flag == "--json" => (PathBuf::from(input), true),
        _ => return Err(usage("lint <map-or-directory> [--json]")),
    };
    let root = asset_root();
    let ids = load_atomic_ids(&root)?;
    let known = ids.iter().cloned().collect::<BTreeSet<_>>();
    let semantics = load_semantics(&root, &known)?;
    let paths = map_paths(&input)?;
    if paths.is_empty() {
        return Err(usage("lint input contains no map project files"));
    }
    let reports = paths
        .iter()
        .map(|path| {
            let project = load_project(path, &ids)?;
            Ok(serde_json::json!({
                "path": path,
                "diagnostics": semantics.lint(&project),
            }))
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    let valid = reports
        .iter()
        .all(|report| report["diagnostics"].as_array().is_some_and(Vec::is_empty));
    if json {
        println!("{}", serde_json::json!({ "valid": valid, "maps": reports }));
    } else if valid {
        println!("semantic lint passed for {} map(s)", reports.len());
    } else {
        for report in &reports {
            let count = report["diagnostics"].as_array().map_or(0, Vec::len);
            if count > 0 {
                println!(
                    "{}: {count} semantic error(s)",
                    report["path"].as_str().unwrap_or("<unknown>")
                );
            }
        }
    }
    if valid {
        Ok(())
    } else {
        Err(usage("map semantic lint failed"))
    }
}

fn inspect_map(arguments: &[std::ffi::OsString]) -> Result<(), Box<dyn Error>> {
    let (path, json) = match arguments {
        [path] => (PathBuf::from(path), false),
        [path, flag] if flag == "--json" => (PathBuf::from(path), true),
        _ => return Err(usage("inspect <map.g3mp> [--json]")),
    };
    let metadata = MapProjectReader::inspect(&fs::read(&path)?)?;
    if json {
        println!(
            "{}",
            serde_json::json!({
                "path": path,
                "container_version": metadata.container_version,
                "document_format": metadata.document_format,
                "map_id": metadata.map_id,
                "tile_size": {
                    "width": metadata.tile_size.width,
                    "height": metadata.tile_size.height,
                },
                "dimensions": {
                    "width": metadata.width,
                    "height": metadata.height,
                    "cells": metadata.cell_count,
                },
                "materials": metadata.material_count,
                "atomic_tiles": metadata.atomic_tile_count,
                "actors": metadata.actor_count,
                "events": metadata.event_count,
                "compression": metadata.compression.as_str(),
                "payload_bytes": metadata.compressed_payload_bytes,
                "raw_payload_bytes": metadata.raw_payload_bytes,
                "integrity": "not_verified",
            })
        );
    } else {
        println!("{}", metadata.map_id);
        println!("{} x {} tiles", metadata.width, metadata.height);
        println!("{}", metadata.document_format);
        println!("{}", metadata.compression.as_str());
        println!(
            "{} compressed bytes / {} raw bytes",
            metadata.compressed_payload_bytes, metadata.raw_payload_bytes
        );
        println!("integrity: not_verified");
    }
    Ok(())
}

fn verify_map(arguments: &[std::ffi::OsString]) -> Result<(), Box<dyn Error>> {
    let [path] = arguments else {
        return Err(usage("verify <map.g3mp>"));
    };
    let ids = load_atomic_ids(&asset_root())?;
    let known = ids.into_iter().collect::<BTreeSet<_>>();
    MapProjectReader::read(&fs::read(path)?, &known)?;
    println!("verified");
    Ok(())
}

fn pack_map(arguments: &[std::ffi::OsString]) -> Result<(), Box<dyn Error>> {
    let [input, output] = arguments else {
        return Err(usage("pack <input.json> <output.g3mp>"));
    };
    let ids = load_atomic_ids(&asset_root())?;
    let json = fs::read_to_string(input)?;
    let project = project_from_json_or_default(Some(&json), &ids)?;
    let known = ids.into_iter().collect::<BTreeSet<_>>();
    let bytes = MapProjectWriter::default().write(&project, &known)?;
    fs::write(output, bytes)?;
    Ok(())
}

fn unpack_map(arguments: &[std::ffi::OsString]) -> Result<(), Box<dyn Error>> {
    let [input, output] = arguments else {
        return Err(usage("unpack <input.g3mp> <output.json>"));
    };
    let ids = load_atomic_ids(&asset_root())?;
    let known = ids.into_iter().collect::<BTreeSet<_>>();
    let project = MapProjectReader::read(&fs::read(input)?, &known)?;
    fs::write(output, project.to_json_pretty(&known)?)?;
    Ok(())
}

fn usage(message: &str) -> Box<dyn Error> {
    Box::new(io::Error::new(io::ErrorKind::InvalidInput, message))
}

fn load_atomic_ids(root: &Path) -> Result<Vec<map_project::AtomicTileId>, Box<dyn Error>> {
    let catalog = load_catalog(root)?;
    Ok(build_tile_assets(read_tile_sources(root, &catalog)?)?.ids)
}

fn load_semantics(
    root: &Path,
    known: &BTreeSet<map_project::AtomicTileId>,
) -> Result<TileSemanticsCatalog, Box<dyn Error>> {
    let path = root.join("source/map/tile/tile-semantics-v1.json");
    Ok(TileSemanticsCatalog::from_json(
        &fs::read_to_string(path)?,
        known,
    )?)
}

fn map_paths(input: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    if input.is_file() {
        return Ok(vec![input.to_owned()]);
    }
    if !input.is_dir() {
        return Err(usage("lint input does not exist"));
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(input)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            paths.extend(map_paths(&path)?);
            continue;
        }
        if path.file_name().is_some_and(|name| name == "world.json") {
            continue;
        }
        if path
            .extension()
            .is_some_and(|extension| extension == "json" || extension == FILE_EXTENSION)
        {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn load_project(
    path: &Path,
    ids: &[map_project::AtomicTileId],
) -> Result<map_project::MapProject, Box<dyn Error>> {
    if path
        .extension()
        .is_some_and(|extension| extension == FILE_EXTENSION)
    {
        let known = ids.iter().cloned().collect::<BTreeSet<_>>();
        return Ok(MapProjectReader::read(&fs::read(path)?, &known)?);
    }
    let json = read_optional_text(path)?;
    Ok(project_from_json_or_default(json.as_deref(), ids)?)
}

fn save_project(path: &Path, model: &EditorModel) -> Result<(), Box<dyn Error>> {
    let diagnostics = model
        .semantic_diagnostics()
        .ok_or_else(|| usage("tile semantic catalog is not configured"))?;
    if let Some(diagnostic) = diagnostics.first() {
        return Err(usage(&format!(
            "map semantic validation failed with {} error(s): {diagnostic:?}",
            diagnostics.len()
        )));
    }
    let known = model.atomic_ids.iter().cloned().collect::<BTreeSet<_>>();
    if path
        .extension()
        .is_some_and(|extension| extension == FILE_EXTENSION)
    {
        return Ok(fs::write(
            path,
            MapProjectWriter::default().write(&model.project, &known)?,
        )?);
    }
    let json = model.project.to_json_pretty(&known)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, json)?;
    Ok(())
}

fn default_project_path() -> PathBuf {
    let maps = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../maps");
    let compressed = maps.join("demo-map.g3mp");
    if compressed.exists() {
        compressed
    } else {
        maps.join("demo-map.json")
    }
}

fn asset_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../assets")
}
