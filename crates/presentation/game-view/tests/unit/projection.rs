use battle_application::{
    Accuracy, BattleApplication, BattleStats, Move, MoveCategory, MoveId, Pokemon, PokemonId,
    PokemonType, TEAM_SIZE, Team,
};
use battle_session::{
    Action, BattleCoordinator, BattleObservation, BattleSession, BattleSessionSnapshot,
    OpponentPolicy,
};
use game_data::PokedexData;
use map_project::{
    AtomicTileId, CompositeTile, CompositeTileId, MapActor, MapActorId, MapDirection, MapProject,
    MapProjectId, TilePosition,
};
use punctum_grid::{GridPos, GridSize};
use punctum_input::{KeyEvent, KeyPhase, LogicalKey, Modifiers, NamedKey, PhysicalKeyCode};
use world_application::{CharacterAppearanceId, Direction, Position, WorldApplication};

use game_ui::{
    BattleMenuPage, BattleUiOutcome, BattleUiState, CommandConsoleView, PokedexAction,
    WorldAnimation,
};

use super::{
    BattleSpriteResources, LayerKind, TextRole, ViewCell, ViewLayer, compose_world,
    move_category_icon_asset, pill_ui_asset, pokemon_icon_asset, project_battle, project_battle_ui,
    project_console, project_console_ui, project_pokedex, project_world, rounded_ui_asset,
    type_icon_asset, world_character_asset,
};

#[test]
fn pokedex_projects_its_selected_canonical_front() {
    let data = PokedexData::embedded_gen3().unwrap();
    let tree = project_pokedex(&data, 0).unwrap();
    for viewport in [
        punctum_ui::UiSize::new(960, 720),
        punctum_ui::UiSize::new(640, 480),
        punctum_ui::UiSize::new(320, 240),
    ] {
        assert!(tree.resolve(viewport).is_ok());
    }
    let frame = tree.resolve(punctum_ui::UiSize::new(960, 720)).unwrap();
    assert!(frame.commands().iter().any(|command| matches!(command,
                punctum_ui::UiDrawCommand::Text { content, .. } if content == "妙蛙种子")));
    assert!(
            frame.commands().iter().any(|command| matches!(command,
                punctum_ui::UiDrawCommand::Image { content, .. } if content.as_str() == "pokedex/1"))
        );
    assert!(frame.commands().iter().any(|command| matches!(
            command,
            punctum_ui::UiDrawCommand::Image { content, border_radius, .. }
                if content.as_str() == "pokedex/1" && *border_radius == punctum_ui::UiBorderRadius::all(12)
        )));
    assert!(frame.commands().iter().any(|command| matches!(
        command,
        punctum_ui::UiDrawCommand::Fill { color, border_radius, .. }
            if *color == super::UiColor::new(237, 242, 233, 255)
                && *border_radius == punctum_ui::UiBorderRadius::all(16)
    )));
    assert!(
            frame.commands().iter().any(|command| matches!(command,
                punctum_ui::UiDrawCommand::Image { content, .. } if content.as_str() == type_icon_asset(PokemonType::Grass).as_str()))
        );
    assert_eq!(frame.action_hits().len(), 5);
    assert_eq!(tree.root().id, punctum_ui::UiId(0));
    assert_eq!(frame.action_hits().len(), 5);
    assert_eq!(
        frame.action_hits()[0].action,
        PokedexAction::SelectEntry { index: 0 }
    );
    assert_eq!(
        frame.action_hits()[0]
            .key
            .as_ref()
            .map(punctum_ui::UiKey::as_str),
        Some("pokedex-entry-1")
    );
}

#[test]
fn battle_pixel_ui_uses_flex_and_keeps_move_metadata_visible() {
    let snapshot = battle_fixture();
    let sprites = BattleSpriteResources::for_slots(0, 0);
    let main = project_battle_ui(&snapshot, BattleUiState::default(), sprites.clone(), 0)
        .unwrap()
        .resolve(punctum_ui::UiSize::new(1000, 720))
        .unwrap();
    assert_eq!(main.action_hits().len(), 4);
    assert!(
        main.action_hits()
            .iter()
            .all(|region| region.bounds.width > 0 && region.bounds.height > 0)
    );
    assert!(main.commands().iter().any(|command| matches!(
        command,
        punctum_ui::UiDrawCommand::Text { content, .. } if content == "战斗"
    )));

    let mut ui = BattleUiState::default();
    handle_battle_key(&mut ui, &key(NamedKey::Enter), snapshot.interaction());
    let fight = project_battle_ui(&snapshot, ui, sprites, 0)
        .unwrap()
        .resolve(punctum_ui::UiSize::new(1000, 720))
        .unwrap();
    assert!(fight.commands().iter().any(|command| matches!(
        command,
        punctum_ui::UiDrawCommand::Text { content, .. } if content == "威40 PP35/35"
    )));
    let images = fight
        .commands()
        .iter()
        .filter_map(|command| match command {
            punctum_ui::UiDrawCommand::Image { content, .. } => Some(content.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(images.contains(&type_icon_asset(PokemonType::Grass).as_str()));
    assert!(images.contains(&move_category_icon_asset(MoveCategory::Special).as_str()));
}

#[test]
fn pokemon_selection_pixel_ui_has_a_detail_panel_and_all_team_members() {
    let snapshot = battle_fixture();
    let mut ui = BattleUiState::default();
    handle_battle_key(&mut ui, &key(NamedKey::ArrowRight), snapshot.interaction());
    handle_battle_key(&mut ui, &key(NamedKey::Enter), snapshot.interaction());

    let frame = project_battle_ui(&snapshot, ui, BattleSpriteResources::for_slots(0, 0), 1)
        .unwrap()
        .resolve(punctum_ui::UiSize::new(1000, 720))
        .unwrap();
    assert!(frame.commands().iter().any(|command| matches!(
        command,
        punctum_ui::UiDrawCommand::Text { content, .. } if content == "选择宝可梦"
    )));
    assert_eq!(frame.action_hits().len(), TEAM_SIZE);
    let images = frame
        .commands()
        .iter()
        .filter_map(|command| match command {
            punctum_ui::UiDrawCommand::Image { content, .. } => Some(content.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    for slot in 0..TEAM_SIZE {
        assert!(images.contains(&pokemon_icon_asset(slot, 1).as_str()));
    }
    assert!(images.contains(&type_icon_asset(PokemonType::Poison).as_str()));
}

#[test]
fn console_pixel_ui_preserves_the_legacy_overlay_without_an_opaque_root() {
    let console = CommandConsoleView {
        query: "gi".to_owned(),
        preedit: "t".to_owned(),
        items: vec!["give potion".to_owned(), "goto town".to_owned()],
        selected_index: Some(1),
        diagnostic: Some("invalid target".to_owned()),
    };
    let frame = project_console_ui(&console)
        .unwrap()
        .resolve(punctum_ui::UiSize::new(1000, 720))
        .unwrap();
    assert!(!frame.commands().iter().any(|command| matches!(
        command,
        punctum_ui::UiDrawCommand::Fill {
            bounds,
            color,
            ..
        } if *bounds == punctum_ui::UiRect::new(0, 0, 1000, 720)
            && color.alpha == 255
    )));
    let actual_labels = frame
        .commands()
        .iter()
        .filter_map(|command| match command {
            punctum_ui::UiDrawCommand::Text { content, .. } => Some(content.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actual_labels,
        ["> git", "give potion", "goto town", "invalid target"]
    );
}

fn key(name: NamedKey) -> KeyEvent {
    KeyEvent {
        physical: Some(PhysicalKeyCode::Unidentified),
        logical: LogicalKey::Named(name),
        modifiers: Modifiers::default(),
        phase: KeyPhase::Press,
    }
}

fn handle_battle_key(
    ui: &mut BattleUiState,
    key: &KeyEvent,
    interaction: &battle_session::BattleInteraction,
) -> BattleUiOutcome {
    let (next, outcome) = (*ui).handle_key(key, interaction);
    *ui = next;
    outcome
}

struct FirstActionPolicy;

impl OpponentPolicy for FirstActionPolicy {
    fn choose_action(
        &self,
        _observation: &BattleObservation,
        legal_actions: &[Action],
    ) -> Option<Action> {
        legal_actions.first().copied()
    }
}

fn battle_session_with_move(
    current_pp: u8,
    accuracy: Accuracy,
) -> BattleSession<FirstActionPolicy> {
    battle_session_with_team_state(current_pp, accuracy, false)
}

fn battle_session_with_team_state(
    current_pp: u8,
    accuracy: Accuracy,
    fainted_own_bench: bool,
) -> BattleSession<FirstActionPolicy> {
    fn team(
        prefix: &str,
        move_type: PokemonType,
        current_pp: u8,
        accuracy: Accuracy,
        fainted_bench: bool,
    ) -> Team {
        let members = (0..TEAM_SIZE)
            .map(|index| {
                let battle_move = Move::new(
                    MoveId::new(format!("{prefix}-move-{index}")).unwrap(),
                    if index == 0 { "撞击" } else { "电光一闪" },
                    move_type,
                    40,
                    accuracy,
                    35,
                    current_pp,
                    0,
                )
                .unwrap();
                let alternate_move = Move::new(
                    MoveId::new(format!("{prefix}-alternate-{index}")).unwrap(),
                    "飞叶快刀",
                    move_type,
                    55,
                    accuracy,
                    25,
                    current_pp.min(25),
                    0,
                )
                .unwrap();
                let current_hp = if fainted_bench && index == 1 {
                    0
                } else {
                    80 + index as u32
                };
                Pokemon::new(
                    PokemonId::new(format!("{prefix}-{index}")).unwrap(),
                    format!("{prefix}{index}"),
                    24,
                    move_type,
                    (index == 0).then_some(PokemonType::Poison),
                    80 + index as u32,
                    current_hp,
                    BattleStats::new(50, 50, 50, 50, 50).unwrap(),
                    vec![battle_move, alternate_move],
                )
                .unwrap()
            })
            .collect();
        Team::new(members).unwrap()
    }

    let application = BattleApplication::new(
        team(
            "己方",
            PokemonType::Grass,
            current_pp,
            accuracy,
            fainted_own_bench,
        ),
        team("对手", PokemonType::Fire, current_pp, accuracy, false),
        42,
    )
    .unwrap();
    BattleSession::new(BattleCoordinator::new(application, FirstActionPolicy)).unwrap()
}

fn battle_session_fixture() -> BattleSession<FirstActionPolicy> {
    battle_session_with_move(35, Accuracy::AlwaysHit)
}

fn battle_fixture() -> BattleSessionSnapshot {
    battle_session_fixture().snapshot()
}

#[test]
fn main_menu_routes_to_fight_pokemon_bag_and_run() {
    let snapshot = battle_fixture();
    let interaction = snapshot.interaction();
    let mut ui = BattleUiState::default();

    assert_eq!(
        handle_battle_key(&mut ui, &key(NamedKey::Enter), interaction),
        BattleUiOutcome::Updated
    );
    assert_eq!(ui.view().0, BattleMenuPage::Fight);
    assert_eq!(
        handle_battle_key(&mut ui, &key(NamedKey::Enter), interaction),
        BattleUiOutcome::Submit(Action::UseMove(battle_session::MoveSlot::new(0).unwrap()))
    );

    ui = BattleUiState::default();
    assert_eq!(
        handle_battle_key(&mut ui, &key(NamedKey::ArrowRight), interaction),
        BattleUiOutcome::Updated
    );
    assert_eq!(
        handle_battle_key(&mut ui, &key(NamedKey::Enter), interaction),
        BattleUiOutcome::Updated
    );
    assert_eq!(ui.view().0, BattleMenuPage::Pokemon);
    assert_eq!(
        handle_battle_key(&mut ui, &key(NamedKey::Enter), interaction),
        BattleUiOutcome::Updated
    );
    handle_battle_key(&mut ui, &key(NamedKey::ArrowRight), interaction);
    assert_eq!(
        handle_battle_key(&mut ui, &key(NamedKey::Enter), interaction),
        BattleUiOutcome::Submit(Action::Switch(battle_session::TeamSlot::new(1).unwrap()))
    );

    ui = BattleUiState::default();
    handle_battle_key(&mut ui, &key(NamedKey::ArrowRight), interaction);
    handle_battle_key(&mut ui, &key(NamedKey::ArrowRight), interaction);
    assert_eq!(
        handle_battle_key(&mut ui, &key(NamedKey::Enter), interaction),
        BattleUiOutcome::Updated
    );
    assert_eq!(ui.view().0, BattleMenuPage::Main);

    ui = BattleUiState::default();
    handle_battle_key(&mut ui, &key(NamedKey::ArrowLeft), interaction);
    assert_eq!(ui.view().1, 3);
    assert_eq!(
        handle_battle_key(&mut ui, &key(NamedKey::Enter), interaction),
        BattleUiOutcome::Submit(Action::Run)
    );
}

#[test]
fn battle_projection_shows_status_and_move_details() {
    let snapshot = battle_fixture();
    let mut ui = BattleUiState::default();
    let main = project_battle(&snapshot, ui, BattleSpriteResources::for_slots(0, 0), 0).unwrap();
    let commands = main
        .labels()
        .filter(|label| matches!(label.role, TextRole::Action(_)))
        .map(|label| label.content.as_str())
        .collect::<Vec<_>>();
    assert_eq!(commands, ["战斗", "宝可梦", "包包", "逃走"]);
    assert!(
        main.labels()
            .any(|label| { label.role == TextRole::PlayerDetail && label.content == "Lv.24" })
    );
    assert!(
        main.images()
            .any(|image| image.asset == type_icon_asset(PokemonType::Grass))
    );
    assert!(
        main.images()
            .any(|image| image.asset == type_icon_asset(PokemonType::Poison))
    );
    assert!(
        main.labels()
            .any(|label| { label.role == TextRole::PlayerHp && label.content == "HP 80/80" })
    );

    handle_battle_key(&mut ui, &key(NamedKey::Enter), snapshot.interaction());
    let fight = project_battle(&snapshot, ui, BattleSpriteResources::for_slots(0, 0), 0).unwrap();
    assert!(fight.labels().any(|label| {
        label.role == TextRole::ActionDetail(0) && label.content == "威40 PP35/35"
    }));
    assert!(fight.labels().any(|label| {
        label.role == TextRole::Action(1)
            && label.content == "飞叶快刀"
            && label.color == super::TEXT
    }));
    assert!(fight.images().any(|image| {
        image.asset == type_icon_asset(PokemonType::Grass)
            && image.bounds.origin == GridPos::new(23, 18)
    }));
    assert!(fight.images().any(|image| {
        image.asset == move_category_icon_asset(MoveCategory::Special)
            && image.bounds.origin == GridPos::new(26, 18)
    }));
}

#[test]
fn pokemon_selection_uses_animated_team_icons() {
    let snapshot = battle_fixture();
    let mut ui = BattleUiState::default();
    handle_battle_key(&mut ui, &key(NamedKey::ArrowRight), snapshot.interaction());
    handle_battle_key(&mut ui, &key(NamedKey::Enter), snapshot.interaction());

    let view = project_battle(&snapshot, ui, BattleSpriteResources::for_slots(0, 0), 0).unwrap();

    assert!(
        view.labels().any(|label| {
            label.role == TextRole::PageTitle && label.content == "选择宝可梦"
        })
    );
    assert!(view.labels().any(|label| {
        label.role == TextRole::SelectedMemberDetail && label.content == "Lv.24  出战"
    }));
    assert_eq!(
        view.labels()
            .filter(|label| matches!(label.role, TextRole::TeamMember(_)))
            .count(),
        TEAM_SIZE
    );
    assert_eq!(
        view.images()
            .filter(|image| image.asset.as_str().starts_with("battle/team/"))
            .count(),
        TEAM_SIZE + 1
    );
    assert!(view.images().any(|image| {
        image.asset == rounded_ui_asset()
            && image.bounds.origin == GridPos::new(1, 4)
            && image.bounds.size == GridSize::new(11, 17)
    }));
    assert!(view.images().any(|image| {
        image.asset == rounded_ui_asset()
            && image.bounds.origin == GridPos::new(13, 4)
            && image.bounds.size == GridSize::new(18, 3)
    }));
    for slot in 0..TEAM_SIZE {
        assert!(
            view.images()
                .any(|image| image.asset == super::pokemon_icon_asset(slot, 0))
        );
    }
    assert!(view.images().any(|image| {
        image.asset == type_icon_asset(PokemonType::Poison)
            && image.bounds.origin == GridPos::new(5, 16)
    }));

    let animated =
        project_battle(&snapshot, ui, BattleSpriteResources::for_slots(0, 0), 1).unwrap();
    assert!(
        animated
            .images()
            .any(|image| image.asset == super::pokemon_icon_asset(0, 1))
    );
    assert!(view.images().any(|image| {
        image.asset == super::pokemon_icon_asset(0, 0)
            && image.bounds.origin == GridPos::new(3, 5)
            && image.bounds.size == GridSize::new(7, 7)
    }));
    assert!(view.images().any(|image| {
        image.asset == super::pokemon_icon_asset(0, 0)
            && image.bounds.origin == GridPos::new(14, 4)
            && image.bounds.size == GridSize::new(3, 3)
    }));
}

#[test]
fn selected_fainted_pokemon_uses_the_unavailable_status() {
    let snapshot = battle_session_with_team_state(35, Accuracy::AlwaysHit, true).snapshot();
    let mut ui = BattleUiState::default();
    handle_battle_key(&mut ui, &key(NamedKey::ArrowRight), snapshot.interaction());
    handle_battle_key(&mut ui, &key(NamedKey::Enter), snapshot.interaction());
    handle_battle_key(&mut ui, &key(NamedKey::ArrowRight), snapshot.interaction());

    let view = project_battle(&snapshot, ui, BattleSpriteResources::for_slots(0, 0), 0).unwrap();
    assert!(view.labels().any(|label| {
        label.role == TextRole::SelectedMemberHp
            && label.content == "无法战斗"
            && label.color == super::HP_LOW
    }));
}

#[test]
fn zero_width_hp_bar_draws_nothing() {
    let mut images = Vec::new();
    super::draw_hp_bar(&mut images, 0, 0, 0, 1, 1);
    assert!(images.is_empty());
}

#[test]
fn product_assets_use_stable_semantic_keys() {
    assert_eq!(rounded_ui_asset().as_str(), "ui/rounded-rect");
    assert_eq!(pill_ui_asset().as_str(), "ui/pill");
    let types = [
        PokemonType::Normal,
        PokemonType::Fighting,
        PokemonType::Flying,
        PokemonType::Poison,
        PokemonType::Ground,
        PokemonType::Rock,
        PokemonType::Bug,
        PokemonType::Ghost,
        PokemonType::Steel,
        PokemonType::Fire,
        PokemonType::Water,
        PokemonType::Grass,
        PokemonType::Electric,
        PokemonType::Psychic,
        PokemonType::Ice,
        PokemonType::Dragon,
        PokemonType::Dark,
    ];
    for pokemon_type in types {
        assert!(
            type_icon_asset(pokemon_type)
                .as_str()
                .starts_with("ui/battle/type/")
        );
    }
    for category in [
        MoveCategory::Physical,
        MoveCategory::Special,
        MoveCategory::Status,
    ] {
        assert!(
            move_category_icon_asset(category)
                .as_str()
                .starts_with("ui/battle/move-category/")
        );
    }
    for direction in [
        Direction::Down,
        Direction::Left,
        Direction::Right,
        Direction::Up,
    ] {
        let appearance = CharacterAppearanceId::new("red").unwrap();
        for animation in [
            WorldAnimation::Stand,
            WorldAnimation::Walk,
            WorldAnimation::Run,
            WorldAnimation::RunStopping,
        ] {
            for frame in 0..4 {
                assert!(
                    world_character_asset(&appearance, direction, animation, frame)
                        .as_str()
                        .starts_with("character/")
                );
            }
        }
    }
}

#[test]
fn message_and_tint_tables_cover_every_semantic_variant() {
    use battle_session::{ObservedBattleOutcome, Participant, TypeEffectiveness, UsedMove};

    for outcome in [
        ObservedBattleOutcome::Winner(Participant::Own),
        ObservedBattleOutcome::Winner(Participant::Opponent),
        ObservedBattleOutcome::Escaped(Participant::Own),
        ObservedBattleOutcome::Escaped(Participant::Opponent),
        ObservedBattleOutcome::Draw,
    ] {
        assert!(!super::outcome_message(outcome).is_empty());
    }
    for effectiveness in [
        TypeEffectiveness::Immune,
        TypeEffectiveness::Quarter,
        TypeEffectiveness::Half,
        TypeEffectiveness::Normal,
        TypeEffectiveness::Double,
        TypeEffectiveness::Quadruple,
    ] {
        assert!(!super::effectiveness_message(effectiveness).is_empty());
    }
    assert_eq!(super::used_move_name(&UsedMove::Struggle), "挣扎");
    let used = UsedMove::Move {
        id: MoveId::new("move").unwrap(),
        name: "招式".into(),
    };
    assert_eq!(super::used_move_name(&used), "招式");
    for animation in [
        super::BattleAnimation::Idle,
        super::BattleAnimation::Acting(Participant::Own),
        super::BattleAnimation::Acting(Participant::Opponent),
        super::BattleAnimation::Hit(Participant::Own),
        super::BattleAnimation::Fainted(Participant::Opponent),
    ] {
        for participant in [Participant::Own, Participant::Opponent] {
            assert_eq!(super::creature_tint(animation, participant).alpha, 255);
        }
    }
}

#[test]
fn a_complete_battle_story_projects_every_reachable_cue() {
    let mut battle = battle_session_fixture();
    let mut ui = BattleUiState::default();
    let mut projected = 0;
    for _ in 0..2_000 {
        let snapshot = battle.snapshot();
        ui = ui.synced(snapshot.interaction());
        let view = project_battle(
            &snapshot,
            ui,
            BattleSpriteResources::for_slots(0, 0),
            projected,
        )
        .unwrap();
        assert_eq!(view.layers()[0].kind, LayerKind::Map);
        projected += 1;
        if battle.is_finished() {
            break;
        }
        if battle.has_pending_playback() {
            let (next, advanced) = battle.advance();
            battle = next;
            assert!(advanced.unwrap());
        } else {
            let action = battle.legal_actions()[0];
            let (next, result) = battle.submit(action);
            battle = next;
            result.unwrap();
        }
    }
    assert!(battle.is_finished());
    assert!(projected > 20);
}

#[test]
fn exhausted_moves_and_misses_have_complete_battle_views() {
    let struggle = battle_session_with_move(0, Accuracy::AlwaysHit).snapshot();
    let mut ui = BattleUiState::default();
    handle_battle_key(&mut ui, &key(NamedKey::Enter), struggle.interaction());
    let view = project_battle(&struggle, ui, BattleSpriteResources::for_slots(0, 0), 0).unwrap();
    assert!(view.labels().any(|label| label.content == "挣扎"));

    let mut battle = battle_session_with_move(35, Accuracy::percent(1).unwrap());
    let action = battle.legal_actions()[0];
    let (next, result) = battle.submit(action);
    battle = next;
    result.unwrap();
    let mut saw_miss = false;
    while battle.has_pending_playback() {
        (battle, _) = battle.advance();
        let snapshot = battle.snapshot();
        if matches!(
            snapshot.cue(),
            Some(battle_session::BattleCue::Missed { .. })
        ) {
            let view = project_battle(
                &snapshot,
                BattleUiState::default(),
                BattleSpriteResources::for_slots(0, 0),
                0,
            )
            .unwrap();
            assert!(view.labels().any(|label| label.content == "攻击没有命中。"));
            saw_miss = true;
        }
    }
    assert!(saw_miss);
}

#[test]
fn world_projection_uses_stable_character_keys_and_explicit_layers() {
    let world = WorldApplication::demo().unwrap();
    let observation = world.observe().unwrap();
    let appearance = observation.actors()[0].appearance();
    let view = project_world(&observation).unwrap();

    assert_eq!(world.observe().unwrap().player(), Position::new(3, 6));
    assert_eq!(
        view.layers()
            .iter()
            .map(|layer| layer.kind)
            .collect::<Vec<_>>(),
        [LayerKind::Map, LayerKind::Character, LayerKind::Hud]
    );
    assert_eq!(
        view.layers()[0].surface.as_ref().unwrap().size(),
        GridSize::new(32, 24)
    );
    assert_eq!(view.images().count(), 1);
    assert_eq!(
        view.images().next().unwrap().asset,
        world_character_asset(appearance, Direction::Down, WorldAnimation::Stand, 0)
    );
    assert_eq!(
        world_character_asset(appearance, Direction::Up, WorldAnimation::Run, 2).as_str(),
        "character/red/3/5"
    );
    assert_eq!(
        world_character_asset(appearance, Direction::Up, WorldAnimation::RunStopping, 99).as_str(),
        "character/red/3/3"
    );
}

#[test]
fn world_camera_motion_does_not_apply_the_map_offset_to_the_character() {
    let world = WorldApplication::demo().unwrap();
    let map = ViewLayer::new(LayerKind::Map).with_surface(
        punctum_grid::Surface::filled(GridSize::new(32, 24), ViewCell::Empty).unwrap(),
    );
    let view = compose_world(
        map,
        GridPos::new(-5, 0),
        &world.observe().unwrap(),
        WorldAnimation::Walk,
        1,
        punctum_gpu::PixelOffset::new(0, 0),
        None,
    )
    .unwrap();

    assert_eq!(
        view.layers()[1].images[0].pixel_offset,
        punctum_gpu::PixelOffset::new(0, 0)
    );
    let map = ViewLayer::new(LayerKind::Map).with_surface(
        punctum_grid::Surface::filled(GridSize::new(32, 24), ViewCell::Empty).unwrap(),
    );
    let with_console = compose_world(
        map,
        GridPos::new(-5, 0),
        &world.observe().unwrap(),
        WorldAnimation::Stand,
        0,
        punctum_gpu::PixelOffset::new(0, 0),
        Some(&CommandConsoleView::default()),
    )
    .unwrap();
    assert_eq!(with_console.layers().len(), 4);
}

#[test]
fn world_projection_adds_a_speech_label_for_an_actor_speaking() {
    let material = CompositeTile::new(
        CompositeTileId::new("ground").unwrap(),
        vec![AtomicTileId::new("tile-0001").unwrap()],
    );
    let mut project = MapProject::blank(MapProjectId::new("speech").unwrap(), 6, 4, Some(material));
    project.player_spawn = TilePosition::new(0, 0);
    project.actors.push(MapActor::new(
        MapActorId::new("guide").unwrap(),
        TilePosition::new(2, 1),
        MapDirection::Left,
        CharacterAppearanceId::new("dppt/000").unwrap(),
    ));
    let mut continuations = std::collections::BTreeMap::new();
    continuations.insert(
        narrative_cps::ContinuationId::new(0),
        narrative_cps::CpsNode::Say {
            text: narrative_cps::TextId::new("text:hello_there").unwrap(),
            next: narrative_cps::ContinuationId::new(1),
        },
    );
    continuations.insert(
        narrative_cps::ContinuationId::new(1),
        narrative_cps::CpsNode::End,
    );
    let script = narrative_cps::ScriptProgram::with_actor(
        narrative_cps::ScriptId::new("script:guide").unwrap(),
        Some(narrative_cps::ActorId::new("actor:guide").unwrap()),
        narrative_cps::ContinuationId::new(0),
        continuations,
    )
    .unwrap();
    let observation = WorldApplication::from_map_project_with_scripts(&project, [script])
        .unwrap()
        .advance_npcs()
        .unwrap()
        .observe()
        .unwrap();
    let view = project_world(&observation).unwrap();
    assert!(
        view.labels()
            .any(|label| label.role == TextRole::Message && label.content == "你好。")
    );
    let speech_background = view.layers()[1]
        .images
        .iter()
        .find(|image| image.asset == rounded_ui_asset())
        .unwrap();
    assert_eq!(speech_background.tint, super::SPEECH_BUBBLE);
    assert_eq!(speech_background.bounds.size, GridSize::new(10, 2));
}

#[test]
fn world_projection_omits_speech_that_cannot_fit_within_the_canvas() -> Result<(), String> {
    let material = CompositeTile::new(
        CompositeTileId::new("ground").map_err(|error| error.to_string())?,
        vec![AtomicTileId::new("tile-0001").map_err(|error| error.to_string())?],
    );
    let mut project = MapProject::blank(
        MapProjectId::new("speech-bottom-edge").map_err(|error| error.to_string())?,
        16,
        14,
        Some(material),
    );
    project.player_spawn = TilePosition::new(0, 0);
    project.actors.push(MapActor::new(
        MapActorId::new("guide").map_err(|error| error.to_string())?,
        TilePosition::new(7, 13),
        MapDirection::Left,
        CharacterAppearanceId::new("dppt/000").map_err(|error| error.to_string())?,
    ));
    let mut continuations = std::collections::BTreeMap::new();
    continuations.insert(
        narrative_cps::ContinuationId::new(0),
        narrative_cps::CpsNode::Say {
            text: narrative_cps::TextId::new("text:hello_there")
                .map_err(|error| error.to_string())?,
            next: narrative_cps::ContinuationId::new(1),
        },
    );
    continuations.insert(
        narrative_cps::ContinuationId::new(1),
        narrative_cps::CpsNode::End,
    );
    let script = narrative_cps::ScriptProgram::with_actor(
        narrative_cps::ScriptId::new("script:guide").map_err(|error| error.to_string())?,
        Some(narrative_cps::ActorId::new("actor:guide").map_err(|error| error.to_string())?),
        narrative_cps::ContinuationId::new(0),
        continuations,
    )
    .map_err(|error| error.to_string())?;
    let observation = WorldApplication::from_map_project_with_scripts(&project, [script])
        .map_err(|error| format!("{error:?}"))?
        .advance_npcs()
        .map_err(|error| format!("{error:?}"))?
        .observe()
        .map_err(|error| format!("{error:?}"))?;

    let view = project_world(&observation).map_err(|error| error.to_string())?;

    assert!(
        view.layers()[1]
            .images
            .iter()
            .all(|image| image.asset != rounded_ui_asset())
    );
    assert!(!view.labels().any(|label| label.role == TextRole::Message));
    Ok(())
}

#[test]
fn command_console_is_an_explicit_top_layer() {
    let layer = project_console(&CommandConsoleView {
        query: "move".into(),
        preedit: "中".into(),
        items: vec!["/battle/move/one use".into(), "/battle/move/two use".into()],
        selected_index: Some(1),
        diagnostic: Some("action rejected".into()),
    })
    .unwrap();

    assert_eq!(layer.kind, LayerKind::Console);
    assert!(
        layer
            .labels
            .iter()
            .any(|label| { label.role == TextRole::ConsoleQuery && label.content == "> move中" })
    );
    assert!(layer.labels.iter().any(|label| {
        label.role == TextRole::ConsoleItem(1) && label.content == "/battle/move/two use"
    }));
    assert!(
        layer
            .labels
            .iter()
            .any(|label| label.role == TextRole::ConsoleDiagnostic)
    );
    assert!(layer.surface.unwrap().get(GridPos::new(2, 9)).is_ok());

    let empty = project_console(&CommandConsoleView::default()).unwrap();
    assert!(
        empty
            .labels
            .iter()
            .any(|label| label.content == "没有匹配指令")
    );
    let items = (0..12).map(|index| format!("item-{index}")).collect();
    let scrolled = project_console(&CommandConsoleView {
        items,
        selected_index: Some(11),
        ..CommandConsoleView::default()
    })
    .unwrap();
    assert_eq!(
        scrolled
            .labels
            .iter()
            .filter(|label| matches!(label.role, TextRole::ConsoleItem(_)))
            .count(),
        8
    );
    assert_eq!(super::visible_console_start(12, Some(11)), 4);
    assert_eq!(super::visible_console_start(3, None), 0);

    let observation = WorldApplication::demo().unwrap().observe().unwrap();
    let world = project_world(&observation).unwrap();
    assert_eq!(
        super::with_console(world.layers().to_vec(), None)
            .unwrap()
            .layers()
            .len(),
        3
    );
    assert_eq!(
        super::with_console(
            world.layers().to_vec(),
            Some(&CommandConsoleView::default())
        )
        .unwrap()
        .layers()
        .last()
        .unwrap()
        .kind,
        LayerKind::Console
    );
}
