use battle_application::Action;
use battle_ramus_adapter::{AdapterDiagnostic, BattleRamusAdapter};
use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsoleEntry {
    pub invocation: String,
}

pub struct GameConsole {
    adapter: Result<BattleRamusAdapter, AdapterDiagnostic>,
}

impl Default for GameConsole {
    fn default() -> Self {
        Self {
            adapter: BattleRamusAdapter::new(),
        }
    }
}

impl GameConsole {
    pub fn entries(&self, legal_actions: &[Action]) -> Vec<ConsoleEntry> {
        self.adapter
            .as_ref()
            .map_err(|error| error.clone())
            .and_then(|adapter| adapter.action_invocations(legal_actions))
            .unwrap_or_default()
            .into_iter()
            .map(|item| ConsoleEntry {
                invocation: item.invocation,
            })
            .collect()
    }

    pub fn execute(&self, invocation: &str) -> Result<Action, String> {
        self.adapter
            .as_ref()
            .map_err(|error| error.clone())
            .and_then(|adapter| adapter.execute_invocation(invocation))
            .map_err(format_diagnostic)
    }
}

fn format_diagnostic(diagnostic: AdapterDiagnostic) -> String {
    format!("{}: {}", diagnostic.code, diagnostic.message)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConsoleIntent {
    Open(Vec<ConsoleEntry>),
    Close,
    InsertText(String),
    Backspace,
    Next,
    Previous,
    Execute,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConsoleOutcome {
    Updated,
    Closed,
    Execute(String),
    NoSelection,
    Ignored,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConsoleState {
    open: bool,
    pub(super) query: String,
    available: Vec<ConsoleEntry>,
    pub(super) items: Vec<ConsoleEntry>,
    pub(super) selected_index: Option<usize>,
    pub(super) diagnostic: Option<String>,
    pub(super) preedit: String,
}

impl ConsoleState {
    pub const fn is_open(&self) -> bool {
        self.open
    }

    pub fn set_preedit(&mut self, text: String) {
        self.preedit = text;
    }

    pub fn handle(&mut self, intent: ConsoleIntent) -> ConsoleOutcome {
        match intent {
            ConsoleIntent::Open(entries) => {
                self.open = true;
                self.query.clear();
                self.preedit.clear();
                self.available = entries;
                self.diagnostic = self
                    .available
                    .is_empty()
                    .then(|| "当前没有可用的战斗指令".into());
                self.refresh_items();
                ConsoleOutcome::Updated
            }
            ConsoleIntent::Close if self.open => {
                self.open = false;
                self.preedit.clear();
                self.diagnostic = None;
                ConsoleOutcome::Closed
            }
            ConsoleIntent::Close => ConsoleOutcome::Ignored,
            ConsoleIntent::InsertText(text) if self.open => {
                self.query.push_str(&text);
                self.preedit.clear();
                self.diagnostic = None;
                self.refresh_items();
                ConsoleOutcome::Updated
            }
            ConsoleIntent::Backspace if self.open => {
                self.query.pop();
                self.diagnostic = None;
                self.refresh_items();
                ConsoleOutcome::Updated
            }
            ConsoleIntent::Next if self.open => {
                self.selected_index = match (self.selected_index, self.items.len()) {
                    (_, 0) => None,
                    (Some(index), len) => Some((index + 1) % len),
                    (None, _) => Some(0),
                };
                ConsoleOutcome::Updated
            }
            ConsoleIntent::Previous if self.open => {
                self.selected_index = match (self.selected_index, self.items.len()) {
                    (_, 0) => None,
                    (Some(0), len) | (None, len) => Some(len - 1),
                    (Some(index), _) => Some(index - 1),
                };
                ConsoleOutcome::Updated
            }
            ConsoleIntent::Execute if self.open => {
                let Some(invocation) = self
                    .selected_index
                    .and_then(|index| self.items.get(index))
                    .map(|item| item.invocation.clone())
                else {
                    self.diagnostic = Some("没有匹配的战斗指令".into());
                    return ConsoleOutcome::NoSelection;
                };
                ConsoleOutcome::Execute(invocation)
            }
            _ => ConsoleOutcome::Ignored,
        }
    }

    pub fn execution_succeeded(&mut self) {
        self.open = false;
        self.preedit.clear();
        self.diagnostic = None;
    }

    pub fn execution_failed(&mut self, message: impl Into<String>) {
        self.diagnostic = Some(message.into());
    }

    fn refresh_items(&mut self) {
        let pattern = Pattern::new(
            &self.query,
            CaseMatching::Smart,
            Normalization::Smart,
            AtomKind::Fuzzy,
        );
        let mut matcher = Matcher::default();
        let mut utf32_buffer = Vec::new();
        let mut matches = self
            .available
            .iter()
            .cloned()
            .filter_map(|item| {
                pattern
                    .score(
                        Utf32Str::new(item.invocation.as_str(), &mut utf32_buffer),
                        &mut matcher,
                    )
                    .map(|score| (item, score))
            })
            .collect::<Vec<_>>();
        matches.sort_by(|(left, left_score), (right, right_score)| {
            right_score
                .cmp(left_score)
                .then_with(|| left.invocation.cmp(&right.invocation))
        });
        self.items = matches.into_iter().map(|(item, _)| item).collect();
        self.selected_index = (!self.items.is_empty()).then_some(0);
    }
}

#[cfg(test)]
mod tests {
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
}
