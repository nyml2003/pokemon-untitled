use battle_application::{Action, MoveSlot};

use super::{ConsoleEntry, ConsoleIntent, ConsoleOutcome, ConsoleState, GameConsole};

fn entries() -> Vec<ConsoleEntry> {
    ["/battle/move/one use", "/battle/team/two switch"]
        .into_iter()
        .map(|invocation| ConsoleEntry {
            invocation: invocation.into(),
        })
        .collect()
}

#[test]
fn filtering_navigation_and_execution_are_deterministic() {
    let mut state = ConsoleState::default();
    state.handle(ConsoleIntent::Open(entries()));
    state.handle(ConsoleIntent::InsertText("teamtwo".into()));
    assert_eq!(state.items.len(), 1);
    assert_eq!(
        state.handle(ConsoleIntent::Execute),
        ConsoleOutcome::Execute("/battle/team/two switch".into())
    );

    state.handle(ConsoleIntent::InsertText("zzz".into()));
    assert_eq!(
        state.handle(ConsoleIntent::Execute),
        ConsoleOutcome::NoSelection
    );
}

#[test]
fn closed_console_ignores_editing() {
    let mut state = ConsoleState::default();
    assert_eq!(state.handle(ConsoleIntent::Close), ConsoleOutcome::Ignored);
    assert_eq!(
        state.handle(ConsoleIntent::InsertText("x".into())),
        ConsoleOutcome::Ignored
    );
}

#[test]
fn empty_and_nonempty_navigation_cover_every_console_transition() {
    let mut state = ConsoleState::default();
    state.set_preedit("draft".into());
    assert_eq!(
        state.handle(ConsoleIntent::Open(Vec::new())),
        ConsoleOutcome::Updated
    );
    assert!(state.is_open());
    assert!(state.preedit.is_empty());
    assert!(state.diagnostic.is_some());
    assert_eq!(state.handle(ConsoleIntent::Next), ConsoleOutcome::Updated);
    assert_eq!(
        state.handle(ConsoleIntent::Previous),
        ConsoleOutcome::Updated
    );
    assert_eq!(
        state.handle(ConsoleIntent::Backspace),
        ConsoleOutcome::Updated
    );
    assert_eq!(
        state.handle(ConsoleIntent::Execute),
        ConsoleOutcome::NoSelection
    );
    state.execution_failed("failed");
    assert_eq!(state.diagnostic.as_deref(), Some("failed"));
    assert_eq!(state.handle(ConsoleIntent::Close), ConsoleOutcome::Closed);

    state.handle(ConsoleIntent::Open(entries()));
    assert_eq!(state.selected_index, Some(0));
    state.handle(ConsoleIntent::Next);
    assert_eq!(state.selected_index, Some(1));
    state.handle(ConsoleIntent::Next);
    assert_eq!(state.selected_index, Some(0));
    state.handle(ConsoleIntent::Previous);
    assert_eq!(state.selected_index, Some(1));
    state.handle(ConsoleIntent::Previous);
    assert_eq!(state.selected_index, Some(0));
    state.selected_index = None;
    state.handle(ConsoleIntent::Next);
    assert_eq!(state.selected_index, Some(0));
    state.selected_index = None;
    state.handle(ConsoleIntent::Previous);
    assert_eq!(state.selected_index, Some(1));
    state.set_preedit("ime".into());
    state.execution_succeeded();
    assert!(!state.is_open());
    assert!(state.preedit.is_empty());
    assert_eq!(
        state.handle(ConsoleIntent::Execute),
        ConsoleOutcome::Ignored
    );
}

#[test]
fn selected_invocation_executes_to_the_exact_action() {
    let console = GameConsole::default();
    let action = Action::UseMove(MoveSlot::new(2).unwrap());
    let invocation = console.entries(&[action]).remove(0).invocation;
    assert_eq!(console.execute(&invocation), Ok(action));
    assert!(console.execute("/not/a/battle/action").is_err());
}
