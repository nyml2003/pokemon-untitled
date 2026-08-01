//! 固定画布上的世界、战斗场景和控制台图层投影。

use super::{assets::*, common::*};
use battle_session::{
    Ability, Action, BattleCue, BattleInteraction, BattleObservation, BattleSessionSnapshot,
    BattleUnit, MoveCategory, ObservedBattleOutcome, Participant, PokemonType, TypeEffectiveness,
    UsedMove,
};
use game_assets::AssetKey;
use game_ui::{BattleMenuPage, BattleUiState, CommandConsoleView, WorldAnimation};
use punctum_gpu::{PixelOffset, Rgba8};
use punctum_grid::{GridPos, GridRect, GridSize, Surface, SurfaceError};
use world_application::{
    CharacterAppearanceId, Direction as WorldDirection, WorldActorObservation, WorldActorRole,
    WorldObservation,
};

/// 将命令控制台投影为固定游戏画布上的 `Console` 图层。
pub fn project_console(console: &CommandConsoleView) -> Result<ViewLayer, ProjectionError> {
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
    let mut surface = Surface::filled(GridSize::new(CANVAS_WIDTH, CANVAS_HEIGHT), ViewCell::Empty)?;
    surface.fill_rect(panel, sprite(PANEL_EDGE))?;
    surface.fill_rect(
        GridRect::new(
            GridPos::new((PANEL_COL + 1) as i32, (PANEL_ROW + 1) as i32),
            GridSize::new(PANEL_WIDTH - 2, PANEL_HEIGHT - 2),
        ),
        sprite(PANEL),
    )?;

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
            surface.fill_rect(
                GridRect::new(GridPos::new(2, row as i32), GridSize::new(28, 1)),
                sprite(SELECTED),
            )?;
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
    Ok(ViewLayer::new(LayerKind::Console)
        .with_surface(surface)
        .with_labels(labels))
}

pub(crate) fn visible_console_start(item_count: usize, selected_index: Option<usize>) -> usize {
    selected_index
        .map_or(0, |selected| selected.saturating_add(1).saturating_sub(8))
        .min(item_count.saturating_sub(8))
}

pub(crate) fn prompt_data(
    interaction: &BattleInteraction,
) -> Option<(&BattleObservation, &[Action])> {
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

/// 将战斗快照和 UI 状态投影为固定游戏画布。
/// 当 UI 位于换宝可梦页面且存在交互提示时，结果改为该页面的独立视图。
pub fn project_battle(
    snapshot: &BattleSessionSnapshot,
    ui: BattleUiState,
    sprites: BattleSpriteResources,
    sprite_frame: usize,
) -> Result<GameView, ProjectionError> {
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
                images.push(type_icon_image(
                    23,
                    18,
                    battle_move
                        .move_types()
                        .first()
                        .copied()
                        .unwrap_or(PokemonType::Normal),
                ));
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

    Ok(GameView::new([
        ViewLayer::new(LayerKind::Map)
            .with_surface(canvas.finish()?)
            .with_images(battlefield_images),
        ViewLayer::new(LayerKind::Character).with_images(character_images),
        ViewLayer::new(LayerKind::Hud)
            .with_images(images)
            .with_labels(labels),
    ]))
}

pub(crate) fn battle_animation(cue: Option<&BattleCue>) -> BattleAnimation {
    match cue {
        Some(BattleCue::MoveUsed { participant, .. }) => BattleAnimation::Acting(*participant),
        Some(BattleCue::DamageApplied { participant, .. })
        | Some(BattleCue::Critical { participant }) => BattleAnimation::Hit(*participant),
        Some(BattleCue::Fainted { participant }) => BattleAnimation::Fainted(*participant),
        _ => BattleAnimation::Idle,
    }
}

pub(crate) fn battle_message(snapshot: &BattleSessionSnapshot) -> String {
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

pub(crate) fn used_move_name(used_move: &UsedMove) -> &str {
    match used_move {
        UsedMove::Move { name, .. } => name,
        UsedMove::Struggle => "挣扎",
    }
}

pub(crate) fn outcome_message(outcome: ObservedBattleOutcome) -> &'static str {
    match outcome {
        ObservedBattleOutcome::Winner(Participant::Own) => "你赢了！",
        ObservedBattleOutcome::Winner(Participant::Opponent) => "对手赢了。",
        ObservedBattleOutcome::Escaped(Participant::Own) => "成功逃走了！",
        ObservedBattleOutcome::Escaped(Participant::Opponent) => "对手逃走了。",
        ObservedBattleOutcome::Draw => "战斗平局。",
    }
}

pub(crate) fn effectiveness_message(effectiveness: TypeEffectiveness) -> &'static str {
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
) -> Result<GameView, ProjectionError> {
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
        selected_pokemon
            .types()
            .first()
            .copied()
            .unwrap_or(PokemonType::Normal),
        selected_pokemon.types().get(1).copied(),
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
    Ok(GameView::new([
        ViewLayer::new(LayerKind::Map),
        ViewLayer::new(LayerKind::Character),
        ViewLayer::new(LayerKind::Hud)
            .with_surface(canvas.finish()?)
            .with_images(images)
            .with_labels(labels),
    ]))
}

/// 以静止动画和零偏移投影世界观察结果。
pub fn project_world(observation: &WorldObservation) -> Result<GameView, ProjectionError> {
    project_world_animated(observation, WorldAnimation::Stand, 0)
}

/// 以指定角色动画投影世界观察结果。
/// 角色和地图均不施加像素偏移。
pub fn project_world_animated(
    observation: &WorldObservation,
    animation: WorldAnimation,
    sprite_frame: usize,
) -> Result<GameView, ProjectionError> {
    project_world_presented(observation, animation, sprite_frame, PixelOffset::new(0, 0))
}

/// 以指定角色动画和像素偏移投影世界观察结果。
/// 偏移只应用于角色图像，不改变地图表面。
pub fn project_world_presented(
    observation: &WorldObservation,
    animation: WorldAnimation,
    sprite_frame: usize,
    pixel_offset: PixelOffset,
) -> Result<GameView, ProjectionError> {
    let mut actors = world_actor_images(
        observation,
        animation,
        sprite_frame,
        pixel_offset,
        PixelOffset::new(0, 0),
    );
    let speech = world_speech_overlay(observation, GridPos::new(0, 0), PixelOffset::new(0, 0));
    actors.extend(speech.images);
    Ok(GameView::new([
        ViewLayer::new(LayerKind::Map).with_surface(Canvas::new(MAP_GROUND).finish()?),
        ViewLayer::new(LayerKind::Character)
            .with_images(actors)
            .with_labels(speech.labels),
        ViewLayer::new(LayerKind::Hud),
    ]))
}

/// 将已渲染地图图层与世界角色、对话和可选控制台组合成游戏视图。
/// 相机会平移并裁剪角色和对话覆盖层。
///
pub fn compose_world(
    map: ViewLayer,
    camera: GridPos,
    observation: &WorldObservation,
    animation: WorldAnimation,
    sprite_frame: usize,
    npc_pixel_offset: PixelOffset,
    console: Option<&CommandConsoleView>,
) -> Result<GameView, ProjectionError> {
    if map.kind != LayerKind::Map {
        return Err(ProjectionError::ExpectedMapLayer { actual: map.kind });
    }
    let viewport_size = map
        .surface
        .as_ref()
        .ok_or(ProjectionError::MapLayerMissingSurface)?
        .size();
    let mut actors = world_actor_images(
        observation,
        animation,
        sprite_frame,
        PixelOffset::new(0, 0),
        npc_pixel_offset,
    );
    for actor in &mut actors {
        actor.bounds.origin.col -= camera.col * 2;
        actor.bounds.origin.row -= camera.row * 2;
    }
    actors.retain(|actor| actor.bounds.clip_to(viewport_size) == Some(actor.bounds));
    let speech = world_speech_overlay(observation, camera, npc_pixel_offset);
    actors.extend(speech.images);
    let mut layers = vec![
        map,
        ViewLayer::new(LayerKind::Character)
            .with_images(actors)
            .with_labels(speech.labels),
        ViewLayer::new(LayerKind::Hud),
    ];
    if let Some(console) = console {
        layers.push(project_console(console)?);
    }
    Ok(GameView::new(layers))
}

struct WorldSpeechOverlay {
    images: Vec<ViewImage>,
    labels: Vec<TextLabel>,
}

fn world_speech_overlay(
    observation: &WorldObservation,
    camera: GridPos,
    pixel_offset: PixelOffset,
) -> WorldSpeechOverlay {
    let mut images = Vec::new();
    let mut labels = Vec::new();
    for actor in observation.actors() {
        let Some(speech) = actor.speech() else {
            continue;
        };
        let center = i32::from(actor.position().x()) * 2 - camera.col * 2 + 1;
        let row = i32::from(actor.position().y()) * 2 - camera.row * 2 - 2;
        let max_row = CANVAS_HEIGHT.saturating_sub(SPEECH_BUBBLE_HEIGHT) as i32;
        if row < 0 || row > max_row || center < 0 || center >= CANVAS_WIDTH as i32 {
            continue;
        }
        let content = speech_text(speech.as_str());
        let width = (content.chars().count() as u32 * 2 + 2).clamp(10, 18);
        let max_col = CANVAS_WIDTH.saturating_sub(width) as i32;
        let col = (center - width as i32 / 2).clamp(0, max_col) as u32;
        let row = row as u32;
        images.push(
            rounded_image(col, row, width, SPEECH_BUBBLE_HEIGHT, SPEECH_BUBBLE, 100)
                .with_pixel_offset(pixel_offset),
        );
        labels.push(label(
            TextRole::Message,
            col + 1,
            row,
            width.saturating_sub(2),
            SPEECH_BUBBLE_HEIGHT,
            content,
            TEXT,
        ));
    }
    WorldSpeechOverlay { images, labels }
}

fn speech_text(text: &str) -> &str {
    match text {
        "text:guide_hello" => "前方的小路很安全。",
        "text:ranger_welcome" => "森林里要注意脚下。",
        "text:collector_found" => "我刚找到一个好东西。",
        "text:hello_there" => "你好。",
        _ => "……",
    }
}

/// 在已有图层后附加可选控制台，并构建游戏视图。
/// `layers` 必须已按 `LayerKind` 非递减顺序排列，且不能包含晚于 `Console` 的图层。
pub fn with_console(
    mut layers: Vec<ViewLayer>,
    console: Option<&CommandConsoleView>,
) -> Result<GameView, ProjectionError> {
    if let Some(console) = console {
        layers.push(project_console(console)?);
    }
    Ok(GameView::new(layers))
}

fn world_actor_images(
    observation: &WorldObservation,
    animation: WorldAnimation,
    sprite_frame: usize,
    player_pixel_offset: PixelOffset,
    npc_pixel_offset: PixelOffset,
) -> Vec<ViewImage> {
    let mut actors = observation.actors().to_vec();
    actors.sort_by(|left, right| {
        (left.position().y(), left.position().x(), left.id().as_str()).cmp(&(
            right.position().y(),
            right.position().x(),
            right.id().as_str(),
        ))
    });
    actors
        .iter()
        .enumerate()
        .map(|(index, actor)| {
            let (animation, frame, pixel_offset) = match actor.role() {
                WorldActorRole::Player => (animation, sprite_frame, player_pixel_offset),
                WorldActorRole::Npc => (WorldAnimation::Stand, 0, npc_pixel_offset),
            };
            world_actor_image(actor, animation, frame, pixel_offset, 20 + index as u16)
        })
        .collect()
}

fn world_actor_image(
    actor: &WorldActorObservation,
    animation: WorldAnimation,
    sprite_frame: usize,
    pixel_offset: PixelOffset,
    z_index: u16,
) -> ViewImage {
    let position = actor.position();
    ViewImage::new(
        GridRect::new(
            GridPos::new(i32::from(position.x()) * 2, i32::from(position.y()) * 2),
            GridSize::new(2, 2),
        ),
        world_character_asset(actor.appearance(), actor.facing(), animation, sprite_frame),
        Rgba8::new(255, 255, 255, 255),
        z_index,
    )
    .with_pixel_offset(pixel_offset)
}

/// 返回角色外观、朝向和动画帧对应的资源键。
/// 行走动画按四帧循环取模，静止动画始终使用静止帧。
pub fn world_character_asset(
    appearance: &CharacterAppearanceId,
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
    AssetKey::from_resource_template(format!(
        "character/{}/{direction_index}/{frame_offset}",
        appearance.as_str()
    ))
}

pub(crate) fn active_pokemon(observation: &BattleObservation) -> &BattleUnit {
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
    pokemon: &BattleUnit,
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

pub(crate) fn draw_hp_bar(
    images: &mut Vec<ViewImage>,
    col: u32,
    row: u32,
    width: u32,
    hp: u32,
    max_hp: u32,
) {
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

pub(crate) fn creature_tint(animation: BattleAnimation, participant: Participant) -> Rgba8 {
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

    fn finish(self) -> Result<Surface<ViewCell>, SurfaceError> {
        Surface::from_cells(GridSize::new(CANVAS_WIDTH, CANVAS_HEIGHT), self.cells)
    }
}

const fn sprite(tint: Rgba8) -> ViewCell {
    ViewCell::Fill(tint)
}
