use battle_application::{
    Accuracy, BattleApplication, BattleStats, Move, MoveId, Pokemon, PokemonId, PokemonType,
    TEAM_SIZE, Team,
};
use battle_session::{BattleCoordinator, BattleSession, OpponentPolicy};
use game_data::CurrentDataSet;
use game_page_model::{PageIntent, PageModel, PausePageModel, demo_named};
use game_session::{GameCommand, GameSession};
use punctum_input::{KeyEvent, KeyPhase, LogicalKey, Modifiers, NamedKey, PhysicalKeyCode};
use std::time::Duration;
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

fn physical_key(code: PhysicalKeyCode, name: &str, phase: KeyPhase) -> KeyEvent {
    KeyEvent {
        physical: Some(code),
        logical: LogicalKey::Character(name.to_owned()),
        modifiers: Modifiers::default(),
        phase,
    }
}

#[test]
fn page_input_maps_keyboard_semantics_without_mouse_dependencies()
-> Result<(), Box<dyn std::error::Error>> {
    let world = demo_named("world-starting-town")
        .ok_or("world demo is missing")?
        .model()?;
    let mut ui = PageUiState::default();
    assert_eq!(
        ui.handle_key(&key(NamedKey::Tab, KeyPhase::Press), &world),
        PageUiOutcome::Intent(PageIntent::OpenPause)
    );
    assert_eq!(
        ui.handle_key(&key(NamedKey::Function(5), KeyPhase::Press), &world),
        PageUiOutcome::Intent(PageIntent::OpenSaveConfirm)
    );

    let pause = demo_named("world-pause-menu")
        .ok_or("pause demo is missing")?
        .model()?;
    assert_eq!(
        ui.handle_key(&key(NamedKey::ArrowRight, KeyPhase::Press), &pause),
        PageUiOutcome::Updated
    );
    assert_eq!(ui.focus(), PageFocus::PauseMenu(1));
    assert_eq!(
        ui.handle_key(
            &physical_key(PhysicalKeyCode::KeyZ, "z", KeyPhase::Press),
            &pause,
        ),
        PageUiOutcome::Intent(PageIntent::SelectPausePage(game_page_model::PausePage::Bag))
    );

    let bag = demo_named("bag-potion-list")
        .ok_or("bag demo is missing")?
        .model()?;
    assert!(matches!(bag, PageModel::Pause(PausePageModel::Bag(_))));
    assert_eq!(
        ui.handle_key(
            &physical_key(PhysicalKeyCode::KeyE, "e", KeyPhase::Press),
            &bag
        ),
        PageUiOutcome::Intent(PageIntent::SelectBagCategory(
            game_page_model::BagFilter::Category(game_foundation::ItemCategory::Medicine)
        ))
    );
    assert_eq!(
        ui.handle_key(
            &physical_key(PhysicalKeyCode::KeyX, "x", KeyPhase::Press),
            &bag
        ),
        PageUiOutcome::Intent(PageIntent::Close)
    );

    let pokedex = demo_named("pokedex-seen-and-unseen")
        .ok_or("pokedex demo is missing")?
        .model()?;
    let mut pokedex_ui = PageUiState::default();
    assert!(matches!(
        pokedex_ui.handle_key(&key(NamedKey::ArrowDown, KeyPhase::Press), &pokedex),
        PageUiOutcome::Intent(PageIntent::SelectPokedexEntry(_))
    ));
    let clicked = match &pokedex {
        PageModel::Pause(PausePageModel::Pokedex(page)) => {
            page.entries
                .get(10)
                .ok_or("pokedex fixture is missing entry 11")?
                .number
        }
        _ => return Err("pokedex demo did not expose entries".into()),
    };
    pokedex_ui.focus_intent(&PageIntent::SelectPokedexEntry(clicked), &pokedex);
    assert_eq!(pokedex_ui.focus(), PageFocus::Pokedex(10));
    assert!(matches!(
        pokedex_ui.handle_key(&key(NamedKey::ArrowDown, KeyPhase::Press), &pokedex),
        PageUiOutcome::Intent(PageIntent::SelectPokedexEntry(number)) if number.value() == 12
    ));
    assert!(matches!(pokedex_ui.focus(), PageFocus::Pokedex(_)));
    assert!(matches!(
        pokedex_ui.handle_key(&key(NamedKey::End, KeyPhase::Press), &pokedex),
        PageUiOutcome::Intent(PageIntent::SelectPokedexEntry(_))
    ));
    assert!(matches!(pokedex_ui.focus(), PageFocus::Pokedex(_)));
    assert!(matches!(
        pokedex_ui.handle_key(&key(NamedKey::Home, KeyPhase::Press), &pokedex),
        PageUiOutcome::Intent(PageIntent::SelectPokedexEntry(_))
    ));
    assert_eq!(
        pokedex_ui.handle_key(&key(NamedKey::ArrowRight, KeyPhase::Press), &pokedex),
        PageUiOutcome::Updated
    );
    assert_eq!(pokedex_ui.focus(), PageFocus::PokedexDetail);
    assert_eq!(
        pokedex_ui.handle_key(&key(NamedKey::ArrowRight, KeyPhase::Press), &pokedex),
        PageUiOutcome::Updated
    );
    assert_eq!(pokedex_ui.focus(), PageFocus::PokedexStats);
    assert_eq!(
        pokedex_ui.handle_key(&key(NamedKey::Enter, KeyPhase::Press), &pokedex),
        PageUiOutcome::Intent(PageIntent::TogglePokedexStatsView)
    );
    assert_eq!(
        pokedex_ui.handle_key(&key(NamedKey::ArrowRight, KeyPhase::Press), &pokedex),
        PageUiOutcome::Intent(PageIntent::SelectPokedexDetail(
            game_page_model::PokedexDetailView::Moves,
        ))
    );
    assert_eq!(pokedex_ui.focus(), PageFocus::PokedexMoves(0));
    assert_eq!(
        pokedex_ui.handle_key(&key(NamedKey::ArrowRight, KeyPhase::Press), &pokedex),
        PageUiOutcome::Ignored
    );
    assert_eq!(pokedex_ui.focus(), PageFocus::PokedexMoves(0));
    Ok(())
}

#[test]
fn pokedex_moves_left_stays_on_stats_when_legacy_detail_state_is_overview()
-> Result<(), Box<dyn std::error::Error>> {
    let pokedex = demo_named("pokedex-seen-and-unseen")
        .ok_or("pokedex demo is missing")?
        .model()?;
    let mut ui = PageUiState::default();
    let _ = ui.handle_key(&key(NamedKey::ArrowRight, KeyPhase::Press), &pokedex);
    let _ = ui.handle_key(&key(NamedKey::ArrowRight, KeyPhase::Press), &pokedex);
    let outcome = ui.handle_key(&key(NamedKey::ArrowRight, KeyPhase::Press), &pokedex);
    assert!(matches!(
        outcome,
        PageUiOutcome::Intent(PageIntent::SelectPokedexDetail(_))
    ));
    assert_eq!(ui.focus(), PageFocus::PokedexMoves(0));

    assert_eq!(
        ui.handle_key(&key(NamedKey::ArrowLeft, KeyPhase::Press), &pokedex),
        PageUiOutcome::Updated
    );
    assert_eq!(ui.focus(), PageFocus::PokedexStats);
    Ok(())
}

#[test]
fn pokedex_motion_tracks_a_new_target_from_its_current_position()
-> Result<(), Box<dyn std::error::Error>> {
    let pokedex = demo_named("pokedex-seen-and-unseen")
        .ok_or("pokedex demo is missing")?
        .model()?;
    let mut ui = PageUiState::default();
    let _ = ui.handle_key(&key(NamedKey::ArrowDown, KeyPhase::Press), &pokedex);
    let _ = ui.handle_key(&key(NamedKey::ArrowRight, KeyPhase::Press), &pokedex);
    assert_eq!(ui.pokedex_visual_state().position, 0);
    assert!(ui.advance(Duration::from_millis(50)));
    let mid = ui.pokedex_visual_state().position;
    assert!(mid > 0 && mid < 1000);

    let _ = ui.handle_key(&key(NamedKey::ArrowRight, KeyPhase::Press), &pokedex);
    for _ in 0..64 {
        if !ui.advance(Duration::from_millis(50)) {
            break;
        }
    }
    assert_eq!(ui.pokedex_visual_state().position, 2000);
    assert_eq!(ui.pokedex_section(), game_page_model::PokedexSection::Stats);
    Ok(())
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
