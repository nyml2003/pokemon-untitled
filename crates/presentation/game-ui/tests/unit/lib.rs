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
    let (fight, outcome) = fight.handle_key(&key(NamedKey::Escape, KeyPhase::Press), interaction);
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
    let mut session = BattleSession::new(BattleCoordinator::new(application, FirstMove)).unwrap();
    let action = session.legal_actions()[0];
    let (next, result) = session.submit(action);
    result.unwrap();
    session = next;
    while session.has_pending_playback() {
        let (next, advanced) = session.advance();
        assert!(advanced.unwrap());
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
    .unwrap()
    .snapshot()
    .interaction()
    {
        BattleInteraction::ChooseAction(prompt) => prompt.clone(),
        _ => unreachable!(),
    }));
    assert_eq!(reset.page, BattleMenuPage::Main);
    assert!(!reset.replacement_mode);
}
