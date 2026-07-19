//! Deterministic owner of Gen3 presentation and interaction state.

#![forbid(unsafe_code)]

mod console;
mod presentation;

use battle_session::{Action, BattleInteraction, BattleObservation, MoveSlot, TeamSlot};
use punctum_input::{KeyEvent, KeyPhase, LogicalKey, NamedKey};

pub use console::{ConsoleEntry, ConsoleIntent, ConsoleOutcome, ConsoleState, GameConsole};
pub use presentation::{
    PokedexAction, PokedexUiSnapshot, PresentationAction, PresentationSnapshot, PresentationState,
    PresentationUpdate,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WorldAnimation {
    #[default]
    Stand,
    Walk,
    Run,
    RunStopping,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BattleMenuPage {
    #[default]
    Main,
    Fight,
    Pokemon,
    Hidden,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BattleUiState {
    page: BattleMenuPage,
    selected_index: usize,
    replacement_mode: bool,
    notice: Option<&'static str>,
}

impl BattleUiState {
    pub const fn view(self) -> (BattleMenuPage, usize, Option<&'static str>) {
        (self.page, self.selected_index, self.notice)
    }

    fn reset(&mut self) {
        self.page = BattleMenuPage::Main;
        self.selected_index = 0;
        self.replacement_mode = false;
        self.notice = None;
    }

    pub fn synced(mut self, interaction: &BattleInteraction) -> Self {
        self.sync_interaction(interaction);
        self
    }

    fn sync_interaction(&mut self, interaction: &BattleInteraction) {
        match interaction {
            BattleInteraction::ChooseAction(_)
                if self.replacement_mode || self.page == BattleMenuPage::Hidden =>
            {
                self.reset();
            }
            BattleInteraction::ChooseReplacement(prompt) if !self.replacement_mode => {
                self.page = BattleMenuPage::Pokemon;
                self.replacement_mode = true;
                self.notice = None;
                let selected = (0..battle_session::TEAM_SIZE).find(|index| {
                    TeamSlot::new(*index)
                        .is_ok_and(|slot| prompt.legal_actions().contains(&Action::Switch(slot)))
                });
                let Some(selected) = selected else {
                    self.page = BattleMenuPage::Hidden;
                    self.notice = Some("没有可替换的宝可梦。");
                    return;
                };
                self.selected_index = selected;
            }
            BattleInteraction::PlaybackLocked | BattleInteraction::Finished(_) => {
                self.page = BattleMenuPage::Hidden;
                self.notice = None;
            }
            BattleInteraction::ChooseAction(_) | BattleInteraction::ChooseReplacement(_) => {}
        }
    }

    pub fn handle_key(
        mut self,
        key: &KeyEvent,
        interaction: &BattleInteraction,
    ) -> (Self, BattleUiOutcome) {
        self.sync_interaction(interaction);
        let Some((observation, actions)) = prompt_data(interaction) else {
            return (self, BattleUiOutcome::Ignored);
        };
        if key.phase == KeyPhase::Release {
            return (self, BattleUiOutcome::Ignored);
        }
        let item_count = self.item_count(observation, actions);
        debug_assert!(item_count > 0);
        self.selected_index = self.selected_index.min(item_count - 1);
        self.notice = None;
        let outcome = match key.logical {
            LogicalKey::Named(NamedKey::ArrowLeft) | LogicalKey::Named(NamedKey::ArrowUp) => {
                self.selected_index = (self.selected_index + item_count - 1) % item_count;
                BattleUiOutcome::Updated
            }
            LogicalKey::Named(NamedKey::ArrowRight) | LogicalKey::Named(NamedKey::ArrowDown) => {
                self.selected_index = (self.selected_index + 1) % item_count;
                BattleUiOutcome::Updated
            }
            LogicalKey::Named(NamedKey::Escape)
                if self.page != BattleMenuPage::Main && !self.replacement_mode =>
            {
                self.reset();
                BattleUiOutcome::Updated
            }
            LogicalKey::Named(NamedKey::Enter) if key.phase == KeyPhase::Press => {
                self.activate(observation, actions)
            }
            _ => BattleUiOutcome::Ignored,
        };
        (self, outcome)
    }

    fn item_count(self, observation: &BattleObservation, actions: &[Action]) -> usize {
        match self.page {
            BattleMenuPage::Main => 4,
            BattleMenuPage::Fight => {
                if actions.contains(&Action::Struggle) {
                    1
                } else {
                    active_pokemon(observation).moves().len()
                }
            }
            BattleMenuPage::Pokemon => observation.own().members().len(),
            BattleMenuPage::Hidden => 0,
        }
    }

    fn activate(&mut self, observation: &BattleObservation, actions: &[Action]) -> BattleUiOutcome {
        match self.page {
            BattleMenuPage::Main => match self.selected_index {
                0 => {
                    self.page = BattleMenuPage::Fight;
                    self.selected_index = 0;
                    BattleUiOutcome::Updated
                }
                1 => {
                    self.page = BattleMenuPage::Pokemon;
                    self.selected_index = observation.own().active_slot().index();
                    BattleUiOutcome::Updated
                }
                2 => {
                    self.notice = Some("包包现在还不能使用。");
                    BattleUiOutcome::Updated
                }
                3 => actions
                    .iter()
                    .copied()
                    .find(|action| *action == Action::Run)
                    .map_or_else(
                        || {
                            self.notice = Some("现在无法逃走。");
                            BattleUiOutcome::Updated
                        },
                        BattleUiOutcome::Submit,
                    ),
                _ => BattleUiOutcome::Ignored,
            },
            BattleMenuPage::Fight => {
                let action = if actions.contains(&Action::Struggle) {
                    Action::Struggle
                } else {
                    let Ok(slot) = MoveSlot::new(self.selected_index) else {
                        self.notice = Some("招式选择无效。");
                        return BattleUiOutcome::Updated;
                    };
                    Action::UseMove(slot)
                };
                if actions.contains(&action) {
                    BattleUiOutcome::Submit(action)
                } else {
                    self.notice = Some("这个招式的 PP 已用完。");
                    BattleUiOutcome::Updated
                }
            }
            BattleMenuPage::Pokemon => {
                let Ok(slot) = TeamSlot::new(self.selected_index) else {
                    self.notice = Some("宝可梦选择无效。");
                    return BattleUiOutcome::Updated;
                };
                let action = Action::Switch(slot);
                if actions.contains(&action) {
                    BattleUiOutcome::Submit(action)
                } else if observation.own().active_slot().index() == self.selected_index {
                    self.notice = Some("这只宝可梦正在战斗。");
                    BattleUiOutcome::Updated
                } else {
                    self.notice = Some("这只宝可梦已经无法战斗。");
                    BattleUiOutcome::Updated
                }
            }
            BattleMenuPage::Hidden => BattleUiOutcome::Ignored,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattleUiOutcome {
    Updated,
    Submit(Action),
    Ignored,
}

fn prompt_data(interaction: &BattleInteraction) -> Option<(&BattleObservation, &[Action])> {
    match interaction {
        BattleInteraction::ChooseAction(prompt) => {
            Some((prompt.observation(), prompt.legal_actions()))
        }
        BattleInteraction::ChooseReplacement(prompt) => {
            Some((prompt.observation(), prompt.legal_actions()))
        }
        BattleInteraction::PlaybackLocked | BattleInteraction::Finished(_) => None,
    }
}

fn active_pokemon(observation: &BattleObservation) -> &battle_session::Pokemon {
    &observation.own().members()[observation.own().active_slot().index()]
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommandConsoleView {
    pub query: String,
    pub preedit: String,
    pub items: Vec<String>,
    pub selected_index: Option<usize>,
    pub diagnostic: Option<String>,
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
