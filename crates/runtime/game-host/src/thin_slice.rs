use std::{
    error::Error,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
};

use game_foundation::{GameState, SaveEnvelope, ThinSliceContent};
use game_ramus_adapter::{GameRamusRouter, RoutedIntent};

use crate::trainer_content;

const DEFAULT_SAVE_PATH: &str = "target/thin-slice.save.json";
type EntryResult = Result<(), Box<dyn Error>>;
type ArgumentResult = Result<Option<EntryResult>, Box<dyn Error>>;
const THIN_SLICE_SCRIPT: &str = "/game/session new
/game/world move direction=up
/game/npc interact npc=professor
/game/world move direction=right
/game/world warp warp=town-to-route
/game/world move direction=right
/game/world encounter roll=7
/game/battle resolve outcome=victory hp=28 pp=24
/game/world move direction=down
/game/world move direction=right
/game/npc interact npc=route-trainer
/game/battle resolve outcome=victory hp=19 pp=16
/game/world warp warp=route-to-town
/game/world move direction=up
/game/shop buy npc=merchant item=potion quantity=1
/game/save save";

pub fn run_from_arguments(arguments: impl Iterator<Item = OsString>) -> ArgumentResult {
    let arguments = arguments.collect::<Vec<_>>();
    let Some(flag) = arguments.first().and_then(|value| value.to_str()) else {
        return Ok(None);
    };
    match (flag, arguments.as_slice()) {
        ("--thin-slice", [_, path]) => {
            let path = PathBuf::from(path);
            Ok(Some(run_script(THIN_SLICE_SCRIPT, &path)))
        }
        ("--thin-slice", [_]) => Ok(Some(run_script(
            THIN_SLICE_SCRIPT,
            Path::new(DEFAULT_SAVE_PATH),
        ))),
        ("--ramus", [_, source, path]) => {
            let Some(source) = source.to_str() else {
                return Ok(Some(Err(usage("--ramus source must be valid UTF-8"))));
            };
            Ok(Some(run_script(source, Path::new(path))))
        }
        ("--ramus", [_, source]) => {
            let Some(source) = source.to_str() else {
                return Ok(Some(Err(usage("--ramus source must be valid UTF-8"))));
            };
            Ok(Some(run_script(source, Path::new(DEFAULT_SAVE_PATH))))
        }
        _ => Ok(Some(Err(usage(
            "game-host [--thin-slice [save-path] | --ramus <source> [save-path]]",
        )))),
    }
}

fn run_script(source: &str, save_path: &Path) -> Result<(), Box<dyn Error>> {
    let trainer_catalog = trainer_content::load_trainer_catalog()?;
    let content = ThinSliceContent::standard()
        .and_then(|content| content.with_trainer_catalog(trainer_catalog))
        .map_err(content_error)?;
    let router = GameRamusRouter::new().map_err(router_error)?;
    let intents = router.route(source).map_err(router_error)?;
    let mut state = GameState::new(&content).map_err(foundation_error)?;
    for intent in intents {
        match intent {
            RoutedIntent::Command(command) => {
                let (next, event) = state.transition(&content, command);
                state = next;
                event.map_err(foundation_error)?;
            }
            RoutedIntent::Save => state = save_and_reload(&content, state, save_path)?,
        }
    }
    println!("thin-slice state: {state:?}");
    Ok(())
}

pub(crate) fn save_and_reload(
    content: &ThinSliceContent,
    state: GameState,
    path: &Path,
) -> Result<GameState, Box<dyn Error>> {
    let envelope = SaveEnvelope::from_state(content, state.clone()).map_err(save_error)?;
    let bytes = envelope.to_json().map_err(save_error)?;
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)?;
    let reloaded = fs::read(path)?;
    let loaded = SaveEnvelope::from_json(content, &reloaded).map_err(save_error)?;
    if loaded.state() != &state {
        return Err(io::Error::other("save reload did not preserve game state").into());
    }
    Ok(loaded.state().clone())
}

fn foundation_error(error: game_foundation::GameError) -> Box<dyn Error> {
    io::Error::other(format!("game command rejected: {error:?}")).into()
}

fn content_error(error: game_foundation::ContentError) -> Box<dyn Error> {
    io::Error::other(format!("game content rejected: {error:?}")).into()
}

fn save_error(error: game_foundation::SaveError) -> Box<dyn Error> {
    io::Error::other(format!("save rejected: {error:?}")).into()
}

fn router_error(error: game_ramus_adapter::RouterDiagnostic) -> Box<dyn Error> {
    io::Error::other(format!(
        "Ramus intent rejected: {}: {}",
        error.code, error.message
    ))
    .into()
}

fn usage(message: &str) -> Box<dyn Error> {
    io::Error::new(io::ErrorKind::InvalidInput, message).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_foundation::{ItemId, Money};

    #[test]
    fn thin_slice_entry_routes_ramus_intents_and_round_trips_the_save() -> Result<(), Box<dyn Error>>
    {
        let path = std::env::temp_dir().join(format!(
            "pokemon-untitled-thin-slice-{}.json",
            std::process::id()
        ));
        run_script(THIN_SLICE_SCRIPT, &path)?;
        let content = ThinSliceContent::standard().map_err(content_error)?;
        let bytes = fs::read(&path)?;
        let save = SaveEnvelope::from_json(&content, &bytes).map_err(save_error)?;
        let trainer = game_foundation::NpcId::new("route-trainer")
            .map_err(|error| io::Error::other(format!("trainer id rejected: {error:?}")))?;
        let potion = ItemId::new("potion")
            .map_err(|error| io::Error::other(format!("item id rejected: {error:?}")))?;
        assert!(save.state().defeated_trainers().contains(&trainer));
        assert_eq!(save.state().money(), Money::new(290));
        assert_eq!(save.state().inventory().quantity(&potion), 2);
        fs::remove_file(path)?;
        Ok(())
    }
}
