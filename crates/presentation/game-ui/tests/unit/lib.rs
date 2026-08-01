use battle_application::{
    Accuracy, BattleApplication, BattleStats, BattleUnit, BattleUnitId, Move, MoveId, PokemonType,
    TEAM_SIZE, Team,
};
use battle_session::{BattleCoordinator, BattleSession, OpponentPolicy};
use game_data::{AbilityId, CurrentDataSet, TypeId};
use game_page_model::{
    NationalDexNumber, PageIntent, PageModel, PausePageModel, PokedexAbilityModel,
    PokedexEntryModel, PokedexMoveCategory, PokedexMoveLearnMethod, PokedexMoveModel, demo_named,
};
use game_session::{GameCommand, GameSession};
use punctum_input::{
    KeyEvent, KeyPhase, LogicalKey, Modifiers, NamedKey, PhysicalKeyCode, TextEvent,
};
use std::time::Duration;
use world_application::Direction;

use super::*;

fn key(name: NamedKey, phase: KeyPhase) -> KeyEvent {
    let physical = match name {
        NamedKey::ArrowUp => Some(PhysicalKeyCode::KeyW),
        NamedKey::ArrowDown => Some(PhysicalKeyCode::KeyS),
        NamedKey::ArrowLeft => Some(PhysicalKeyCode::KeyA),
        NamedKey::ArrowRight => Some(PhysicalKeyCode::KeyD),
        NamedKey::Enter => Some(PhysicalKeyCode::KeyJ),
        NamedKey::Escape => Some(PhysicalKeyCode::KeyK),
        _ => Some(PhysicalKeyCode::Unidentified),
    };
    KeyEvent {
        physical,
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

fn input_key() -> KeyEvent {
    KeyEvent {
        physical: None,
        logical: LogicalKey::Unidentified,
        modifiers: Modifiers::default(),
        phase: KeyPhase::Press,
    }
}

fn pokedex_entry(
    number: u16,
    known: bool,
    type_ids: Vec<TypeId>,
    height_decimeters: Option<u16>,
    weight_hectograms: Option<u16>,
    abilities: Vec<AbilityId>,
) -> Result<PokedexEntryModel, Box<dyn std::error::Error>> {
    Ok(PokedexEntryModel {
        number: NationalDexNumber::new(number)?,
        name: known.then(|| String::from("测试精灵")),
        stats: None,
        types: Vec::new(),
        type_ids,
        genus: None,
        height_decimeters,
        weight_hectograms,
        abilities: abilities
            .into_iter()
            .map(|id| PokedexAbilityModel {
                id,
                name: format!("特性{}", id.0),
                hidden: false,
            })
            .collect(),
        known,
    })
}

fn pokedex_move(
    name: &str,
    type_id: TypeId,
    category: PokedexMoveCategory,
    power: Option<u16>,
    accuracy: Option<u8>,
    priority: i8,
) -> PokedexMoveModel {
    PokedexMoveModel {
        name: String::from(name),
        move_type: String::from("测试属性"),
        type_id,
        category,
        power,
        accuracy,
        pp: Some(10),
        priority,
        method: PokedexMoveLearnMethod::LevelUp,
        level: Some(1),
    }
}

#[test]
fn pokedex_filter_requires_known_facts_and_respects_all_constraints()
-> Result<(), Box<dyn std::error::Error>> {
    let grass = TypeId(12);
    let poison = TypeId(4);
    let overgrow = AbilityId(65);
    let known = pokedex_entry(
        1,
        true,
        vec![grass, poison],
        Some(7),
        Some(69),
        vec![overgrow],
    )?;
    let unknown = pokedex_entry(
        4,
        false,
        vec![grass, poison],
        Some(7),
        Some(69),
        vec![overgrow],
    )?;

    let mut filter = PokedexFilterModel::default();
    filter.type_ids = [grass, poison].into_iter().collect();
    filter.type_match = TypeMatch::All;
    filter.generations = [1].into_iter().collect();
    filter.height_decimeters = (Some(7), Some(7));
    filter.weight_hectograms = (Some(69), Some(69));
    filter.ability = Some(overgrow);
    assert!(filter.matches(&known));
    assert!(!filter.matches(&unknown));

    filter.type_match = TypeMatch::Any;
    filter.type_ids = [TypeId(10), poison].into_iter().collect();
    assert!(filter.matches(&known));

    filter.generations = [2].into_iter().collect();
    assert!(!filter.matches(&known));
    Ok(())
}

#[test]
fn pokedex_filter_does_not_match_missing_range_values() -> Result<(), Box<dyn std::error::Error>> {
    let entry = pokedex_entry(152, true, vec![TypeId(12)], None, Some(69), Vec::new())?;
    let mut filter = PokedexFilterModel::default();
    filter.height_decimeters = (Some(1), Some(10));
    assert!(!filter.matches(&entry));
    Ok(())
}

#[test]
fn move_filter_distinguishes_guaranteed_hit_and_priority() -> Result<(), Box<dyn std::error::Error>>
{
    let electric = TypeId(13);
    let guaranteed = pokedex_move(
        "电磁波",
        electric,
        PokedexMoveCategory::Status,
        None,
        None,
        0,
    );
    let priority = pokedex_move(
        "先制电击",
        electric,
        PokedexMoveCategory::Special,
        Some(70),
        Some(100),
        1,
    );
    let mut filter = MoveFilterModel::default();
    filter.name_query = String::from("电");
    filter.type_ids = [electric].into_iter().collect();
    filter.accuracy = Some(None);
    assert!(filter.matches(&guaranteed));
    assert!(!filter.matches(&priority));

    let mut priority_filter = filter;
    priority_filter.accuracy = Some(Some(100));
    priority_filter.priority_only = true;
    assert!(!priority_filter.matches(&guaranteed));
    assert!(priority_filter.matches(&priority));
    Ok(())
}

#[test]
fn pokedex_form_uses_select_text_events_and_debounced_ranges()
-> Result<(), Box<dyn std::error::Error>> {
    let pokedex = demo_named("pokedex-seen-and-unseen")
        .ok_or("pokedex demo is missing")?
        .model()?;
    let mut ui = PageUiState::default();
    assert_eq!(
        ui.handle_input(
            &physical_key(PhysicalKeyCode::KeyL, "l", KeyPhase::Press),
            None,
            &pokedex
        ),
        PageUiOutcome::Updated
    );
    assert!(matches!(
        ui.pokedex_visual_state().filter_overlay,
        PokedexFilterOverlay::Pokedex(_)
    ));
    assert_eq!(
        ui.handle_input(
            &physical_key(PhysicalKeyCode::KeyL, "l", KeyPhase::Release),
            None,
            &pokedex
        ),
        PageUiOutcome::Ignored
    );
    assert!(matches!(
        ui.pokedex_visual_state().filter_overlay,
        PokedexFilterOverlay::Pokedex(_)
    ));

    for _ in 0..3 {
        assert_eq!(
            ui.handle_input(
                &physical_key(PhysicalKeyCode::KeyE, "e", KeyPhase::Press),
                None,
                &pokedex
            ),
            PageUiOutcome::Updated
        );
    }
    let text = TextEvent::new("0.7")?;
    assert_eq!(
        ui.handle_input(&input_key(), Some(&text), &pokedex),
        PageUiOutcome::Updated
    );
    assert_eq!(
        ui.pokedex_visual_state().pokedex_filter.height_decimeters,
        (None, None)
    );
    assert!(ui.advance(Duration::from_millis(300)));
    ui.sync(&pokedex);
    assert_eq!(
        ui.pokedex_visual_state().pokedex_filter.height_decimeters,
        (Some(7), None)
    );

    assert_eq!(
        ui.handle_input(&key(NamedKey::Escape, KeyPhase::Press), None, &pokedex),
        PageUiOutcome::Updated
    );
    assert_eq!(
        ui.handle_input(&key(NamedKey::Escape, KeyPhase::Press), None, &pokedex),
        PageUiOutcome::Updated
    );
    assert!(matches!(
        ui.pokedex_visual_state().filter_overlay,
        PokedexFilterOverlay::Compact
    ));
    Ok(())
}

#[test]
fn default_pokedex_filter_keeps_unknown_entries_visible() -> Result<(), Box<dyn std::error::Error>>
{
    let pokedex = demo_named("pokedex-seen-and-unseen")
        .ok_or("pokedex demo is missing")?
        .model()?;
    let mut ui = PageUiState::default();
    ui.sync(&pokedex);
    let PageModel::Pause(PausePageModel::Pokedex(page)) = &pokedex else {
        return Err("pokedex demo did not expose a pokedex page".into());
    };
    assert_eq!(
        ui.pokedex_visual_state().visible_entry_indices.len(),
        page.entries.len()
    );
    Ok(())
}

#[test]
fn pokedex_ability_select_filters_text_events_and_commits_keyboard_choice()
-> Result<(), Box<dyn std::error::Error>> {
    let pokedex = demo_named("pokedex-seen-and-unseen")
        .ok_or("pokedex demo is missing")?
        .model()?;
    let mut ui = PageUiState::default();
    assert_eq!(
        ui.handle_input(
            &physical_key(PhysicalKeyCode::KeyL, "l", KeyPhase::Press),
            None,
            &pokedex
        ),
        PageUiOutcome::Updated
    );
    for _ in 0..7 {
        assert_eq!(
            ui.handle_input(
                &physical_key(PhysicalKeyCode::KeyE, "e", KeyPhase::Press),
                None,
                &pokedex
            ),
            PageUiOutcome::Updated
        );
    }
    assert_eq!(
        ui.handle_view_intent(&PageIntent::TogglePokedexAbilitySelect, &pokedex),
        Some(PageUiOutcome::Updated)
    );
    let query = TextEvent::new("特")?;
    assert_eq!(
        ui.handle_input(&input_key(), Some(&query), &pokedex),
        PageUiOutcome::Updated
    );
    assert_eq!(
        ui.pokedex_visual_state().pokedex_filter.ability_query(),
        "特"
    );
    assert_eq!(
        ui.handle_input(&key(NamedKey::Escape, KeyPhase::Press), None, &pokedex),
        PageUiOutcome::Updated
    );
    assert!(
        ui.pokedex_visual_state()
            .pokedex_filter
            .ability_query()
            .is_empty()
    );
    assert_eq!(
        ui.handle_input(&key(NamedKey::ArrowDown, KeyPhase::Press), None, &pokedex),
        PageUiOutcome::Updated
    );
    assert!(matches!(
        ui.handle_input(&key(NamedKey::Enter, KeyPhase::Press), None, &pokedex),
        PageUiOutcome::Updated | PageUiOutcome::Intent(_)
    ));
    assert!(ui.pokedex_visual_state().pokedex_filter.ability.is_some());
    Ok(())
}

#[test]
fn page_input_maps_keyboard_semantics_without_mouse_dependencies()
-> Result<(), Box<dyn std::error::Error>> {
    let world = demo_named("world-starting-town")
        .ok_or("world demo is missing")?
        .model()?;
    let mut ui = PageUiState::default();
    assert_eq!(
        ui.handle_key(
            &physical_key(PhysicalKeyCode::KeyR, "r", KeyPhase::Press),
            &world
        ),
        PageUiOutcome::Intent(PageIntent::OpenPause)
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
            &physical_key(PhysicalKeyCode::KeyJ, "j", KeyPhase::Press),
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
            &physical_key(PhysicalKeyCode::KeyK, "k", KeyPhase::Press),
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
    assert_eq!(pokedex_ui.focus(), PageFocus::PokedexBrowse(10));
    assert!(matches!(
        pokedex_ui.handle_key(&key(NamedKey::ArrowDown, KeyPhase::Press), &pokedex),
        PageUiOutcome::Intent(PageIntent::SelectPokedexEntry(number)) if number.value() == 12
    ));
    assert!(matches!(pokedex_ui.focus(), PageFocus::PokedexBrowse(_)));
    assert_eq!(
        pokedex_ui.handle_key(&key(NamedKey::ArrowRight, KeyPhase::Press), &pokedex),
        PageUiOutcome::Updated
    );
    assert_eq!(pokedex_ui.focus(), PageFocus::PokedexDetailFacts);
    assert_eq!(
        pokedex_ui.handle_key(&key(NamedKey::Enter, KeyPhase::Press), &pokedex),
        PageUiOutcome::Ignored
    );
    assert_eq!(
        pokedex_ui.handle_key(&key(NamedKey::ArrowRight, KeyPhase::Press), &pokedex),
        PageUiOutcome::Updated
    );
    assert_eq!(pokedex_ui.focus(), PageFocus::PokedexDetailMoves(0));
    assert_eq!(
        pokedex_ui.handle_key(&key(NamedKey::ArrowRight, KeyPhase::Press), &pokedex),
        PageUiOutcome::Ignored
    );
    assert_eq!(pokedex_ui.focus(), PageFocus::PokedexDetailMoves(0));
    Ok(())
}

#[test]
fn pokedex_moves_left_returns_to_profile() -> Result<(), Box<dyn std::error::Error>> {
    let pokedex = demo_named("pokedex-seen-and-unseen")
        .ok_or("pokedex demo is missing")?
        .model()?;
    let mut ui = PageUiState::default();
    let _ = ui.handle_key(&key(NamedKey::ArrowRight, KeyPhase::Press), &pokedex);
    let outcome = ui.handle_key(&key(NamedKey::ArrowRight, KeyPhase::Press), &pokedex);
    assert_eq!(outcome, PageUiOutcome::Updated);
    assert_eq!(ui.focus(), PageFocus::PokedexDetailMoves(0));

    assert_eq!(
        ui.handle_key(&key(NamedKey::ArrowLeft, KeyPhase::Press), &pokedex),
        PageUiOutcome::Updated
    );
    assert_eq!(ui.focus(), PageFocus::PokedexDetailFacts);
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
    assert_eq!(ui.focus(), PageFocus::PokedexBrowse(1));
    assert!(ui.advance(Duration::from_millis(50)));
    assert!(ui.pokedex_visual_state().wheel_position > 0);
    let _ = ui.handle_key(&key(NamedKey::ArrowRight, KeyPhase::Press), &pokedex);
    assert_eq!(ui.pokedex_visual_state().scene_position, 0);
    assert!(ui.advance(Duration::from_millis(50)));
    let mid = ui.pokedex_visual_state().scene_position;
    assert!(mid > 0 && mid < 1000);
    assert!(mid < 200, "transition should ease into the first frame");

    let _ = ui.handle_key(&key(NamedKey::ArrowRight, KeyPhase::Press), &pokedex);
    for _ in 0..64 {
        if !ui.advance(Duration::from_millis(50)) {
            break;
        }
    }
    assert_eq!(ui.pokedex_visual_state().scene_position, 1000);
    assert_eq!(ui.pokedex_scene(), PokedexScene::Detail);
    assert_eq!(ui.pokedex_detail_mode(), PokedexDetailMode::Moves);
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
        vec![PokemonType::Normal],
        power,
        Accuracy::AlwaysHit,
        20,
        20,
        0,
    )
    .unwrap()
}

fn pokemon(name: &str, hp: u32, attack: u16, speed: u16, power: u16) -> BattleUnit {
    let species = battle_application::Species::new(
        name,
        battle_application::StatBlock::new(45, 49, 49, 65, 65, 45),
        battle_application::NationalDexId::new(1),
        battle_application::FormId::new(0),
        vec![PokemonType::Normal],
        vec![],
    )
    .unwrap();
    let state = battle_application::BattleState::new(
        50,
        BattleStats::new(attack, 50, attack, 50, speed).unwrap(),
        hp,
        hp,
        vec![battle_move(&format!("{name}-move"), power)],
        vec![],
        None,
        battle_application::StatStages::neutral(),
    )
    .unwrap();
    BattleUnit::new(species, BattleUnitId::new(name).unwrap(), state).unwrap()
}

fn team(prefix: &str, lead: BattleUnit) -> Team {
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
