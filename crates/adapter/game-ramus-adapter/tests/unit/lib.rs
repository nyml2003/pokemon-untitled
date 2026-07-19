use game_foundation::{BattleOutcome, GameCommand};

use super::{GameRamusRouter, RoutedIntent};

#[test]
fn routes_the_complete_thin_slice_command_language() -> Result<(), String> {
    let router = GameRamusRouter::new().map_err(|error| error.message)?;
    let intents = router.route("/game/session new\n/game/npc interact npc=professor\n/game/world move direction=right\n/game/world warp warp=town-to-route\n/game/world encounter roll=7\n/game/battle resolve outcome=victory hp=28 pp=24\n/game/npc interact npc=route-trainer\n/game/battle resolve outcome=victory hp=19 pp=16\n/game/world warp warp=route-to-town\n/game/shop buy npc=merchant item=potion quantity=1\n/game/save save").map_err(|error| error.message)?;
    assert_eq!(intents.len(), 11);
    assert!(matches!(
        intents.first(),
        Some(RoutedIntent::Command(GameCommand::NewGame))
    ));
    assert!(matches!(
        intents.get(5),
        Some(RoutedIntent::Command(GameCommand::ResolveBattle {
            outcome: BattleOutcome::Victory,
            hp: 28,
            pp: 24
        }))
    ));
    assert!(matches!(intents.last(), Some(RoutedIntent::Save)));
    Ok(())
}

#[test]
fn rejects_non_ramus_shell_syntax() -> Result<(), String> {
    let router = GameRamusRouter::new().map_err(|error| error.message)?;
    let error = match router.route("/game/session new | cat") {
        Ok(_) => return Err("Ramus accepted shell pipe syntax".into()),
        Err(error) => error,
    };
    assert_eq!(error.stage, super::DiagnosticStage::Parse);
    assert_eq!(error.code, "forbidden-syntax");
    Ok(())
}
