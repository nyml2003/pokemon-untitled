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
                self.selected_index = (0..battle_session::TEAM_SIZE)
                    .find(|index| {
                        prompt.legal_actions().contains(&Action::Switch(
                            TeamSlot::new(*index).expect("team indexes stay within the team limit"),
                        ))
                    })
                    .expect("a replacement prompt offers at least one team switch");
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
                    Action::UseMove(
                        MoveSlot::new(self.selected_index)
                            .expect("visible move indexes stay within the move limit"),
                    )
                };
                if actions.contains(&action) {
                    BattleUiOutcome::Submit(action)
                } else {
                    self.notice = Some("这个招式的 PP 已用完。");
                    BattleUiOutcome::Updated
                }
            }
            BattleMenuPage::Pokemon => {
                let action = Action::Switch(
                    TeamSlot::new(self.selected_index)
                        .expect("team page indexes stay within the team limit"),
                );
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
mod tests {
    use battle_application::{
        Accuracy, BattleApplication, BattleStats, Move, MoveId, Pokemon, PokemonId, PokemonType,
        TEAM_SIZE, Team,
    };
    use battle_session::{BattleCoordinator, BattleSession, OpponentPolicy};
    use game_data::CurrentDataSet;
    use game_session::{GameCommand, GameSession};
    use punctum_input::{KeyEvent, KeyPhase, LogicalKey, Modifiers, NamedKey};
    use world_application::Direction;

    use super::*;

    fn key(name: NamedKey, phase: KeyPhase) -> KeyEvent {
        KeyEvent {
            physical: None,
            logical: LogicalKey::Named(name),
            modifiers: Modifiers::default(),
            phase,
        }
    }

    fn battle_game() -> GameSession {
        let mut game = GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 17).unwrap();
        for _ in 0..4 {
            let (next, result) = game.transition(GameCommand::StepWorld(Direction::Right));
            result.unwrap();
            game = next;
        }
        game
    }

    #[test]
    fn battle_menu_navigation_and_every_activation_are_deterministic() {
        let game = battle_game();
        let snapshot = game.snapshot();
        let battle = snapshot.battle().unwrap();
        let interaction = battle.session().interaction();
        let BattleInteraction::ChooseAction(prompt) = interaction else {
            panic!("new battles choose an action");
        };
        let observation = prompt.observation();
        let actions = prompt.legal_actions();

        let mut state = BattleUiState::default();
        for name in [
            NamedKey::ArrowLeft,
            NamedKey::ArrowRight,
            NamedKey::ArrowUp,
            NamedKey::ArrowDown,
        ] {
            let (next, outcome) = state.handle_key(&key(name, KeyPhase::Press), interaction);
            assert_eq!(outcome, BattleUiOutcome::Updated);
            state = next;
        }
        let (_, outcome) = state.handle_key(&key(NamedKey::Enter, KeyPhase::Release), interaction);
        assert_eq!(outcome, BattleUiOutcome::Ignored);
        let (_, outcome) = state.handle_key(&key(NamedKey::Enter, KeyPhase::Repeat), interaction);
        assert_eq!(outcome, BattleUiOutcome::Ignored);

        let mut fight = BattleUiState::default();
        assert_eq!(
            fight.activate(observation, actions),
            BattleUiOutcome::Updated
        );
        assert_eq!(fight.page, BattleMenuPage::Fight);
        assert!(matches!(
            fight.activate(observation, actions),
            BattleUiOutcome::Submit(Action::UseMove(_))
        ));
        assert_eq!(fight.activate(observation, &[]), BattleUiOutcome::Updated);
        assert!(fight.notice.is_some());
        assert_eq!(fight.item_count(observation, &[Action::Struggle]), 1);
        assert_eq!(
            fight.activate(observation, &[Action::Struggle]),
            BattleUiOutcome::Submit(Action::Struggle)
        );
        let (fight, outcome) =
            fight.handle_key(&key(NamedKey::Escape, KeyPhase::Press), interaction);
        assert_eq!(outcome, BattleUiOutcome::Updated);
        assert_eq!(fight.page, BattleMenuPage::Main);

        let mut pokemon = BattleUiState {
            selected_index: 1,
            ..BattleUiState::default()
        };
        assert_eq!(
            pokemon.activate(observation, actions),
            BattleUiOutcome::Updated
        );
        assert_eq!(pokemon.page, BattleMenuPage::Pokemon);
        pokemon.selected_index = 1;
        assert_eq!(pokemon.item_count(observation, actions), TEAM_SIZE);
        assert_eq!(
            pokemon.activate(observation, actions),
            BattleUiOutcome::Submit(Action::Switch(TeamSlot::new(1).unwrap()))
        );
        pokemon.selected_index = observation.own().active_slot().index();
        assert_eq!(pokemon.activate(observation, &[]), BattleUiOutcome::Updated);
        assert_eq!(pokemon.notice, Some("这只宝可梦正在战斗。"));
        pokemon.selected_index = 1;
        assert_eq!(pokemon.activate(observation, &[]), BattleUiOutcome::Updated);
        assert_eq!(pokemon.notice, Some("这只宝可梦已经无法战斗。"));

        let mut bag = BattleUiState {
            selected_index: 2,
            ..BattleUiState::default()
        };
        assert_eq!(bag.activate(observation, actions), BattleUiOutcome::Updated);
        assert!(bag.notice.is_some());
        let mut run = BattleUiState {
            selected_index: 3,
            ..BattleUiState::default()
        };
        assert_eq!(
            run.activate(observation, actions),
            BattleUiOutcome::Submit(Action::Run)
        );
        assert_eq!(run.activate(observation, &[]), BattleUiOutcome::Updated);
        assert!(run.notice.is_some());
        let mut invalid = BattleUiState {
            selected_index: 4,
            ..BattleUiState::default()
        };
        assert_eq!(
            invalid.activate(observation, actions),
            BattleUiOutcome::Ignored
        );
        invalid.page = BattleMenuPage::Hidden;
        assert_eq!(
            invalid.activate(observation, actions),
            BattleUiOutcome::Ignored
        );
        assert_eq!(invalid.item_count(observation, actions), 0);
        let (_, outcome) = invalid.handle_key(&key(NamedKey::Enter, KeyPhase::Press), interaction);
        assert_eq!(outcome, BattleUiOutcome::Updated);

        let hidden = BattleUiState {
            page: BattleMenuPage::Hidden,
            ..BattleUiState::default()
        }
        .synced(interaction);
        assert_eq!(hidden.page, BattleMenuPage::Main);
        let playback = BattleUiState {
            notice: Some("old"),
            ..hidden
        }
        .synced(&BattleInteraction::PlaybackLocked);
        assert_eq!(playback.page, BattleMenuPage::Hidden);
        let (_, outcome) = playback.handle_key(
            &key(NamedKey::Enter, KeyPhase::Press),
            &BattleInteraction::PlaybackLocked,
        );
        assert_eq!(outcome, BattleUiOutcome::Ignored);
        assert_eq!(BattleUiState::default().view().0, BattleMenuPage::Main);
    }

    #[derive(Default)]
    struct FirstMove;

    impl OpponentPolicy for FirstMove {
        fn choose_action(
            &self,
            _observation: &BattleObservation,
            legal_actions: &[Action],
        ) -> Option<Action> {
            legal_actions
                .iter()
                .copied()
                .find(|action| matches!(action, Action::UseMove(_)))
                .or_else(|| legal_actions.first().copied())
        }
    }

    fn battle_move(name: &str, power: u16) -> Move {
        Move::new(
            MoveId::new(name).unwrap(),
            name,
            PokemonType::Normal,
            power,
            Accuracy::AlwaysHit,
            20,
            20,
            0,
        )
        .unwrap()
    }

    fn pokemon(name: &str, hp: u32, attack: u16, speed: u16, power: u16) -> Pokemon {
        Pokemon::new(
            PokemonId::new(name).unwrap(),
            name,
            50,
            PokemonType::Normal,
            None,
            hp,
            hp,
            BattleStats::new(attack, 50, attack, 50, speed).unwrap(),
            vec![battle_move(&format!("{name}-move"), power)],
        )
        .unwrap()
    }

    fn team(prefix: &str, lead: Pokemon) -> Team {
        let mut members = vec![lead];
        for index in 1..TEAM_SIZE {
            members.push(pokemon(&format!("{prefix}-{index}"), 100, 50, 50, 40));
        }
        Team::new(members).unwrap()
    }

    #[test]
    fn replacement_prompt_selects_the_first_offered_team_slot() {
        let player = team("player", pokemon("victim", 10, 10, 1, 1));
        let opponent = team("opponent", pokemon("killer", 100, 500, 100, 500));
        let application = BattleApplication::new(player, opponent, 9).unwrap();
        let mut session = BattleSession::new(BattleCoordinator::new(application, FirstMove));
        let action = session.legal_actions()[0];
        let (next, result) = session.submit(action);
        result.unwrap();
        session = next;
        while session.has_pending_playback() {
            let (next, advanced) = session.advance();
            assert!(advanced);
            session = next;
        }
        let interaction = session.snapshot().interaction().clone();
        let state = BattleUiState::default().synced(&interaction);
        assert_eq!(state.page, BattleMenuPage::Pokemon);
        assert!(state.replacement_mode);
        let BattleInteraction::ChooseReplacement(ref prompt) = interaction else {
            panic!("the knocked out lead requires replacement");
        };
        let first = prompt
            .legal_actions()
            .iter()
            .find_map(|action| match action {
                Action::Switch(slot) => Some(slot.index()),
                _ => None,
            })
            .unwrap();
        assert_eq!(state.selected_index, first);
        let (state, outcome) =
            state.handle_key(&key(NamedKey::ArrowDown, KeyPhase::Press), &interaction);
        assert_eq!(outcome, BattleUiOutcome::Updated);
        let reset = state.synced(&BattleInteraction::ChooseAction(match BattleSession::new(
            BattleCoordinator::new(
                BattleApplication::new(
                    team("new-player", pokemon("new-own", 100, 50, 50, 40)),
                    team("new-opponent", pokemon("new-foe", 100, 50, 50, 40)),
                    1,
                )
                .unwrap(),
                FirstMove,
            ),
        )
        .snapshot()
        .interaction()
        {
            BattleInteraction::ChooseAction(prompt) => prompt.clone(),
            _ => unreachable!(),
        }));
        assert_eq!(reset.page, BattleMenuPage::Main);
        assert!(!reset.replacement_mode);
    }
}
