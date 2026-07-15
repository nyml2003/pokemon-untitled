//! Pure Gen3 product-view projection.

#![forbid(unsafe_code)]

use battle_session::{
    Ability, Action, BattleCue, BattleInteraction, BattleObservation, BattleSessionSnapshot,
    MoveCategory, ObservedBattleOutcome, Participant, Pokemon, PokemonType, TypeEffectiveness,
    UsedMove,
};
use game_assets::AssetKey;
use game_data::PokedexData;
use game_ui::{BattleMenuPage, BattleUiState, CommandConsoleView, WorldAnimation};
use punctum_gpu::{PixelOffset, Rgba8};
use punctum_grid::{GridPos, GridRect, GridSize, Surface};
use world_application::{Direction as WorldDirection, Position, WorldObservation};

pub const CANVAS_WIDTH: u32 = 32;
pub const CANVAS_HEIGHT: u32 = 24;

const SKY: Rgba8 = Rgba8::new(146, 211, 218, 255);
const SKY_DEEP: Rgba8 = Rgba8::new(102, 177, 184, 255);
const DISTANT_GRASS: Rgba8 = Rgba8::new(75, 143, 105, 255);
const GROUND: Rgba8 = Rgba8::new(54, 105, 76, 255);
const GROUND_DARK: Rgba8 = Rgba8::new(37, 78, 62, 255);
const PLATFORM: Rgba8 = Rgba8::new(174, 201, 145, 255);
const PLATFORM_SHADOW: Rgba8 = Rgba8::new(45, 82, 64, 150);
const PANEL: Rgba8 = Rgba8::new(28, 34, 45, 248);
const PANEL_EDGE: Rgba8 = Rgba8::new(218, 225, 214, 255);
const SELECTED: Rgba8 = Rgba8::new(73, 211, 168, 255);
const SELECTED_DARK: Rgba8 = Rgba8::new(29, 70, 67, 255);
const BATTLE_CARD: Rgba8 = Rgba8::new(242, 246, 239, 255);
const BATTLE_CARD_SHADOW: Rgba8 = Rgba8::new(24, 37, 45, 190);
const BATTLE_INK: Rgba8 = Rgba8::new(26, 39, 45, 255);
const BATTLE_MUTED: Rgba8 = Rgba8::new(82, 96, 98, 255);
const OPPONENT_ACCENT: Rgba8 = Rgba8::new(241, 112, 116, 255);
const PLAYER_ACCENT: Rgba8 = Rgba8::new(57, 190, 151, 255);
const ACTION_PANEL: Rgba8 = Rgba8::new(19, 25, 34, 255);
const ACTION_PANEL_ALT: Rgba8 = Rgba8::new(30, 38, 49, 255);
const ACTION_BORDER: Rgba8 = Rgba8::new(83, 98, 112, 255);
const PARTY_BG: Rgba8 = Rgba8::new(13, 18, 27, 255);
const PARTY_PANEL: Rgba8 = Rgba8::new(25, 33, 44, 255);
const PARTY_PANEL_ALT: Rgba8 = Rgba8::new(34, 44, 57, 255);
const PARTY_EDGE: Rgba8 = Rgba8::new(73, 89, 105, 255);
const HP_GOOD: Rgba8 = Rgba8::new(74, 190, 102, 255);
const HP_MID: Rgba8 = Rgba8::new(226, 177, 66, 255);
const HP_LOW: Rgba8 = Rgba8::new(224, 91, 72, 255);
const HP_TRACK_EDGE: Rgba8 = Rgba8::new(38, 46, 55, 255);
const HP_GOOD_GLOW: Rgba8 = Rgba8::new(119, 231, 142, 255);
const HP_MID_GLOW: Rgba8 = Rgba8::new(255, 214, 101, 255);
const HP_LOW_GLOW: Rgba8 = Rgba8::new(255, 133, 111, 255);
const TEXT: Rgba8 = Rgba8::new(244, 246, 239, 255);
const MUTED_TEXT: Rgba8 = Rgba8::new(182, 194, 194, 255);
const CONSOLE_ERROR: Rgba8 = Rgba8::new(255, 142, 126, 255);
const MAP_GROUND: Rgba8 = Rgba8::new(138, 187, 116, 255);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BattleAnimation {
    #[default]
    Idle,
    Acting(Participant),
    Hit(Participant),
    Fainted(Participant),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextRole {
    Location,
    OpponentName,
    OpponentDetail,
    OpponentHp,
    PlayerName,
    PlayerDetail,
    PlayerHp,
    Action(usize),
    ActionDetail(usize),
    PageTitle,
    TeamMember(usize),
    TeamMemberHp(usize),
    TeamMemberType(usize),
    SelectedMemberName,
    SelectedMemberDetail,
    SelectedMemberHp,
    Message,
    ConsoleQuery,
    ConsoleItem(usize),
    ConsoleDiagnostic,
    Editor,
    PokedexTitle,
    PokedexEntry,
    PokedexDetail,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextLabel {
    pub role: TextRole,
    pub col: u32,
    pub row: u32,
    pub width: u32,
    pub height: u32,
    pub content: String,
    pub color: Rgba8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewCell {
    Empty,
    Fill(Rgba8),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewImage {
    pub bounds: GridRect,
    pub asset: AssetKey,
    pub tint: Rgba8,
    pub z_index: u16,
    pub pixel_offset: PixelOffset,
}

impl ViewImage {
    pub fn new(bounds: GridRect, asset: AssetKey, tint: Rgba8, z_index: u16) -> Self {
        Self {
            bounds,
            asset,
            tint,
            z_index,
            pixel_offset: PixelOffset::new(0, 0),
        }
    }

    pub const fn with_pixel_offset(mut self, pixel_offset: PixelOffset) -> Self {
        self.pixel_offset = pixel_offset;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewLayer {
    pub kind: LayerKind,
    pub surface: Option<Surface<ViewCell>>,
    pub images: Vec<ViewImage>,
    pub labels: Vec<TextLabel>,
}

impl ViewLayer {
    pub fn new(kind: LayerKind) -> Self {
        Self {
            kind,
            surface: None,
            images: Vec::new(),
            labels: Vec::new(),
        }
    }

    pub fn with_surface(mut self, surface: Surface<ViewCell>) -> Self {
        self.surface = Some(surface);
        self
    }

    pub fn with_images(mut self, images: Vec<ViewImage>) -> Self {
        self.images = images;
        self
    }

    pub fn with_labels(mut self, labels: Vec<TextLabel>) -> Self {
        self.labels = labels;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LayerKind {
    Map,
    Character,
    Hud,
    Console,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameView {
    layers: Vec<ViewLayer>,
}

impl GameView {
    pub fn new(layers: impl IntoIterator<Item = ViewLayer>) -> Self {
        let layers = layers.into_iter().collect::<Vec<_>>();
        assert!(layers.windows(2).all(|pair| pair[0].kind <= pair[1].kind));
        Self { layers }
    }

    pub fn layers(&self) -> &[ViewLayer] {
        &self.layers
    }

    pub fn images(&self) -> impl Iterator<Item = &ViewImage> {
        self.layers.iter().flat_map(|layer| &layer.images)
    }

    pub fn labels(&self) -> impl Iterator<Item = &TextLabel> {
        self.layers.iter().flat_map(|layer| &layer.labels)
    }
}

pub fn project_pokedex(pokedex: &PokedexData, selected_index: usize) -> GameView {
    let entries = pokedex.entries();
    let selected_index = selected_index.min(entries.len().saturating_sub(1));
    let entry = &entries[selected_index];
    let mut canvas = Canvas::new(Rgba8::new(13, 21, 29, 255));
    canvas.fill(0, 0, CANVAS_WIDTH, 3, Rgba8::new(21, 47, 60, 255));
    canvas.fill(1, 4, 11, 17, Rgba8::new(31, 52, 64, 255));
    canvas.fill(13, 4, 18, 17, Rgba8::new(237, 242, 233, 255));
    canvas.fill(14, 5, 16, 9, Rgba8::new(201, 220, 208, 255));
    canvas.fill(14, 15, 16, 5, Rgba8::new(31, 52, 64, 255));
    let first = selected_index
        .saturating_sub(2)
        .min(entries.len().saturating_sub(5));
    let mut labels = vec![
        label(TextRole::PokedexTitle, 2, 1, 20, 1, "宝可梦图鉴", TEXT),
        label(
            TextRole::PokedexDetail,
            23,
            1,
            7,
            1,
            &format!("{}/{}", selected_index + 1, entries.len()),
            MUTED_TEXT,
        ),
        label(
            TextRole::PokedexDetail,
            15,
            5,
            8,
            1,
            &format!("No.{:03}", entry.national_dex),
            BATTLE_INK,
        ),
        label(
            TextRole::PokedexTitle,
            15,
            7,
            12,
            1,
            &entry.localized_name,
            BATTLE_INK,
        ),
        label(
            TextRole::PokedexDetail,
            15,
            8,
            13,
            1,
            &entry.english_name,
            BATTLE_MUTED,
        ),
        label(
            TextRole::PokedexDetail,
            15,
            16,
            14,
            1,
            &format!(
                "HP {:>3}  ATK {:>3}  DEF {:>3}",
                entry.base_stats.hp, entry.base_stats.attack, entry.base_stats.defense
            ),
            TEXT,
        ),
        label(
            TextRole::PokedexDetail,
            15,
            18,
            15,
            1,
            &format!(
                "SPA {:>3}  SPD {:>3}  SPE {:>3}",
                entry.base_stats.special_attack,
                entry.base_stats.special_defense,
                entry.base_stats.speed
            ),
            TEXT,
        ),
    ];
    for (row, candidate) in entries.iter().skip(first).take(5).enumerate() {
        let prefix = if first + row == selected_index {
            ">"
        } else {
            " "
        };
        labels.push(label(
            TextRole::PokedexEntry,
            2,
            6 + row as u32 * 3,
            9,
            1,
            &format!(
                "{prefix}{:03} {}",
                candidate.national_dex, candidate.localized_name
            ),
            if first + row == selected_index {
                SELECTED
            } else {
                TEXT
            },
        ));
    }
    let mut images = vec![ViewImage::new(
        GridRect::new(GridPos::new(23, 5), GridSize::new(6, 6)),
        AssetKey::new(format!("pokedex/{}", entry.national_dex))
            .expect("Pokedex asset key is valid"),
        Rgba8::new(255, 255, 255, 255),
        1,
    )];
    for (index, kind) in entry.types.iter().enumerate() {
        if let Some(pokemon_type) = pokedex_type(kind.id.0) {
            images.push(type_icon_image(15 + index as u32 * 3, 10, pokemon_type));
        } else {
            labels.push(label(
                TextRole::PokedexDetail,
                15,
                10 + index as u32,
                13,
                1,
                &kind.name,
                BATTLE_INK,
            ));
        }
    }
    GameView::new([
        ViewLayer::new(LayerKind::Map).with_surface(canvas.finish()),
        ViewLayer::new(LayerKind::Character).with_images(images),
        ViewLayer::new(LayerKind::Hud).with_labels(labels),
    ])
}

pub fn project_console(console: &CommandConsoleView) -> ViewLayer {
    const PANEL_COL: u32 = 1;
    const PANEL_ROW: u32 = 4;
    const PANEL_WIDTH: u32 = 30;
    const PANEL_HEIGHT: u32 = 16;
    const FIRST_ITEM_ROW: u32 = 8;
    const MAX_ITEMS: usize = 8;

    let panel = GridRect::new(
        GridPos::new(PANEL_COL as i32, PANEL_ROW as i32),
        GridSize::new(PANEL_WIDTH, PANEL_HEIGHT),
    );
    let mut surface = Surface::filled(GridSize::new(CANVAS_WIDTH, CANVAS_HEIGHT), ViewCell::Empty)
        .expect("the fixed console surface is valid");
    surface
        .fill_rect(panel, sprite(PANEL_EDGE))
        .expect("the fixed console panel fits the game canvas");
    surface
        .fill_rect(
            GridRect::new(
                GridPos::new((PANEL_COL + 1) as i32, (PANEL_ROW + 1) as i32),
                GridSize::new(PANEL_WIDTH - 2, PANEL_HEIGHT - 2),
            ),
            sprite(PANEL),
        )
        .expect("the fixed console body fits the game canvas");

    let mut labels = vec![label(
        TextRole::ConsoleQuery,
        3,
        6,
        26,
        1,
        &format!("> {}{}", console.query, console.preedit),
        TEXT,
    )];

    let first_visible = visible_console_start(console.items.len(), console.selected_index);
    for (visible_index, (item_index, item)) in console
        .items
        .iter()
        .enumerate()
        .skip(first_visible)
        .take(MAX_ITEMS)
        .enumerate()
    {
        let row = FIRST_ITEM_ROW + visible_index as u32;
        if console.selected_index == Some(item_index) {
            surface
                .fill_rect(
                    GridRect::new(GridPos::new(2, row as i32), GridSize::new(28, 1)),
                    sprite(SELECTED),
                )
                .expect("the fixed console selection fits the game canvas");
        }
        labels.push(label(
            TextRole::ConsoleItem(item_index),
            3,
            row,
            26,
            1,
            item,
            TEXT,
        ));
    }

    if console.items.is_empty() {
        labels.push(label(
            TextRole::ConsoleItem(0),
            3,
            FIRST_ITEM_ROW,
            26,
            1,
            "没有匹配指令",
            MUTED_TEXT,
        ));
    }
    if let Some(diagnostic) = &console.diagnostic {
        labels.push(label(
            TextRole::ConsoleDiagnostic,
            3,
            18,
            26,
            1,
            diagnostic,
            CONSOLE_ERROR,
        ));
    }
    ViewLayer::new(LayerKind::Console)
        .with_surface(surface)
        .with_labels(labels)
}

fn visible_console_start(item_count: usize, selected_index: Option<usize>) -> usize {
    selected_index
        .map_or(0, |selected| selected.saturating_add(1).saturating_sub(8))
        .min(item_count.saturating_sub(8))
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

pub fn project_battle(
    snapshot: &BattleSessionSnapshot,
    ui: BattleUiState,
    sprites: BattleSpriteResources,
    sprite_frame: usize,
) -> GameView {
    let prompt = prompt_data(snapshot.interaction());
    let (page, selected_index, notice) = ui.view();
    let message = notice
        .map(str::to_owned)
        .unwrap_or_else(|| battle_message(snapshot));
    if page == BattleMenuPage::Pokemon
        && let Some((observation, _)) = prompt
    {
        return project_pokemon_page(observation, ui, &message, sprite_frame);
    }

    let scene = snapshot.scene();
    let own = scene.own();
    let opponent = scene.opponent();
    let mut canvas = Canvas::new(SKY);
    draw_battlefield(&mut canvas);
    let battlefield_images = battlefield_images();
    let mut images = Vec::new();
    draw_status_panel(
        &mut images,
        1,
        1,
        opponent.current_hp(),
        opponent.max_hp(),
        OPPONENT_ACCENT,
    );
    draw_status_panel(
        &mut images,
        17,
        11,
        own.current_hp(),
        own.max_hp(),
        PLAYER_ACCENT,
    );
    let actions = prompt.map_or(&[][..], |(_, actions)| actions);
    let observation = prompt.map(|(observation, _)| observation);
    let action_count = match page {
        BattleMenuPage::Main => 4,
        BattleMenuPage::Fight => {
            if actions.contains(&Action::Struggle) {
                1
            } else {
                observation.map_or(0, |observation| active_pokemon(observation).moves().len())
            }
        }
        BattleMenuPage::Pokemon | BattleMenuPage::Hidden => 0,
    };
    draw_action_panel(&mut images, page, action_count, selected_index);
    let character_images = battle_images(battle_animation(snapshot.cue()), sprites, sprite_frame);
    images.extend(type_icon_images(
        10,
        3,
        opponent.primary_type(),
        opponent.secondary_type(),
    ));
    images.extend(type_icon_images(
        26,
        13,
        own.primary_type(),
        own.secondary_type(),
    ));

    let mut labels = vec![
        label(
            TextRole::OpponentName,
            4,
            2,
            7,
            1,
            opponent.name(),
            BATTLE_INK,
        ),
        label(
            TextRole::OpponentDetail,
            4,
            3,
            6,
            1,
            &format!("Lv.{}", opponent.level()),
            BATTLE_MUTED,
        ),
        label(
            TextRole::OpponentHp,
            4,
            4,
            9,
            1,
            &format!("HP {}/{}", opponent.current_hp(), opponent.max_hp()),
            BATTLE_MUTED,
        ),
        label(TextRole::PlayerName, 20, 12, 7, 1, own.name(), BATTLE_INK),
        label(
            TextRole::PlayerDetail,
            20,
            13,
            6,
            1,
            &format!("Lv.{}", own.level()),
            BATTLE_MUTED,
        ),
        label(
            TextRole::PlayerHp,
            20,
            14,
            9,
            1,
            &format!("HP {}/{}", own.current_hp(), own.max_hp()),
            BATTLE_MUTED,
        ),
    ];
    match page {
        BattleMenuPage::Main => {
            for (index, content) in ["战斗", "宝可梦", "包包", "逃走"].into_iter().enumerate()
            {
                let col = 20 + (index as u32 % 2) * 6;
                let row = 18 + (index as u32 / 2) * 2;
                labels.push(label(
                    TextRole::Action(index),
                    col,
                    row,
                    5,
                    1,
                    content,
                    if index == selected_index {
                        BATTLE_INK
                    } else {
                        TEXT
                    },
                ));
            }
            labels.push(label(TextRole::Message, 3, 20, 13, 2, &message, MUTED_TEXT));
        }
        BattleMenuPage::Fight if actions.contains(&Action::Struggle) => {
            labels.push(label(TextRole::Action(0), 3, 18, 8, 1, "挣扎", BATTLE_INK));
            images.push(type_icon_image(23, 18, PokemonType::Normal));
            images.push(move_category_icon_image(26, 18, MoveCategory::Physical));
            labels.push(label(
                TextRole::ActionDetail(0),
                23,
                20,
                7,
                1,
                "威50 PP--",
                MUTED_TEXT,
            ));
            labels.push(label(TextRole::Message, 3, 22, 17, 1, &message, MUTED_TEXT));
        }
        BattleMenuPage::Fight => {
            let moves = observation
                .map(active_pokemon)
                .map_or(&[][..], |pokemon| pokemon.moves());
            for (index, battle_move) in moves.iter().enumerate().take(4) {
                let col = 3 + (index as u32 % 2) * 10;
                let row = 18 + (index as u32 / 2) * 2;
                labels.push(label(
                    TextRole::Action(index),
                    col,
                    row,
                    8,
                    1,
                    battle_move.name(),
                    if index == selected_index {
                        BATTLE_INK
                    } else {
                        TEXT
                    },
                ));
            }
            if let Some(battle_move) = moves.get(selected_index) {
                images.push(type_icon_image(23, 18, battle_move.move_type()));
                images.push(move_category_icon_image(26, 18, battle_move.category()));
                labels.push(label(
                    TextRole::ActionDetail(selected_index),
                    23,
                    20,
                    7,
                    1,
                    &format!(
                        "威{} PP{}/{}",
                        battle_move.power(),
                        battle_move.current_pp(),
                        battle_move.max_pp()
                    ),
                    MUTED_TEXT,
                ));
            }
            labels.push(label(TextRole::Message, 3, 22, 17, 1, &message, MUTED_TEXT));
        }
        BattleMenuPage::Hidden => {
            labels.push(label(TextRole::Message, 3, 20, 26, 2, &message, TEXT))
        }
        BattleMenuPage::Pokemon => {}
    }

    GameView::new([
        ViewLayer::new(LayerKind::Map)
            .with_surface(canvas.finish())
            .with_images(battlefield_images),
        ViewLayer::new(LayerKind::Character).with_images(character_images),
        ViewLayer::new(LayerKind::Hud)
            .with_images(images)
            .with_labels(labels),
    ])
}

fn battle_animation(cue: Option<&BattleCue>) -> BattleAnimation {
    match cue {
        Some(BattleCue::MoveUsed { participant, .. }) => BattleAnimation::Acting(*participant),
        Some(BattleCue::DamageApplied { participant, .. })
        | Some(BattleCue::Critical { participant }) => BattleAnimation::Hit(*participant),
        Some(BattleCue::Fainted { participant }) => BattleAnimation::Fainted(*participant),
        _ => BattleAnimation::Idle,
    }
}

fn battle_message(snapshot: &BattleSessionSnapshot) -> String {
    let scene = snapshot.scene();
    match snapshot.cue() {
        Some(BattleCue::TurnStarted { turn }) => format!("第 {turn} 回合"),
        Some(BattleCue::Switched { participant }) => {
            format!("{} 上场了。", combatant_name(scene, *participant))
        }
        Some(BattleCue::MoveUsed {
            participant,
            used_move,
        }) => format!(
            "{} 使用了 {}！",
            combatant_name(scene, *participant),
            used_move_name(used_move)
        ),
        Some(BattleCue::DamageApplied {
            participant,
            amount,
        }) => format!(
            "{} 受到 {} 点伤害。",
            combatant_name(scene, *participant),
            amount
        ),
        Some(BattleCue::StatusApplied {
            participant,
            status,
        }) => format!(
            "{} {}了。",
            combatant_name(scene, *participant),
            major_status_message(*status)
        ),
        Some(BattleCue::StatusFailed { .. }) => "但是失败了。".into(),
        Some(BattleCue::StatusPreventsAction {
            participant,
            status,
        }) => format!(
            "{} 因{}无法行动。",
            combatant_name(scene, *participant),
            major_status_reason(*status)
        ),
        Some(BattleCue::StatusCured {
            participant,
            status,
        }) => format!(
            "{} 从{}中恢复了。",
            combatant_name(scene, *participant),
            major_status_kind_message(*status)
        ),
        Some(BattleCue::StatStageChanged {
            participant,
            stat,
            change,
            stage: _,
        }) => format!(
            "{} 的{}{}了。",
            combatant_name(scene, *participant),
            battle_stat_message(*stat),
            if *change > 0 { "提高" } else { "降低" }
        ),
        Some(BattleCue::Healed {
            participant,
            amount,
        }) => format!(
            "{} 回复了 {} 点 HP。",
            combatant_name(scene, *participant),
            amount
        ),
        Some(BattleCue::EffectFailed { .. }) => "但是失败了。".into(),
        Some(BattleCue::ProtectionActivated { participant }) => {
            format!("{} 进入了守住状态。", combatant_name(scene, *participant))
        }
        Some(BattleCue::ProtectionFailed { participant }) => {
            format!("{} 的守住失败了。", combatant_name(scene, *participant))
        }
        Some(BattleCue::MoveBlocked { target, .. }) => {
            format!("{} 守住了攻击！", combatant_name(scene, *target))
        }
        Some(BattleCue::SubstituteCreated {
            participant,
            substitute_hp,
        }) => format!(
            "{} 制造了替身（{} HP）。",
            combatant_name(scene, *participant),
            substitute_hp
        ),
        Some(BattleCue::SubstituteBlocked { target, .. }) => {
            format!("{} 的替身挡住了招式。", combatant_name(scene, *target))
        }
        Some(BattleCue::SubstituteDamaged {
            participant,
            amount,
            remaining_hp,
        }) => format!(
            "{} 的替身受到了 {} 点伤害（剩余 {}）。",
            combatant_name(scene, *participant),
            amount,
            remaining_hp
        ),
        Some(BattleCue::SubstituteBroke { participant }) => {
            format!("{} 的替身消失了。", combatant_name(scene, *participant))
        }
        Some(BattleCue::WeatherStarted {
            weather,
            turns_remaining,
        }) => match turns_remaining {
            Some(turns) => format!("{}开始了，剩余 {turns} 回合。", weather_message(*weather)),
            None => format!("{}开始了。", weather_message(*weather)),
        },
        Some(BattleCue::WeatherUpdated {
            weather,
            turns_remaining,
        }) => format!(
            "{}，剩余 {turns_remaining} 回合。",
            weather_message(*weather)
        ),
        Some(BattleCue::WeatherEnded { weather }) => {
            format!("{}停止了。", weather_message(*weather))
        }
        Some(BattleCue::AbilityActivated {
            participant,
            ability,
        }) => format!(
            "{} 的{}发动了！",
            combatant_name(scene, *participant),
            ability_message(*ability)
        ),
        Some(BattleCue::Flinched { participant }) => {
            format!("{} 畏缩了。", combatant_name(scene, *participant))
        }
        Some(BattleCue::Missed { .. }) => "攻击没有命中。".into(),
        Some(BattleCue::Critical { .. }) => "会心一击！".into(),
        Some(BattleCue::Effectiveness { effectiveness, .. }) => {
            effectiveness_message(*effectiveness).into()
        }
        Some(BattleCue::Fainted { participant }) => {
            format!("{} 倒下了。", combatant_name(scene, *participant))
        }
        Some(BattleCue::ReplacementRequired { .. }) => "请选择下一只宝可梦".into(),
        Some(BattleCue::BattleFinished { outcome }) => outcome_message(*outcome).into(),
        None => match snapshot.interaction() {
            BattleInteraction::ChooseAction(_) => "请选择行动".into(),
            BattleInteraction::ChooseReplacement(_) => "请选择下一只宝可梦".into(),
            BattleInteraction::PlaybackLocked => String::new(),
            BattleInteraction::Finished(prompt) => outcome_message(prompt.outcome()).into(),
        },
    }
}

fn major_status_message(status: battle_session::MajorStatus) -> &'static str {
    match status {
        battle_session::MajorStatus::Burn => "烧伤",
        battle_session::MajorStatus::BadlyPoisoned { .. } => "剧毒",
        battle_session::MajorStatus::Freeze => "冰冻",
        battle_session::MajorStatus::Paralysis => "麻痹",
        battle_session::MajorStatus::Poison => "中毒",
        battle_session::MajorStatus::Sleep { .. } => "睡着",
    }
}

fn major_status_reason(status: battle_session::MajorStatus) -> &'static str {
    match status {
        battle_session::MajorStatus::Freeze => "冰冻",
        battle_session::MajorStatus::Paralysis => "麻痹",
        battle_session::MajorStatus::Sleep { .. } => "睡眠",
        battle_session::MajorStatus::BadlyPoisoned { .. }
        | battle_session::MajorStatus::Burn
        | battle_session::MajorStatus::Poison => "状态",
    }
}

fn major_status_kind_message(status: battle_session::MajorStatusKind) -> &'static str {
    match status {
        battle_session::MajorStatusKind::Burn => "烧伤",
        battle_session::MajorStatusKind::BadlyPoisoned => "剧毒",
        battle_session::MajorStatusKind::Freeze => "冰冻",
        battle_session::MajorStatusKind::Paralysis => "麻痹",
        battle_session::MajorStatusKind::Poison => "中毒",
        battle_session::MajorStatusKind::Sleep => "睡眠",
    }
}

fn battle_stat_message(stat: battle_session::BattleStat) -> &'static str {
    match stat {
        battle_session::BattleStat::Attack => "攻击",
        battle_session::BattleStat::Defense => "防御",
        battle_session::BattleStat::SpecialAttack => "特攻",
        battle_session::BattleStat::SpecialDefense => "特防",
        battle_session::BattleStat::Speed => "速度",
        battle_session::BattleStat::Accuracy => "命中率",
        battle_session::BattleStat::Evasion => "闪避率",
    }
}

fn weather_message(weather: battle_session::Weather) -> &'static str {
    match weather {
        battle_session::Weather::Hail => "冰雹",
        battle_session::Weather::Rain => "下雨",
        battle_session::Weather::Sandstorm => "沙暴",
        battle_session::Weather::Sun => "阳光强烈",
    }
}

fn ability_message(ability: Ability) -> &'static str {
    match ability {
        Ability::AirLock => "气闸",
        Ability::ArenaTrap => "沙穴",
        Ability::BattleArmor => "战斗盔甲",
        Ability::Blaze => "猛火",
        Ability::Chlorophyll => "叶绿素",
        Ability::ClearBody => "清晰之躯",
        Ability::CloudNine => "无关天气",
        Ability::CompoundEyes => "复眼",
        Ability::Drizzle => "降雨",
        Ability::Drought => "日照",
        Ability::EarlyBird => "早起",
        Ability::FlashFire => "闪火",
        Ability::Guts => "根性",
        Ability::HugePower => "大力士",
        Ability::HyperCutter => "怪力钳",
        Ability::Hustle => "活力",
        Ability::Immunity => "免疫",
        Ability::Intimidate => "威吓",
        Ability::InnerFocus => "精神力",
        Ability::KeenEye => "锐利目光",
        Ability::Insomnia => "不眠",
        Ability::Levitate => "飘浮",
        Ability::Limber => "柔软",
        Ability::LiquidOoze => "污泥浆",
        Ability::MagmaArmor => "熔岩铠甲",
        Ability::MarvelScale => "神奇鳞片",
        Ability::NaturalCure => "自然回复",
        Ability::Overgrow => "茂盛",
        Ability::Pressure => "压迫感",
        Ability::PurePower => "瑜伽之力",
        Ability::RainDish => "雨盘",
        Ability::RockHead => "坚硬脑袋",
        Ability::SandStream => "扬沙",
        Ability::SandVeil => "沙隐",
        Ability::SereneGrace => "天恩",
        Ability::ShellArmor => "硬壳盔甲",
        Ability::ShedSkin => "蜕皮",
        Ability::ShieldDust => "鳞粉",
        Ability::ShadowTag => "踩影",
        Ability::SpeedBoost => "加速",
        Ability::Synchronize => "同步",
        Ability::SwiftSwim => "悠游自如",
        Ability::Swarm => "虫之预感",
        Ability::ThickFat => "厚脂肪",
        Ability::Torrent => "激流",
        Ability::VitalSpirit => "干劲",
        Ability::VoltAbsorb => "蓄电",
        Ability::WaterAbsorb => "蓄水",
        Ability::WaterVeil => "水幕",
        Ability::WhiteSmoke => "白色烟雾",
    }
}

fn combatant_name(scene: &battle_session::BattleScene, participant: Participant) -> &str {
    match participant {
        Participant::Own => scene.own().name(),
        Participant::Opponent => scene.opponent().name(),
    }
}

fn used_move_name(used_move: &UsedMove) -> &str {
    match used_move {
        UsedMove::Move { name, .. } => name,
        UsedMove::Struggle => "挣扎",
    }
}

fn outcome_message(outcome: ObservedBattleOutcome) -> &'static str {
    match outcome {
        ObservedBattleOutcome::Winner(Participant::Own) => "你赢了！",
        ObservedBattleOutcome::Winner(Participant::Opponent) => "对手赢了。",
        ObservedBattleOutcome::Escaped(Participant::Own) => "成功逃走了！",
        ObservedBattleOutcome::Escaped(Participant::Opponent) => "对手逃走了。",
        ObservedBattleOutcome::Draw => "战斗平局。",
    }
}

fn effectiveness_message(effectiveness: TypeEffectiveness) -> &'static str {
    match effectiveness {
        TypeEffectiveness::Immune => "没有效果。",
        TypeEffectiveness::Quarter | TypeEffectiveness::Half => "效果不太好……",
        TypeEffectiveness::Normal => "命中了。",
        TypeEffectiveness::Double | TypeEffectiveness::Quadruple => "效果绝佳！",
    }
}

fn project_pokemon_page(
    observation: &BattleObservation,
    ui: BattleUiState,
    message: &str,
    sprite_frame: usize,
) -> GameView {
    let selected_index = ui.view().1;
    let selected_pokemon = &observation.own().members()[selected_index];
    let canvas = Canvas::new(PARTY_BG);

    let mut labels = vec![
        label(TextRole::PageTitle, 3, 1, 26, 1, "选择宝可梦", TEXT),
        label(
            TextRole::SelectedMemberName,
            2,
            13,
            9,
            1,
            selected_pokemon.name(),
            TEXT,
        ),
        label(
            TextRole::SelectedMemberDetail,
            2,
            14,
            9,
            1,
            &format!(
                "Lv.{}{}",
                selected_pokemon.level(),
                if selected_index == observation.own().active_slot().index() {
                    "  出战"
                } else {
                    ""
                }
            ),
            MUTED_TEXT,
        ),
        label(
            TextRole::SelectedMemberHp,
            2,
            18,
            9,
            1,
            &if selected_pokemon.is_fainted() {
                "无法战斗".into()
            } else {
                format!(
                    "HP {}/{}",
                    selected_pokemon.current_hp(),
                    selected_pokemon.max_hp()
                )
            },
            if selected_pokemon.is_fainted() {
                HP_LOW
            } else {
                TEXT
            },
        ),
    ];
    let mut images = vec![
        rounded_image(1, 0, 30, 3, PARTY_PANEL_ALT, 0),
        rounded_image(1, 4, 11, 17, PARTY_PANEL, 0),
        rounded_image(1, 21, 30, 3, PARTY_PANEL, 0),
    ];
    images.push(pokemon_icon_image(
        GridRect::new(GridPos::new(3, 5), GridSize::new(7, 7)),
        selected_index,
        selected_pokemon.is_fainted(),
        sprite_frame,
    ));
    images.extend(type_icon_images(
        3,
        16,
        selected_pokemon.primary_type(),
        selected_pokemon.secondary_type(),
    ));
    draw_hp_bar(
        &mut images,
        2,
        20,
        9,
        selected_pokemon.current_hp(),
        selected_pokemon.max_hp(),
    );

    for (index, pokemon) in observation.own().members().iter().enumerate() {
        let row = 4 + index as u32 * 3;
        let selected = index == selected_index;
        draw_team_card(&mut images, 13, row, selected, pokemon);
        images.push(pokemon_icon_image(
            GridRect::new(GridPos::new(14, row as i32), GridSize::new(3, 3)),
            index,
            pokemon.is_fainted(),
            sprite_frame,
        ));
        let active = index == observation.own().active_slot().index();
        labels.push(label(
            TextRole::TeamMember(index),
            18,
            row,
            8,
            1,
            pokemon.name(),
            if pokemon.is_fainted() {
                MUTED_TEXT
            } else if selected {
                TEXT
            } else {
                MUTED_TEXT
            },
        ));
        labels.push(label(
            TextRole::TeamMemberType(index),
            26,
            row,
            4,
            1,
            &if active {
                "出战".into()
            } else {
                format!("Lv.{}", pokemon.level())
            },
            if active { PLAYER_ACCENT } else { MUTED_TEXT },
        ));
        labels.push(label(
            TextRole::TeamMemberHp(index),
            18,
            row + 1,
            11,
            1,
            &if pokemon.is_fainted() {
                "无法战斗".into()
            } else {
                format!("{}/{}", pokemon.current_hp(), pokemon.max_hp())
            },
            if pokemon.is_fainted() {
                HP_LOW
            } else {
                MUTED_TEXT
            },
        ));
    }
    labels.push(label(TextRole::Message, 3, 22, 27, 1, message, MUTED_TEXT));
    GameView::new([
        ViewLayer::new(LayerKind::Map),
        ViewLayer::new(LayerKind::Character),
        ViewLayer::new(LayerKind::Hud)
            .with_surface(canvas.finish())
            .with_images(images)
            .with_labels(labels),
    ])
}

pub fn project_world(observation: &WorldObservation) -> GameView {
    project_world_animated(observation, WorldAnimation::Stand, 0)
}

pub fn project_world_animated(
    observation: &WorldObservation,
    animation: WorldAnimation,
    sprite_frame: usize,
) -> GameView {
    project_world_presented(observation, animation, sprite_frame, PixelOffset::new(0, 0))
}

pub fn project_world_presented(
    observation: &WorldObservation,
    animation: WorldAnimation,
    sprite_frame: usize,
    pixel_offset: PixelOffset,
) -> GameView {
    GameView::new([
        ViewLayer::new(LayerKind::Map).with_surface(Canvas::new(MAP_GROUND).finish()),
        ViewLayer::new(LayerKind::Character).with_images(vec![world_player_image(
            observation.player(),
            observation.facing(),
            animation,
            sprite_frame,
            pixel_offset,
        )]),
        ViewLayer::new(LayerKind::Hud),
    ])
}

pub fn compose_world(
    map: ViewLayer,
    camera: GridPos,
    observation: &WorldObservation,
    animation: WorldAnimation,
    sprite_frame: usize,
    console: Option<&CommandConsoleView>,
) -> GameView {
    assert_eq!(map.kind, LayerKind::Map);
    let mut player = world_player_image(
        observation.player(),
        observation.facing(),
        animation,
        sprite_frame,
        PixelOffset::new(0, 0),
    );
    player.bounds.origin.col -= camera.col * 2;
    player.bounds.origin.row -= camera.row * 2;
    let mut layers = vec![
        map,
        ViewLayer::new(LayerKind::Character).with_images(vec![player]),
        ViewLayer::new(LayerKind::Hud),
    ];
    if let Some(console) = console {
        layers.push(project_console(console));
    }
    GameView::new(layers)
}

pub fn with_console(mut layers: Vec<ViewLayer>, console: Option<&CommandConsoleView>) -> GameView {
    if let Some(console) = console {
        layers.push(project_console(console));
    }
    GameView::new(layers)
}

fn world_player_image(
    position: Position,
    direction: WorldDirection,
    animation: WorldAnimation,
    sprite_frame: usize,
    pixel_offset: PixelOffset,
) -> ViewImage {
    ViewImage::new(
        GridRect::new(
            GridPos::new(i32::from(position.x()) * 2, i32::from(position.y()) * 2),
            GridSize::new(2, 2),
        ),
        world_character_asset(direction, animation, sprite_frame),
        Rgba8::new(255, 255, 255, 255),
        20,
    )
    .with_pixel_offset(pixel_offset)
}

pub fn world_character_asset(
    direction: WorldDirection,
    animation: WorldAnimation,
    sprite_frame: usize,
) -> AssetKey {
    let direction_index = match direction {
        WorldDirection::Down => 0,
        WorldDirection::Left => 1,
        WorldDirection::Right => 2,
        WorldDirection::Up => 3,
    };
    let frame_offset = match animation {
        WorldAnimation::Stand => 0,
        WorldAnimation::Walk => match sprite_frame % 4 {
            0 => 1,
            1 | 3 => 0,
            _ => 2,
        },
        WorldAnimation::Run => match sprite_frame % 4 {
            0 => 4,
            1 | 3 => 3,
            _ => 5,
        },
        WorldAnimation::RunStopping => 3,
    };
    AssetKey::new(format!("character/{direction_index}/{frame_offset}"))
        .expect("character asset keys are non-empty")
}

fn active_pokemon(observation: &BattleObservation) -> &Pokemon {
    &observation.own().members()[observation.own().active_slot().index()]
}

fn rounded_image(
    col: u32,
    row: u32,
    width: u32,
    height: u32,
    tint: Rgba8,
    z_index: u16,
) -> ViewImage {
    shape_image(col, row, width, height, rounded_ui_asset(), tint, z_index)
}

fn pill_image(col: u32, row: u32, width: u32, height: u32, tint: Rgba8, z_index: u16) -> ViewImage {
    shape_image(col, row, width, height, pill_ui_asset(), tint, z_index)
}

#[allow(clippy::too_many_arguments)]
fn shape_image(
    col: u32,
    row: u32,
    width: u32,
    height: u32,
    asset: AssetKey,
    tint: Rgba8,
    z_index: u16,
) -> ViewImage {
    ViewImage::new(
        GridRect::new(
            GridPos::new(col as i32, row as i32),
            GridSize::new(width, height),
        ),
        asset,
        tint,
        z_index,
    )
}

fn draw_battlefield(canvas: &mut Canvas) {
    canvas.fill(0, 0, CANVAS_WIDTH, 7, SKY);
    canvas.fill(0, 7, CANVAS_WIDTH, 2, SKY_DEEP);
    canvas.fill(0, 9, CANVAS_WIDTH, 3, DISTANT_GRASS);
    canvas.fill(0, 12, CANVAS_WIDTH, 5, GROUND);
    canvas.fill(0, 16, CANVAS_WIDTH, 1, GROUND_DARK);
    for col in [1, 5, 12, 16, 28] {
        canvas.fill(col, 10, 3, 1, GROUND);
    }
}

fn battlefield_images() -> Vec<ViewImage> {
    vec![
        pill_image(20, 8, 11, 3, PLATFORM_SHADOW, 0),
        pill_image(20, 7, 10, 3, PLATFORM, 1),
        pill_image(1, 14, 14, 3, PLATFORM_SHADOW, 0),
        pill_image(2, 13, 13, 3, PLATFORM, 1),
    ]
}

fn draw_status_panel(
    images: &mut Vec<ViewImage>,
    col: u32,
    row: u32,
    hp: u32,
    max_hp: u32,
    accent: Rgba8,
) {
    images.push(
        rounded_image(col, row, 14, 5, BATTLE_CARD_SHADOW, 0)
            .with_pixel_offset(PixelOffset::new(3, 3)),
    );
    images.push(rounded_image(col, row, 14, 5, BATTLE_CARD, 1));
    images.push(pill_image(col + 1, row + 1, 1, 3, accent, 2));
    draw_hp_bar(images, col + 2, row + 4, 10, hp, max_hp);
}

fn draw_action_panel(
    images: &mut Vec<ViewImage>,
    page: BattleMenuPage,
    action_count: usize,
    selected: usize,
) {
    images.push(rounded_image(0, 17, CANVAS_WIDTH, 7, ACTION_BORDER, 0));
    images.push(rounded_image(1, 18, CANVAS_WIDTH - 2, 5, ACTION_PANEL, 1));
    match page {
        BattleMenuPage::Main => {
            images.push(rounded_image(18, 18, 13, 5, ACTION_PANEL_ALT, 2));
            images.push(pill_image(17, 19, 1, 3, ACTION_BORDER, 2));
            for index in 0..action_count.min(4) {
                if index == selected {
                    let col = 19 + (index as u32 % 2) * 6;
                    let row = 18 + (index as u32 / 2) * 2;
                    images.push(pill_image(col, row.saturating_sub(1), 6, 3, SELECTED, 3));
                }
            }
        }
        BattleMenuPage::Fight => {
            images.push(rounded_image(22, 18, 9, 5, ACTION_PANEL_ALT, 2));
            images.push(pill_image(21, 19, 1, 3, ACTION_BORDER, 2));
            for index in 0..action_count.min(4) {
                if index == selected {
                    let col = 2 + (index as u32 % 2) * 10;
                    let row = 18 + (index as u32 / 2) * 2;
                    images.push(pill_image(col, row.saturating_sub(1), 10, 3, SELECTED, 3));
                }
            }
        }
        BattleMenuPage::Pokemon | BattleMenuPage::Hidden => {}
    }
}

fn draw_team_card(
    images: &mut Vec<ViewImage>,
    col: u32,
    row: u32,
    selected: bool,
    pokemon: &Pokemon,
) {
    images.push(rounded_image(
        col,
        row,
        18,
        3,
        if selected {
            SELECTED_DARK
        } else {
            PARTY_PANEL_ALT
        },
        0,
    ));
    images.push(rounded_image(
        col + 1,
        row + 1,
        1,
        1,
        if pokemon.is_fainted() {
            HP_LOW
        } else if selected {
            SELECTED
        } else {
            PARTY_EDGE
        },
        2,
    ));
    draw_hp_bar(
        images,
        col + 5,
        row + 2,
        11,
        pokemon.current_hp(),
        pokemon.max_hp(),
    );
}

fn pokemon_icon_image(
    bounds: GridRect,
    slot: usize,
    fainted: bool,
    sprite_frame: usize,
) -> ViewImage {
    ViewImage::new(
        bounds,
        pokemon_icon_asset(slot, sprite_frame),
        if fainted {
            Rgba8::new(112, 112, 112, 255)
        } else {
            Rgba8::new(255, 255, 255, 255)
        },
        10,
    )
}

fn draw_hp_bar(images: &mut Vec<ViewImage>, col: u32, row: u32, width: u32, hp: u32, max_hp: u32) {
    if width == 0 {
        return;
    }
    images.push(pill_image(col, row, width, 1, HP_TRACK_EDGE, 3));
    let filled = hp.saturating_mul(width).checked_div(max_hp).unwrap_or(0);
    let (color, glow) = if hp.saturating_mul(4) <= max_hp {
        (HP_LOW, HP_LOW_GLOW)
    } else if hp.saturating_mul(2) <= max_hp {
        (HP_MID, HP_MID_GLOW)
    } else {
        (HP_GOOD, HP_GOOD_GLOW)
    };
    if filled > 0 {
        let filled = filled.min(width);
        images.push(pill_image(col, row, filled, 1, color, 4));
        images.push(pill_image(col + filled - 1, row, 1, 1, glow, 5));
    }
}

fn type_icon_images(
    col: u32,
    row: u32,
    primary: PokemonType,
    secondary: Option<PokemonType>,
) -> Vec<ViewImage> {
    let mut images = vec![type_icon_image(col, row, primary)];
    if let Some(secondary) = secondary {
        images.push(type_icon_image(col + 2, row, secondary));
    }
    images
}

fn type_icon_image(col: u32, row: u32, pokemon_type: PokemonType) -> ViewImage {
    ViewImage::new(
        GridRect::new(GridPos::new(col as i32, row as i32), GridSize::new(2, 1)),
        type_icon_asset(pokemon_type),
        Rgba8::new(255, 255, 255, 255),
        20,
    )
}

fn pokedex_type(id: u16) -> Option<PokemonType> {
    Some(match id {
        1 => PokemonType::Normal,
        2 => PokemonType::Fighting,
        3 => PokemonType::Flying,
        4 => PokemonType::Poison,
        5 => PokemonType::Ground,
        6 => PokemonType::Rock,
        7 => PokemonType::Bug,
        8 => PokemonType::Ghost,
        9 => PokemonType::Steel,
        10 => PokemonType::Fire,
        11 => PokemonType::Water,
        12 => PokemonType::Grass,
        13 => PokemonType::Electric,
        14 => PokemonType::Psychic,
        15 => PokemonType::Ice,
        16 => PokemonType::Dragon,
        17 => PokemonType::Dark,
        _ => return None,
    })
}

pub fn type_icon_asset(pokemon_type: PokemonType) -> AssetKey {
    let name = match pokemon_type {
        PokemonType::Normal => "normal",
        PokemonType::Fighting => "fighting",
        PokemonType::Flying => "flying",
        PokemonType::Poison => "poison",
        PokemonType::Ground => "ground",
        PokemonType::Rock => "rock",
        PokemonType::Bug => "bug",
        PokemonType::Ghost => "ghost",
        PokemonType::Steel => "steel",
        PokemonType::Fire => "fire",
        PokemonType::Water => "water",
        PokemonType::Grass => "grass",
        PokemonType::Electric => "electric",
        PokemonType::Psychic => "psychic",
        PokemonType::Ice => "ice",
        PokemonType::Dragon => "dragon",
        PokemonType::Dark => "dark",
    };
    AssetKey::new(format!("ui/battle/type/{name}")).expect("type icon asset keys are non-empty")
}

fn move_category_icon_image(col: u32, row: u32, category: MoveCategory) -> ViewImage {
    ViewImage::new(
        GridRect::new(GridPos::new(col as i32, row as i32), GridSize::new(2, 1)),
        move_category_icon_asset(category),
        Rgba8::new(255, 255, 255, 255),
        20,
    )
}

pub fn move_category_icon_asset(category: MoveCategory) -> AssetKey {
    let name = match category {
        MoveCategory::Physical => "physical",
        MoveCategory::Special => "special",
        MoveCategory::Status => "status",
    };
    AssetKey::new(format!("ui/battle/move-category/{name}"))
        .expect("move category asset keys are non-empty")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BattleSpriteResources {
    own: [AssetKey; 2],
    opponent: [AssetKey; 2],
}

impl BattleSpriteResources {
    pub fn for_slots(own_slot: usize, opponent_slot: usize) -> Self {
        Self {
            own: [
                player_back_asset(own_slot, 0),
                player_back_asset(own_slot, 1),
            ],
            opponent: [
                opponent_front_asset(opponent_slot, 0),
                opponent_front_asset(opponent_slot, 1),
            ],
        }
    }
}

pub fn player_back_asset(slot: usize, frame: usize) -> AssetKey {
    AssetKey::new(format!("battle/player/{slot}/back/{}", frame % 2))
        .expect("player sprite asset keys are non-empty")
}

pub fn opponent_front_asset(slot: usize, frame: usize) -> AssetKey {
    AssetKey::new(format!("battle/opponent/{slot}/front/{}", frame % 2))
        .expect("opponent sprite asset keys are non-empty")
}

pub fn pokemon_icon_asset(slot: usize, frame: usize) -> AssetKey {
    AssetKey::new(format!("battle/team/{slot}/icon/{}", frame % 2))
        .expect("team icon asset keys are non-empty")
}

pub fn rounded_ui_asset() -> AssetKey {
    AssetKey::new("ui/rounded-rect").expect("the rounded UI asset key is non-empty")
}

pub fn pill_ui_asset() -> AssetKey {
    AssetKey::new("ui/pill").expect("the pill UI asset key is non-empty")
}

fn battle_images(
    animation: BattleAnimation,
    sprites: BattleSpriteResources,
    sprite_frame: usize,
) -> Vec<ViewImage> {
    let player_origin = if animation == BattleAnimation::Acting(Participant::Own) {
        GridPos::new(6, 9)
    } else {
        GridPos::new(5, 10)
    };
    let opponent_origin = if animation == BattleAnimation::Acting(Participant::Opponent) {
        GridPos::new(21, 5)
    } else {
        GridPos::new(22, 4)
    };

    vec![
        ViewImage::new(
            GridRect::new(player_origin, GridSize::new(8, 8)),
            sprites.own[sprite_frame % 2].clone(),
            creature_tint(animation, Participant::Own),
            10,
        ),
        ViewImage::new(
            GridRect::new(opponent_origin, GridSize::new(8, 8)),
            sprites.opponent[sprite_frame % 2].clone(),
            creature_tint(animation, Participant::Opponent),
            10,
        ),
    ]
}

fn creature_tint(animation: BattleAnimation, participant: Participant) -> Rgba8 {
    match animation {
        BattleAnimation::Hit(target) if target == participant => Rgba8::new(255, 112, 112, 255),
        BattleAnimation::Fainted(target) if target == participant => Rgba8::new(112, 112, 112, 255),
        _ => Rgba8::new(255, 255, 255, 255),
    }
}

fn label(
    role: TextRole,
    col: u32,
    row: u32,
    width: u32,
    height: u32,
    content: &str,
    color: Rgba8,
) -> TextLabel {
    TextLabel {
        role,
        col,
        row,
        width,
        height,
        content: content.into(),
        color,
    }
}

struct Canvas {
    cells: Vec<ViewCell>,
}

impl Canvas {
    fn new(color: Rgba8) -> Self {
        Self {
            cells: vec![sprite(color); (CANVAS_WIDTH * CANVAS_HEIGHT) as usize],
        }
    }

    fn set(&mut self, col: u32, row: u32, color: Rgba8) {
        if col < CANVAS_WIDTH && row < CANVAS_HEIGHT {
            self.cells[(row * CANVAS_WIDTH + col) as usize] = sprite(color);
        }
    }

    fn fill(&mut self, col: u32, row: u32, width: u32, height: u32, color: Rgba8) {
        for y in row..row.saturating_add(height).min(CANVAS_HEIGHT) {
            for x in col..col.saturating_add(width).min(CANVAS_WIDTH) {
                self.set(x, y, color);
            }
        }
    }

    fn finish(self) -> Surface<ViewCell> {
        Surface::from_cells(GridSize::new(CANVAS_WIDTH, CANVAS_HEIGHT), self.cells)
            .expect("the fixed battle canvas dimensions are valid")
    }
}

const fn sprite(tint: Rgba8) -> ViewCell {
    ViewCell::Fill(tint)
}

#[cfg(test)]
mod tests {
    use battle_application::{
        Accuracy, BattleApplication, BattleStats, Move, MoveCategory, MoveId, Pokemon, PokemonId,
        PokemonType, TEAM_SIZE, Team,
    };
    use battle_session::{
        Action, BattleCoordinator, BattleObservation, BattleSession, BattleSessionSnapshot,
        OpponentPolicy,
    };
    use game_data::PokedexData;
    use punctum_grid::{GridPos, GridSize};
    use punctum_input::{KeyEvent, KeyPhase, LogicalKey, Modifiers, NamedKey, PhysicalKeyCode};
    use world_application::{Direction, Position, WorldApplication};

    use game_ui::{
        BattleMenuPage, BattleUiOutcome, BattleUiState, CommandConsoleView, WorldAnimation,
    };

    use super::{
        BattleSpriteResources, LayerKind, TextRole, ViewCell, ViewLayer, compose_world,
        move_category_icon_asset, pill_ui_asset, project_battle, project_console, project_pokedex,
        project_world, rounded_ui_asset, type_icon_asset, world_character_asset,
    };

    #[test]
    fn pokedex_projects_its_selected_canonical_front() {
        let data = PokedexData::embedded_gen3().unwrap();
        let view = project_pokedex(&data, 0);
        assert!(view.labels().any(|label| label.content == "妙蛙种子"));
        assert!(
            view.images()
                .any(|image| image.asset.as_str() == "pokedex/1")
        );
        assert!(
            view.images()
                .any(|image| image.asset == type_icon_asset(PokemonType::Grass))
        );
        assert!(
            view.images()
                .any(|image| image.asset == type_icon_asset(PokemonType::Poison))
        );
        assert_eq!(view.layers()[0].kind, LayerKind::Map);
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
        BattleSession::new(BattleCoordinator::new(application, FirstActionPolicy))
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
        let main = project_battle(&snapshot, ui, BattleSpriteResources::for_slots(0, 0), 0);
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
        let fight = project_battle(&snapshot, ui, BattleSpriteResources::for_slots(0, 0), 0);
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

        let view = project_battle(&snapshot, ui, BattleSpriteResources::for_slots(0, 0), 0);

        assert!(view.labels().any(|label| {
            label.role == TextRole::PageTitle && label.content == "选择宝可梦"
        }));
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

        let animated = project_battle(&snapshot, ui, BattleSpriteResources::for_slots(0, 0), 1);
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

        let view = project_battle(&snapshot, ui, BattleSpriteResources::for_slots(0, 0), 0);
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
            for animation in [
                WorldAnimation::Stand,
                WorldAnimation::Walk,
                WorldAnimation::Run,
                WorldAnimation::RunStopping,
            ] {
                for frame in 0..4 {
                    assert!(
                        world_character_asset(direction, animation, frame)
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
            );
            assert_eq!(view.layers()[0].kind, LayerKind::Map);
            projected += 1;
            if battle.is_finished() {
                break;
            }
            if battle.has_pending_playback() {
                let (next, advanced) = battle.advance();
                battle = next;
                assert!(advanced);
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
        let view = project_battle(&struggle, ui, BattleSpriteResources::for_slots(0, 0), 0);
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
                );
                assert!(view.labels().any(|label| label.content == "攻击没有命中。"));
                saw_miss = true;
            }
        }
        assert!(saw_miss);
    }

    #[test]
    fn world_projection_uses_stable_character_keys_and_explicit_layers() {
        let world = WorldApplication::demo().unwrap();
        let view = project_world(&world.observe());

        assert_eq!(world.observe().player(), Position::new(3, 6));
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
            world_character_asset(Direction::Down, WorldAnimation::Stand, 0)
        );
        assert_eq!(
            world_character_asset(Direction::Up, WorldAnimation::Run, 2).as_str(),
            "character/3/5"
        );
        assert_eq!(
            world_character_asset(Direction::Up, WorldAnimation::RunStopping, 99).as_str(),
            "character/3/3"
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
            &world.observe(),
            WorldAnimation::Walk,
            1,
            None,
        );

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
            &world.observe(),
            WorldAnimation::Stand,
            0,
            Some(&CommandConsoleView::default()),
        );
        assert_eq!(with_console.layers().len(), 4);
    }

    #[test]
    fn command_console_is_an_explicit_top_layer() {
        let layer = project_console(&CommandConsoleView {
            query: "move".into(),
            preedit: "中".into(),
            items: vec!["/battle/move/one use".into(), "/battle/move/two use".into()],
            selected_index: Some(1),
            diagnostic: Some("action rejected".into()),
        });

        assert_eq!(layer.kind, LayerKind::Console);
        assert!(
            layer.labels.iter().any(|label| {
                label.role == TextRole::ConsoleQuery && label.content == "> move中"
            })
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

        let empty = project_console(&CommandConsoleView::default());
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
        });
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

        let world = project_world(&WorldApplication::demo().unwrap().observe());
        assert_eq!(
            super::with_console(world.layers().to_vec(), None)
                .layers()
                .len(),
            3
        );
        assert_eq!(
            super::with_console(
                world.layers().to_vec(),
                Some(&CommandConsoleView::default())
            )
            .layers()
            .last()
            .unwrap()
            .kind,
            LayerKind::Console
        );
    }
}
