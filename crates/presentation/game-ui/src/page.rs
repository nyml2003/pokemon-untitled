//! 玩家页面的键盘焦点和语义输入状态。

use std::time::Duration;

use game_page_model::{
    BagFilter, PageIntent, PageModel, PausePageModel, PokedexFilterIntent, PokedexMoveCategory,
    PokedexPageModel,
};
use punctum_input::{KeyEvent, KeyPhase, TextEvent};
use punctum_ui::{FormPresentation, KeyboardFormState, KeyboardSingleColumnFixedHeightScrollView};

use crate::{
    GameControl, MoveFilterItem, MoveFilterModel, PokedexFilterItem, PokedexFilterModel,
    move_filter_form, pokedex_filter_form,
};

/// 玩家页面当前可见的键盘焦点。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageFocus {
    World,
    PauseMenu(usize),
    Party(usize),
    BagCategory(usize),
    BagItem(usize),
    PokedexBrowse(usize),
    PokedexDetailFacts,
    PokedexDetailMoves(usize),
    TrainerCard,
    Shop(usize),
    SaveConfirm,
}

/// 页面输入处理后的纯结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageUiOutcome {
    Updated,
    Intent(PageIntent),
    Ignored,
}

/// 图鉴页面投影所需的双场景和详情内部模式。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PokedexScene {
    #[default]
    Browse,
    Detail,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PokedexDetailMode {
    #[default]
    Facts,
    Moves,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PokedexVisualState {
    pub scene: PokedexScene,
    pub scene_position: i32,
    pub detail_mode: PokedexDetailMode,
    pub wheel_position: i32,
    pub visible_entry_indices: Vec<usize>,
    pub visible_move_indices: Vec<usize>,
    pub filter_overlay: PokedexFilterOverlay,
    pub pokedex_filter: PokedexFilterModel,
    pub move_filter: MoveFilterModel,
    pub pokedex_ability_cursor: usize,
    pub move_accuracy_cursor: usize,
    pub form_scroll_y: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PokedexFilterOverlay {
    #[default]
    Compact,
    Pokedex(KeyboardFormState<PokedexFilterItem>),
    Moves(KeyboardFormState<MoveFilterItem>),
}

impl Default for PokedexVisualState {
    fn default() -> Self {
        Self {
            scene: PokedexScene::Browse,
            scene_position: 0,
            detail_mode: PokedexDetailMode::Facts,
            wheel_position: 0,
            visible_entry_indices: (0..386).collect(),
            visible_move_indices: (0..1024).collect(),
            filter_overlay: PokedexFilterOverlay::Compact,
            pokedex_filter: PokedexFilterModel::default(),
            move_filter: MoveFilterModel::default(),
            pokedex_ability_cursor: 0,
            move_accuracy_cursor: 0,
            form_scroll_y: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TrackMotion {
    current: i32,
    target: i32,
    start: i32,
    elapsed_ms: u32,
}

impl TrackMotion {
    fn set_target(&mut self, target: i32) {
        if target == self.target {
            return;
        }
        self.start = self.current;
        self.target = target;
        self.elapsed_ms = 0;
    }

    fn snap(&mut self, position: i32) {
        self.current = position;
        self.target = position;
        self.start = position;
        self.elapsed_ms = 0;
    }

    fn advance(&mut self, elapsed: Duration, duration_ms: i32) -> bool {
        if self.current == self.target {
            return false;
        }
        let duration_ms = u32::try_from(duration_ms.max(1)).map_or(u32::MAX, |value| value);
        let elapsed_ms = u32::try_from(elapsed.as_millis().min(50)).map_or(50, |value| value);
        self.elapsed_ms = self.elapsed_ms.saturating_add(elapsed_ms).min(duration_ms);
        if self.elapsed_ms == duration_ms {
            self.current = self.target;
        } else {
            let progress = i64::from(self.elapsed_ms) * 1000 / i64::from(duration_ms);
            let eased = ease_in_out_cubic(progress);
            let distance = i64::from(self.target) - i64::from(self.start);
            let position = i64::from(self.start) + distance * eased / 1000;
            self.current = position.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        }
        true
    }
}

fn ease_in_out_cubic(progress: i64) -> i64 {
    let progress = progress.clamp(0, 1000);
    if progress < 500 {
        4 * progress * progress * progress / 1_000_000
    } else {
        let remaining = 1000 - progress;
        1000 - 4 * remaining * remaining * remaining / 1_000_000
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PokedexMotion {
    scene: TrackMotion,
    wheel: TrackMotion,
}

impl PokedexMotion {
    const STEP: i32 = 1000;
    const SECTION_DURATION_MS: i32 = 280;
    const WHEEL_DURATION_MS: i32 = 200;

    const fn scene_position(scene: PokedexScene) -> i32 {
        match scene {
            PokedexScene::Browse => 0,
            PokedexScene::Detail => Self::STEP,
        }
    }

    fn set_scene_target(&mut self, scene: PokedexScene) {
        self.scene.set_target(Self::scene_position(scene));
    }

    fn set_wheel_target(&mut self, index: usize) {
        self.wheel.set_target(index_position(index));
    }

    fn snap(&mut self, scene: PokedexScene, index: usize) {
        self.scene.snap(Self::scene_position(scene));
        self.wheel.snap(index_position(index));
    }

    fn advance(&mut self, elapsed: Duration) -> bool {
        self.scene.advance(elapsed, Self::SECTION_DURATION_MS)
            | self.wheel.advance(elapsed, Self::WHEEL_DURATION_MS)
    }

    const fn active(self) -> bool {
        self.scene.current != self.scene.target || self.wheel.current != self.wheel.target
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PageKey {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Cancel,
    OpenPause,
    PreviousCategory,
    NextCategory,
    Backspace,
}

/// 不依赖窗口或渲染器的玩家页面输入状态。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageUiState {
    focus: PageFocus,
    pokedex_motion: PokedexMotion,
    pokedex_filter: PokedexFilterModel,
    move_filter: MoveFilterModel,
    pokedex_filter_state: KeyboardFormState<PokedexFilterItem>,
    move_filter_state: KeyboardFormState<MoveFilterItem>,
    filter_overlay: PokedexFilterOverlay,
    visible_entry_indices: Vec<usize>,
    visible_move_indices: Vec<usize>,
    pokedex_type_cursor: usize,
    pokedex_generation_cursor: usize,
    pokedex_ability_cursor: usize,
    move_type_cursor: usize,
    move_category_cursor: usize,
    move_accuracy_cursor: usize,
    form_scroll_y: u32,
}

impl Default for PageUiState {
    fn default() -> Self {
        Self {
            focus: PageFocus::World,
            pokedex_motion: PokedexMotion::default(),
            pokedex_filter: PokedexFilterModel::default(),
            move_filter: MoveFilterModel::default(),
            pokedex_filter_state: KeyboardFormState::default(),
            move_filter_state: KeyboardFormState::default(),
            filter_overlay: PokedexFilterOverlay::Compact,
            visible_entry_indices: Vec::new(),
            visible_move_indices: Vec::new(),
            pokedex_type_cursor: 0,
            pokedex_generation_cursor: 0,
            pokedex_ability_cursor: 0,
            move_type_cursor: 0,
            move_category_cursor: 0,
            move_accuracy_cursor: 0,
            form_scroll_y: 0,
        }
    }
}

impl PageUiState {
    pub const fn focus(&self) -> PageFocus {
        self.focus
    }

    pub const fn pokedex_scene(&self) -> PokedexScene {
        match self.focus {
            PageFocus::PokedexBrowse(_) => PokedexScene::Browse,
            PageFocus::PokedexDetailFacts | PageFocus::PokedexDetailMoves(_) => {
                PokedexScene::Detail
            }
            _ => PokedexScene::Browse,
        }
    }

    pub const fn pokedex_detail_mode(&self) -> PokedexDetailMode {
        match self.focus {
            PageFocus::PokedexDetailMoves(_) => PokedexDetailMode::Moves,
            _ => PokedexDetailMode::Facts,
        }
    }

    pub fn pokedex_visual_state(&self) -> PokedexVisualState {
        PokedexVisualState {
            scene: self.pokedex_scene(),
            scene_position: self.pokedex_motion.scene.current,
            detail_mode: self.pokedex_detail_mode(),
            wheel_position: self.pokedex_motion.wheel.current,
            visible_entry_indices: self.visible_entry_indices.clone(),
            visible_move_indices: self.visible_move_indices.clone(),
            filter_overlay: self.filter_overlay.clone(),
            pokedex_filter: self.pokedex_filter.clone(),
            move_filter: self.move_filter.clone(),
            pokedex_ability_cursor: self.pokedex_ability_cursor,
            move_accuracy_cursor: self.move_accuracy_cursor,
            form_scroll_y: self.form_scroll_y,
        }
    }

    /// 推进图鉴转场；返回 true 表示需要重新绘制。
    pub fn advance(&mut self, elapsed: Duration) -> bool {
        self.pokedex_motion.advance(elapsed)
            || self.pokedex_filter.advance(elapsed)
            || self.move_filter.advance(elapsed)
    }

    pub const fn pokedex_motion_active(&self) -> bool {
        self.pokedex_motion.active()
    }

    /// 返回筛选草稿应提交前的剩余等待时间。
    pub fn pokedex_filter_next_delay(&self) -> Option<Duration> {
        self.pokedex_filter
            .next_delay()
            .into_iter()
            .chain(self.move_filter.next_delay())
            .min()
    }

    fn set_pokedex_scene(&mut self, scene: PokedexScene) {
        self.pokedex_motion.set_scene_target(scene);
    }

    fn set_pokedex_wheel(&mut self, index: usize) {
        self.pokedex_motion.set_wheel_target(index);
    }

    fn snap_pokedex_state(&mut self, scene: PokedexScene, index: usize) {
        self.pokedex_motion.snap(scene, index);
    }

    pub fn sync(&mut self, model: &PageModel) {
        self.refresh_pokedex_filters(model);
        let next_focus = match (self.focus, model) {
            (PageFocus::PauseMenu(index), PageModel::Pause(PausePageModel::Menu)) => {
                PageFocus::PauseMenu(index.min(3))
            }
            (PageFocus::Party(index), PageModel::Pause(PausePageModel::Party(page))) => {
                PageFocus::Party(if page.members.is_empty() {
                    0
                } else {
                    index.min(page.members.len() - 1)
                })
            }
            (PageFocus::BagCategory(index), PageModel::Pause(PausePageModel::Bag(_))) => {
                PageFocus::BagCategory(index.min(3))
            }
            (PageFocus::BagItem(index), PageModel::Pause(PausePageModel::Bag(page))) => {
                if page.entries.is_empty() {
                    PageFocus::BagCategory(0)
                } else {
                    PageFocus::BagItem(index.min(page.entries.len() - 1))
                }
            }
            (PageFocus::PokedexBrowse(index), PageModel::Pause(PausePageModel::Pokedex(page))) => {
                let _ = page;
                PageFocus::PokedexBrowse(
                    index.min(self.visible_entry_indices.len().saturating_sub(1)),
                )
            }
            (PageFocus::PokedexDetailFacts, PageModel::Pause(PausePageModel::Pokedex(_))) => {
                PageFocus::PokedexDetailFacts
            }
            (
                PageFocus::PokedexDetailMoves(index),
                PageModel::Pause(PausePageModel::Pokedex(_)),
            ) => PageFocus::PokedexDetailMoves(
                index.min(self.visible_move_indices.len().saturating_sub(1)),
            ),
            (PageFocus::TrainerCard, PageModel::Pause(PausePageModel::TrainerCard(_))) => {
                PageFocus::TrainerCard
            }
            (PageFocus::Shop(index), PageModel::Shop(_)) => PageFocus::Shop(index.min(2)),
            (PageFocus::SaveConfirm, PageModel::SaveConfirm(_)) => PageFocus::SaveConfirm,
            (PageFocus::World, PageModel::World(_)) => PageFocus::World,
            _ => default_focus(model),
        };
        let entered_pokedex = !is_pokedex_focus(self.focus) && is_pokedex_focus(next_focus);
        self.focus = next_focus;
        if entered_pokedex {
            self.snap_pokedex_state(
                scene_for_focus(next_focus),
                self.selected_visible_entry_index(model),
            );
        }
    }

    fn refresh_pokedex_filters(&mut self, model: &PageModel) {
        let PageModel::Pause(PausePageModel::Pokedex(page)) = model else {
            self.visible_entry_indices.clear();
            self.visible_move_indices.clear();
            return;
        };
        self.visible_entry_indices = page
            .entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| self.pokedex_filter.matches(entry).then_some(index))
            .collect();
        self.visible_move_indices = page
            .moves
            .iter()
            .enumerate()
            .filter_map(|(index, item)| self.move_filter.matches(item).then_some(index))
            .collect();
    }

    fn selected_visible_entry_index(&self, model: &PageModel) -> usize {
        let PageModel::Pause(PausePageModel::Pokedex(page)) = model else {
            return 0;
        };
        self.selected_visible_entry_index_for(page).unwrap_or(0)
    }

    fn selected_visible_entry_index_for(&self, page: &PokedexPageModel) -> Option<usize> {
        self.visible_entry_indices.iter().position(|index| {
            page.entries
                .get(*index)
                .is_some_and(|entry| entry.number == page.selected.number)
        })
    }

    fn selected_visible_move_index(&self, page: &PokedexPageModel) -> Option<usize> {
        self.visible_move_indices
            .iter()
            .position(|index| *index == page.selected_move)
    }

    /// 根据已执行的页面 intent 同步鼠标点击和键盘焦点。
    pub fn focus_intent(&mut self, intent: &PageIntent, model: &PageModel) {
        match (intent, model) {
            (
                PageIntent::SelectPokedexEntry(number),
                PageModel::Pause(PausePageModel::Pokedex(page)),
            ) => {
                if let Some(index) = page
                    .entries
                    .iter()
                    .position(|entry| entry.number == *number)
                    .and_then(|index| {
                        self.visible_entry_indices
                            .iter()
                            .position(|visible| *visible == index)
                    })
                {
                    self.focus = match self.focus {
                        PageFocus::PokedexDetailFacts => PageFocus::PokedexDetailFacts,
                        PageFocus::PokedexDetailMoves(move_index) => {
                            PageFocus::PokedexDetailMoves(move_index)
                        }
                        _ => PageFocus::PokedexBrowse(index),
                    };
                    self.set_pokedex_scene(scene_for_focus(self.focus));
                    self.set_pokedex_wheel(index);
                }
            }
            (
                PageIntent::SelectPokedexMove(index),
                PageModel::Pause(PausePageModel::Pokedex(_)),
            ) => {
                let filtered_index = self
                    .visible_move_indices
                    .iter()
                    .position(|visible| *visible == *index)
                    .unwrap_or(0);
                self.focus = PageFocus::PokedexDetailMoves(filtered_index);
                self.set_pokedex_scene(PokedexScene::Detail);
            }
            _ => {}
        }
    }

    pub fn action_key(&self, model: &PageModel) -> Option<String> {
        match (self.focus, model) {
            (PageFocus::World, PageModel::World(_)) => None,
            (PageFocus::PauseMenu(index), PageModel::Pause(PausePageModel::Menu)) => [
                "page-pause-party",
                "page-pause-bag",
                "page-pause-pokedex",
                "page-pause-trainer-card",
            ]
            .get(index)
            .map(|key| String::from(*key)),
            (PageFocus::Party(index), PageModel::Pause(PausePageModel::Party(page))) => page
                .members
                .get(index)
                .map(|member| format!("page-party-{}", member.id.as_str())),
            (PageFocus::BagCategory(index), PageModel::Pause(PausePageModel::Bag(_))) => {
                bag_category_key(index).map(String::from)
            }
            (PageFocus::BagItem(index), PageModel::Pause(PausePageModel::Bag(page))) => page
                .entries
                .get(index)
                .map(|entry| format!("page-bag-{}", entry.item.as_str())),
            (PageFocus::PokedexBrowse(index), PageModel::Pause(PausePageModel::Pokedex(page))) => {
                self.visible_entry_indices
                    .get(index)
                    .and_then(|visible| page.entries.get(*visible))
                    .map(|entry| format!("page-pokedex-index-{}", entry.number.value()))
            }
            (PageFocus::PokedexDetailFacts, PageModel::Pause(PausePageModel::Pokedex(_))) => None,
            (
                PageFocus::PokedexDetailMoves(index),
                PageModel::Pause(PausePageModel::Pokedex(_)),
            ) => self
                .visible_move_indices
                .get(index)
                .map(|visible| format!("page-pokedex-move-{visible}")),
            (PageFocus::Shop(index), PageModel::Shop(_)) => {
                ["page-shop-less", "page-shop-more", "page-shop-confirm"]
                    .get(index)
                    .map(|key| String::from(*key))
            }
            (PageFocus::SaveConfirm, PageModel::SaveConfirm(_)) => {
                Some(String::from("page-save-confirm"))
            }
            _ => None,
        }
    }

    pub fn handle_key(&mut self, key: &KeyEvent, model: &PageModel) -> PageUiOutcome {
        self.handle_input(key, None, model)
    }

    pub fn handle_input(
        &mut self,
        key: &KeyEvent,
        text: Option<&TextEvent>,
        model: &PageModel,
    ) -> PageUiOutcome {
        self.sync(model);
        let control = GameControl::from_key_event(key);
        if key.phase == KeyPhase::Press
            && is_pokedex_focus(self.focus)
            && control == Some(GameControl::Select)
        {
            return self.toggle_filter_overlay();
        }
        if self.filter_is_expanded() {
            return self.handle_filter_input(key, control, text, model);
        }
        let Some(action) = page_key(key) else {
            return PageUiOutcome::Ignored;
        };
        if !matches!(key.phase, KeyPhase::Press | KeyPhase::Repeat) {
            return PageUiOutcome::Ignored;
        }
        match action {
            PageKey::OpenPause => match model {
                PageModel::World(_) if key.phase == KeyPhase::Press => {
                    PageUiOutcome::Intent(PageIntent::OpenPause)
                }
                _ => PageUiOutcome::Ignored,
            },
            PageKey::Cancel if key.phase == KeyPhase::Press => match model {
                PageModel::World(_) => PageUiOutcome::Ignored,
                _ => PageUiOutcome::Intent(PageIntent::Close),
            },
            PageKey::Confirm if key.phase == KeyPhase::Press => self.confirm(model),
            PageKey::PreviousCategory => self.change_bag_category(model, false),
            PageKey::NextCategory => self.change_bag_category(model, true),
            PageKey::Up | PageKey::Down | PageKey::Left | PageKey::Right => {
                self.move_focus(action, model)
            }
            PageKey::Backspace | PageKey::Confirm | PageKey::Cancel => PageUiOutcome::Ignored,
        }
    }

    pub fn handle_view_intent(
        &mut self,
        intent: &PageIntent,
        model: &PageModel,
    ) -> Option<PageUiOutcome> {
        match intent {
            PageIntent::TogglePokedexFilter if is_pokedex_focus(self.focus) => {
                Some(self.toggle_filter_overlay())
            }
            PageIntent::TogglePokedexAbilitySelect if is_pokedex_focus(self.focus) => {
                Some(self.toggle_pokedex_ability_select(model))
            }
            PageIntent::TogglePokedexMoveAccuracySelect if is_pokedex_focus(self.focus) => {
                Some(self.toggle_move_accuracy_select(model))
            }
            PageIntent::PokedexFilter(intent) if is_pokedex_focus(self.focus) => {
                Some(self.apply_filter_intent(intent, model))
            }
            _ => None,
        }
    }

    fn filter_is_expanded(&self) -> bool {
        matches!(
            self.filter_overlay,
            PokedexFilterOverlay::Pokedex(ref state)
                if state.presentation() == FormPresentation::Expanded
        ) || matches!(
            self.filter_overlay,
            PokedexFilterOverlay::Moves(ref state)
                if state.presentation() == FormPresentation::Expanded
        )
    }

    fn toggle_filter_overlay(&mut self) -> PageUiOutcome {
        match self.filter_overlay {
            PokedexFilterOverlay::Pokedex(_) => {
                self.pokedex_filter_state.compact();
                self.filter_overlay = PokedexFilterOverlay::Compact;
                self.form_scroll_y = 0;
                PageUiOutcome::Updated
            }
            PokedexFilterOverlay::Moves(_) => {
                self.move_filter_state.compact();
                self.filter_overlay = PokedexFilterOverlay::Compact;
                self.form_scroll_y = 0;
                PageUiOutcome::Updated
            }
            PokedexFilterOverlay::Compact => match self.focus {
                PageFocus::PokedexDetailMoves(_) => {
                    let Ok(form) = move_filter_form() else {
                        return PageUiOutcome::Ignored;
                    };
                    self.move_filter_state.expand(&form);
                    self.form_scroll_y = 0;
                    self.filter_overlay =
                        PokedexFilterOverlay::Moves(self.move_filter_state.clone());
                    PageUiOutcome::Updated
                }
                PageFocus::PokedexBrowse(_) | PageFocus::PokedexDetailFacts => {
                    let Ok(form) = pokedex_filter_form() else {
                        return PageUiOutcome::Ignored;
                    };
                    self.pokedex_filter_state.expand(&form);
                    self.form_scroll_y = 0;
                    self.filter_overlay =
                        PokedexFilterOverlay::Pokedex(self.pokedex_filter_state.clone());
                    PageUiOutcome::Updated
                }
                _ => PageUiOutcome::Ignored,
            },
        }
    }

    fn handle_filter_input(
        &mut self,
        key: &KeyEvent,
        control: Option<GameControl>,
        text: Option<&TextEvent>,
        model: &PageModel,
    ) -> PageUiOutcome {
        if !matches!(key.phase, KeyPhase::Press | KeyPhase::Repeat) {
            return PageUiOutcome::Ignored;
        }
        if let Some(text) = text
            && let Some(outcome) = self.apply_filter_text(text, model)
        {
            return outcome;
        }
        if control == Some(GameControl::L) {
            return self.move_filter_tab(true);
        }
        if control == Some(GameControl::R) {
            return self.move_filter_tab(false);
        }
        let Some(action) = page_key(key) else {
            return PageUiOutcome::Ignored;
        };
        if action == PageKey::Cancel && key.phase == KeyPhase::Press {
            return self.cancel_filter_input(model);
        }
        if action == PageKey::Backspace
            && key.phase == KeyPhase::Press
            && let Some(outcome) = self.delete_filter_text(model)
        {
            return outcome;
        }
        self.handle_filter_key(action, model)
    }

    fn move_filter_tab(&mut self, backwards: bool) -> PageUiOutcome {
        match self.filter_overlay {
            PokedexFilterOverlay::Pokedex(_) => {
                let Ok(form) = pokedex_filter_form() else {
                    return PageUiOutcome::Ignored;
                };
                let changed = if backwards {
                    self.pokedex_filter_state.focus_previous(&form)
                } else {
                    self.pokedex_filter_state.focus_next(&form)
                };
                self.filter_overlay =
                    PokedexFilterOverlay::Pokedex(self.pokedex_filter_state.clone());
                self.sync_form_scroll();
                if changed {
                    PageUiOutcome::Updated
                } else {
                    PageUiOutcome::Ignored
                }
            }
            PokedexFilterOverlay::Moves(_) => {
                let Ok(form) = move_filter_form() else {
                    return PageUiOutcome::Ignored;
                };
                let changed = if backwards {
                    self.move_filter_state.focus_previous(&form)
                } else {
                    self.move_filter_state.focus_next(&form)
                };
                self.filter_overlay = PokedexFilterOverlay::Moves(self.move_filter_state.clone());
                self.sync_form_scroll();
                if changed {
                    PageUiOutcome::Updated
                } else {
                    PageUiOutcome::Ignored
                }
            }
            PokedexFilterOverlay::Compact => PageUiOutcome::Ignored,
        }
    }

    fn apply_filter_text(&mut self, text: &TextEvent, model: &PageModel) -> Option<PageUiOutcome> {
        let intent = match &self.filter_overlay {
            PokedexFilterOverlay::Pokedex(state)
                if state.opened_select() == Some(&PokedexFilterItem::Ability) =>
            {
                PokedexFilterIntent::SetAbilityQuery(append_text(
                    self.pokedex_filter.ability_query(),
                    text.text(),
                ))
            }
            PokedexFilterOverlay::Pokedex(state) => match state.focused_item()? {
                PokedexFilterItem::HeightMinimum => PokedexFilterIntent::SetHeightMinimum(
                    append_number_text(self.pokedex_filter.height_draft().0, text.text()),
                ),
                PokedexFilterItem::HeightMaximum => PokedexFilterIntent::SetHeightMaximum(
                    append_number_text(self.pokedex_filter.height_draft().1, text.text()),
                ),
                PokedexFilterItem::WeightMinimum => PokedexFilterIntent::SetWeightMinimum(
                    append_number_text(self.pokedex_filter.weight_draft().0, text.text()),
                ),
                PokedexFilterItem::WeightMaximum => PokedexFilterIntent::SetWeightMaximum(
                    append_number_text(self.pokedex_filter.weight_draft().1, text.text()),
                ),
                _ => return None,
            },
            PokedexFilterOverlay::Moves(state) => match state.focused_item()? {
                MoveFilterItem::Name => PokedexFilterIntent::SetMoveName(append_text(
                    &self.move_filter.name_query,
                    text.text(),
                )),
                MoveFilterItem::PowerMinimum => PokedexFilterIntent::SetMovePowerMinimum(
                    append_number_text(self.move_filter.power_draft().0, text.text()),
                ),
                MoveFilterItem::PowerMaximum => PokedexFilterIntent::SetMovePowerMaximum(
                    append_number_text(self.move_filter.power_draft().1, text.text()),
                ),
                _ => return None,
            },
            PokedexFilterOverlay::Compact => return None,
        };
        Some(self.apply_filter_intent(&intent, model))
    }

    fn cancel_filter_input(&mut self, model: &PageModel) -> PageUiOutcome {
        let intent = match &self.filter_overlay {
            PokedexFilterOverlay::Pokedex(state)
                if state.opened_select() == Some(&PokedexFilterItem::Ability)
                    && !self.pokedex_filter.ability_query().is_empty() =>
            {
                Some(PokedexFilterIntent::SetAbilityQuery(String::new()))
            }
            PokedexFilterOverlay::Pokedex(state) => match state.focused_item() {
                Some(PokedexFilterItem::HeightMinimum)
                    if !self.pokedex_filter.height_draft().0.is_empty() =>
                {
                    Some(PokedexFilterIntent::SetHeightMinimum(String::new()))
                }
                Some(PokedexFilterItem::HeightMaximum)
                    if !self.pokedex_filter.height_draft().1.is_empty() =>
                {
                    Some(PokedexFilterIntent::SetHeightMaximum(String::new()))
                }
                Some(PokedexFilterItem::WeightMinimum)
                    if !self.pokedex_filter.weight_draft().0.is_empty() =>
                {
                    Some(PokedexFilterIntent::SetWeightMinimum(String::new()))
                }
                Some(PokedexFilterItem::WeightMaximum)
                    if !self.pokedex_filter.weight_draft().1.is_empty() =>
                {
                    Some(PokedexFilterIntent::SetWeightMaximum(String::new()))
                }
                _ => None,
            },
            PokedexFilterOverlay::Moves(state) => match state.focused_item() {
                Some(MoveFilterItem::Name) if !self.move_filter.name_query.is_empty() => {
                    Some(PokedexFilterIntent::SetMoveName(String::new()))
                }
                Some(MoveFilterItem::PowerMinimum)
                    if !self.move_filter.power_draft().0.is_empty() =>
                {
                    Some(PokedexFilterIntent::SetMovePowerMinimum(String::new()))
                }
                Some(MoveFilterItem::PowerMaximum)
                    if !self.move_filter.power_draft().1.is_empty() =>
                {
                    Some(PokedexFilterIntent::SetMovePowerMaximum(String::new()))
                }
                _ => None,
            },
            PokedexFilterOverlay::Compact => None,
        };
        if let Some(intent) = intent {
            return self.apply_filter_intent(&intent, model);
        }
        match self.filter_overlay {
            PokedexFilterOverlay::Pokedex(_) => {
                if self.pokedex_filter_state.close_select() {
                    self.filter_overlay =
                        PokedexFilterOverlay::Pokedex(self.pokedex_filter_state.clone());
                    PageUiOutcome::Updated
                } else {
                    self.pokedex_filter_state.compact();
                    self.filter_overlay = PokedexFilterOverlay::Compact;
                    PageUiOutcome::Updated
                }
            }
            PokedexFilterOverlay::Moves(_) => {
                if self.move_filter_state.close_select() {
                    self.filter_overlay =
                        PokedexFilterOverlay::Moves(self.move_filter_state.clone());
                    PageUiOutcome::Updated
                } else {
                    self.move_filter_state.compact();
                    self.filter_overlay = PokedexFilterOverlay::Compact;
                    PageUiOutcome::Updated
                }
            }
            PokedexFilterOverlay::Compact => PageUiOutcome::Ignored,
        }
    }

    fn handle_filter_key(&mut self, key: PageKey, model: &PageModel) -> PageUiOutcome {
        let overlay_before = self.filter_overlay.clone();
        let cursor_before = (
            self.pokedex_type_cursor,
            self.pokedex_generation_cursor,
            self.pokedex_ability_cursor,
            self.move_type_cursor,
            self.move_category_cursor,
            self.move_accuracy_cursor,
        );
        let intent = match &self.filter_overlay {
            PokedexFilterOverlay::Pokedex(state) => {
                let Some(item) = state.focused_item() else {
                    return PageUiOutcome::Ignored;
                };
                self.pokedex_filter_key(*item, key, model)
            }
            PokedexFilterOverlay::Moves(state) => {
                let Some(item) = state.focused_item() else {
                    return PageUiOutcome::Ignored;
                };
                self.move_filter_key(*item, key, model)
            }
            PokedexFilterOverlay::Compact => None,
        };
        if let Some(intent) = intent {
            return self.apply_filter_intent(&intent, model);
        }
        let cursor_after = (
            self.pokedex_type_cursor,
            self.pokedex_generation_cursor,
            self.pokedex_ability_cursor,
            self.move_type_cursor,
            self.move_category_cursor,
            self.move_accuracy_cursor,
        );
        if overlay_before != self.filter_overlay || cursor_before != cursor_after {
            PageUiOutcome::Updated
        } else {
            PageUiOutcome::Ignored
        }
    }

    fn pokedex_filter_key(
        &mut self,
        item: PokedexFilterItem,
        key: PageKey,
        model: &PageModel,
    ) -> Option<PokedexFilterIntent> {
        match item {
            PokedexFilterItem::Types => {
                let types = pokedex_type_options(model);
                self.pokedex_type_cursor = move_cursor(self.pokedex_type_cursor, types.len(), key)?;
                (key == PageKey::Confirm)
                    .then(|| types.get(self.pokedex_type_cursor).copied())
                    .flatten()
                    .map(PokedexFilterIntent::ToggleType)
            }
            PokedexFilterItem::TypeMatch => match key {
                PageKey::Left | PageKey::Up => Some(PokedexFilterIntent::SetTypeMatchAll(false)),
                PageKey::Right | PageKey::Down => Some(PokedexFilterIntent::SetTypeMatchAll(true)),
                PageKey::Confirm => Some(PokedexFilterIntent::SetTypeMatchAll(
                    self.pokedex_filter.type_match != crate::TypeMatch::All,
                )),
                _ => None,
            },
            PokedexFilterItem::Generations => {
                self.pokedex_generation_cursor =
                    move_cursor(self.pokedex_generation_cursor, 3, key)?;
                (key == PageKey::Confirm)
                    .then_some(self.pokedex_generation_cursor.saturating_add(1) as u8)
                    .map(PokedexFilterIntent::ToggleGeneration)
            }
            PokedexFilterItem::Ability if self.pokedex_filter_state.opened_select().is_some() => {
                let abilities = pokedex_ability_options(model, self.pokedex_filter.ability_query());
                if matches!(key, PageKey::Up | PageKey::Down) {
                    self.pokedex_ability_cursor =
                        move_cursor(self.pokedex_ability_cursor, abilities.len(), key)?;
                    return None;
                }
                (key == PageKey::Confirm)
                    .then(|| abilities.get(self.pokedex_ability_cursor).copied())
                    .flatten()
                    .map(PokedexFilterIntent::SelectAbility)
            }
            PokedexFilterItem::Ability if key == PageKey::Confirm => {
                let Ok(form) = pokedex_filter_form() else {
                    return None;
                };
                if self.pokedex_filter_state.opened_select().is_some() {
                    self.pokedex_filter_state.close_select();
                    self.filter_overlay =
                        PokedexFilterOverlay::Pokedex(self.pokedex_filter_state.clone());
                    None
                } else if self.pokedex_filter_state.open_select(&form, &item) {
                    self.filter_overlay =
                        PokedexFilterOverlay::Pokedex(self.pokedex_filter_state.clone());
                    None
                } else {
                    None
                }
            }
            PokedexFilterItem::Reset if key == PageKey::Confirm => {
                Some(PokedexFilterIntent::ResetPokedex)
            }
            PokedexFilterItem::HeightMinimum
            | PokedexFilterItem::HeightMaximum
            | PokedexFilterItem::WeightMinimum
            | PokedexFilterItem::WeightMaximum
            | PokedexFilterItem::Ability
            | PokedexFilterItem::Reset => None,
        }
    }

    fn move_filter_key(
        &mut self,
        item: MoveFilterItem,
        key: PageKey,
        model: &PageModel,
    ) -> Option<PokedexFilterIntent> {
        match item {
            MoveFilterItem::Types => {
                let types = move_type_options(model);
                self.move_type_cursor = move_cursor(self.move_type_cursor, types.len(), key)?;
                (key == PageKey::Confirm)
                    .then(|| types.get(self.move_type_cursor).copied())
                    .flatten()
                    .map(PokedexFilterIntent::ToggleMoveType)
            }
            MoveFilterItem::Category => {
                self.move_category_cursor = move_cursor(self.move_category_cursor, 4, key)?;
                (key == PageKey::Confirm).then(|| {
                    PokedexFilterIntent::SelectMoveCategory(move_category_at(
                        self.move_category_cursor,
                    ))
                })
            }
            MoveFilterItem::Accuracy if self.move_filter_state.opened_select().is_some() => {
                let accuracies = move_accuracy_options(model);
                if matches!(key, PageKey::Up | PageKey::Down) {
                    self.move_accuracy_cursor =
                        move_cursor(self.move_accuracy_cursor, accuracies.len(), key)?;
                    return None;
                }
                (key == PageKey::Confirm)
                    .then(|| accuracies.get(self.move_accuracy_cursor).copied())
                    .flatten()
                    .map(PokedexFilterIntent::SelectMoveAccuracy)
            }
            MoveFilterItem::Accuracy if key == PageKey::Confirm => {
                let Ok(form) = move_filter_form() else {
                    return None;
                };
                if self.move_filter_state.opened_select().is_some() {
                    self.move_filter_state.close_select();
                    self.filter_overlay =
                        PokedexFilterOverlay::Moves(self.move_filter_state.clone());
                    None
                } else if self.move_filter_state.open_select(&form, &item) {
                    self.filter_overlay =
                        PokedexFilterOverlay::Moves(self.move_filter_state.clone());
                    None
                } else {
                    None
                }
            }
            MoveFilterItem::Priority if key == PageKey::Confirm => {
                Some(PokedexFilterIntent::ToggleMovePriority)
            }
            MoveFilterItem::Reset if key == PageKey::Confirm => {
                Some(PokedexFilterIntent::ResetMove)
            }
            MoveFilterItem::Name
            | MoveFilterItem::PowerMinimum
            | MoveFilterItem::PowerMaximum
            | MoveFilterItem::Accuracy
            | MoveFilterItem::Priority
            | MoveFilterItem::Reset => None,
        }
    }

    fn apply_filter_intent(
        &mut self,
        intent: &PokedexFilterIntent,
        model: &PageModel,
    ) -> PageUiOutcome {
        match intent {
            PokedexFilterIntent::SelectAbility(_) => {
                self.pokedex_filter_state.close_select();
                self.filter_overlay =
                    PokedexFilterOverlay::Pokedex(self.pokedex_filter_state.clone());
            }
            PokedexFilterIntent::SelectMoveAccuracy(_) => {
                self.move_filter_state.close_select();
                self.filter_overlay = PokedexFilterOverlay::Moves(self.move_filter_state.clone());
            }
            _ => {}
        }
        let changed = self.pokedex_filter.apply(intent) || self.move_filter.apply(intent);
        if !changed {
            return PageUiOutcome::Updated;
        }
        self.refresh_pokedex_filters(model);
        self.filter_selection_outcome(model)
    }

    fn delete_filter_text(&mut self, model: &PageModel) -> Option<PageUiOutcome> {
        let intent = match &self.filter_overlay {
            PokedexFilterOverlay::Pokedex(state)
                if state.opened_select() == Some(&PokedexFilterItem::Ability) =>
            {
                Some(PokedexFilterIntent::SetAbilityQuery(remove_last(
                    self.pokedex_filter.ability_query(),
                )))
            }
            PokedexFilterOverlay::Pokedex(state) => match state.focused_item()? {
                PokedexFilterItem::HeightMinimum => Some(PokedexFilterIntent::SetHeightMinimum(
                    remove_last(self.pokedex_filter.height_draft().0),
                )),
                PokedexFilterItem::HeightMaximum => Some(PokedexFilterIntent::SetHeightMaximum(
                    remove_last(self.pokedex_filter.height_draft().1),
                )),
                PokedexFilterItem::WeightMinimum => Some(PokedexFilterIntent::SetWeightMinimum(
                    remove_last(self.pokedex_filter.weight_draft().0),
                )),
                PokedexFilterItem::WeightMaximum => Some(PokedexFilterIntent::SetWeightMaximum(
                    remove_last(self.pokedex_filter.weight_draft().1),
                )),
                _ => None,
            },
            PokedexFilterOverlay::Moves(state) => match state.focused_item()? {
                MoveFilterItem::Name => Some(PokedexFilterIntent::SetMoveName(remove_last(
                    &self.move_filter.name_query,
                ))),
                MoveFilterItem::PowerMinimum => Some(PokedexFilterIntent::SetMovePowerMinimum(
                    remove_last(self.move_filter.power_draft().0),
                )),
                MoveFilterItem::PowerMaximum => Some(PokedexFilterIntent::SetMovePowerMaximum(
                    remove_last(self.move_filter.power_draft().1),
                )),
                _ => None,
            },
            PokedexFilterOverlay::Compact => None,
        }?;
        Some(self.apply_filter_intent(&intent, model))
    }

    fn sync_form_scroll(&mut self) {
        let focus_index = match &self.filter_overlay {
            PokedexFilterOverlay::Pokedex(state) => state.focused_item().map(|item| match item {
                PokedexFilterItem::Types => 0,
                PokedexFilterItem::TypeMatch => 1,
                PokedexFilterItem::Generations => 2,
                PokedexFilterItem::HeightMinimum => 3,
                PokedexFilterItem::HeightMaximum => 4,
                PokedexFilterItem::WeightMinimum => 5,
                PokedexFilterItem::WeightMaximum => 6,
                PokedexFilterItem::Ability => 7,
                PokedexFilterItem::Reset => 8,
            }),
            PokedexFilterOverlay::Moves(state) => state.focused_item().map(|item| match item {
                MoveFilterItem::Name => 0,
                MoveFilterItem::Types => 1,
                MoveFilterItem::Category => 2,
                MoveFilterItem::PowerMinimum => 3,
                MoveFilterItem::PowerMaximum => 4,
                MoveFilterItem::Accuracy => 5,
                MoveFilterItem::Priority => 6,
                MoveFilterItem::Reset => 7,
            }),
            PokedexFilterOverlay::Compact => None,
        };
        self.form_scroll_y = focus_index
            .map(|index: usize| index.saturating_sub(4).saturating_mul(36))
            .map_or(0, |value| {
                u32::try_from(value).map_or(u32::MAX, |value| value)
            });
    }

    fn toggle_pokedex_ability_select(&mut self, model: &PageModel) -> PageUiOutcome {
        let Ok(form) = pokedex_filter_form() else {
            return PageUiOutcome::Ignored;
        };
        if self.pokedex_filter_state.opened_select().is_some() {
            self.pokedex_filter_state.close_select();
        } else if !self
            .pokedex_filter_state
            .open_select(&form, &PokedexFilterItem::Ability)
        {
            return PageUiOutcome::Ignored;
        } else {
            let abilities = pokedex_ability_options(model, self.pokedex_filter.ability_query());
            self.pokedex_ability_cursor = abilities
                .iter()
                .position(|ability| *ability == self.pokedex_filter.ability)
                .map_or(0, |index| index);
        }
        self.filter_overlay = PokedexFilterOverlay::Pokedex(self.pokedex_filter_state.clone());
        PageUiOutcome::Updated
    }

    fn toggle_move_accuracy_select(&mut self, model: &PageModel) -> PageUiOutcome {
        let Ok(form) = move_filter_form() else {
            return PageUiOutcome::Ignored;
        };
        if self.move_filter_state.opened_select().is_some() {
            self.move_filter_state.close_select();
        } else if !self
            .move_filter_state
            .open_select(&form, &MoveFilterItem::Accuracy)
        {
            return PageUiOutcome::Ignored;
        } else {
            let accuracies = move_accuracy_options(model);
            self.move_accuracy_cursor = accuracies
                .iter()
                .position(|accuracy| *accuracy == self.move_filter.accuracy)
                .map_or(0, |index| index);
        }
        self.filter_overlay = PokedexFilterOverlay::Moves(self.move_filter_state.clone());
        PageUiOutcome::Updated
    }

    fn filter_selection_outcome(&mut self, model: &PageModel) -> PageUiOutcome {
        let PageModel::Pause(PausePageModel::Pokedex(page)) = model else {
            return PageUiOutcome::Updated;
        };
        if self.selected_visible_entry_index_for(page).is_none() {
            let Some(index) = self.visible_entry_indices.first().copied() else {
                return PageUiOutcome::Updated;
            };
            let Some(entry) = page.entries.get(index) else {
                return PageUiOutcome::Updated;
            };
            self.focus = match self.focus {
                PageFocus::PokedexDetailMoves(_) => PageFocus::PokedexDetailMoves(0),
                PageFocus::PokedexDetailFacts => PageFocus::PokedexDetailFacts,
                _ => PageFocus::PokedexBrowse(0),
            };
            self.set_pokedex_wheel(0);
            return PageUiOutcome::Intent(PageIntent::SelectPokedexEntry(entry.number));
        }
        let selected_move_matches = self.visible_move_indices.contains(&page.selected_move);
        if matches!(self.focus, PageFocus::PokedexDetailMoves(_)) && !selected_move_matches {
            let Some(index) = self.visible_move_indices.first().copied() else {
                self.focus = PageFocus::PokedexDetailMoves(0);
                return PageUiOutcome::Updated;
            };
            self.focus = PageFocus::PokedexDetailMoves(0);
            return PageUiOutcome::Intent(PageIntent::SelectPokedexMove(index));
        }
        PageUiOutcome::Updated
    }

    fn confirm(&mut self, model: &PageModel) -> PageUiOutcome {
        match (self.focus, model) {
            (PageFocus::PauseMenu(index), PageModel::Pause(PausePageModel::Menu)) => [
                PageIntent::SelectPausePage(game_page_model::PausePage::Party),
                PageIntent::SelectPausePage(game_page_model::PausePage::Bag),
                PageIntent::SelectPausePage(game_page_model::PausePage::Pokedex),
                PageIntent::SelectPausePage(game_page_model::PausePage::TrainerCard),
            ]
            .get(index)
            .cloned()
            .map_or(PageUiOutcome::Ignored, PageUiOutcome::Intent),
            (PageFocus::Party(index), PageModel::Pause(PausePageModel::Party(page))) => page
                .members
                .get(index)
                .map(|member| {
                    PageUiOutcome::Intent(PageIntent::SelectPartyMember(member.id.clone()))
                })
                .map_or(PageUiOutcome::Ignored, |outcome| outcome),
            (PageFocus::BagItem(index), PageModel::Pause(PausePageModel::Bag(page))) => page
                .entries
                .get(index)
                .map(|entry| PageUiOutcome::Intent(PageIntent::SelectBagItem(entry.item.clone())))
                .unwrap_or(PageUiOutcome::Ignored),
            (PageFocus::BagCategory(_), PageModel::Pause(PausePageModel::Bag(_))) => {
                bag_filter(self.focus).map_or(PageUiOutcome::Ignored, |category| {
                    PageUiOutcome::Intent(PageIntent::SelectBagCategory(category))
                })
            }
            (PageFocus::PokedexBrowse(_), PageModel::Pause(PausePageModel::Pokedex(_))) => {
                self.focus = PageFocus::PokedexDetailFacts;
                self.set_pokedex_scene(PokedexScene::Detail);
                PageUiOutcome::Updated
            }
            (PageFocus::PokedexDetailFacts, PageModel::Pause(PausePageModel::Pokedex(_))) => {
                PageUiOutcome::Ignored
            }
            (
                PageFocus::PokedexDetailMoves(index),
                PageModel::Pause(PausePageModel::Pokedex(page)),
            ) => self
                .visible_move_indices
                .get(index)
                .and_then(|visible| page.moves.get(*visible).map(|_| *visible))
                .map(|visible| PageUiOutcome::Intent(PageIntent::SelectPokedexMove(visible)))
                .unwrap_or(PageUiOutcome::Ignored),
            (PageFocus::Shop(0), PageModel::Shop(page)) => page
                .selected_item
                .as_ref()
                .and_then(|item| item.quantity.checked_sub(1))
                .filter(|quantity| *quantity > 0)
                .map(|quantity| PageUiOutcome::Intent(PageIntent::SetShopQuantity(quantity)))
                .unwrap_or(PageUiOutcome::Ignored),
            (PageFocus::Shop(1), PageModel::Shop(page)) => page
                .selected_item
                .as_ref()
                .and_then(|item| item.quantity.checked_add(1))
                .map(|quantity| PageUiOutcome::Intent(PageIntent::SetShopQuantity(quantity)))
                .unwrap_or(PageUiOutcome::Ignored),
            (PageFocus::Shop(2), PageModel::Shop(page)) => page
                .selected_item
                .as_ref()
                .filter(|item| item.affordable)
                .map(|_| PageUiOutcome::Intent(PageIntent::ConfirmShopPurchase))
                .unwrap_or(PageUiOutcome::Ignored),
            (PageFocus::SaveConfirm, PageModel::SaveConfirm(page)) if page.available => {
                PageUiOutcome::Intent(PageIntent::ConfirmSave)
            }
            _ => PageUiOutcome::Ignored,
        }
    }

    fn change_bag_category(&mut self, model: &PageModel, next: bool) -> PageUiOutcome {
        let PageModel::Pause(PausePageModel::Bag(page)) = model else {
            return PageUiOutcome::Ignored;
        };
        let index = match self.focus {
            PageFocus::BagCategory(index) => index,
            _ => 0,
        };
        self.focus = PageFocus::BagCategory(step_index(index, 4, next));
        let Some(category) = bag_filter(self.focus) else {
            return PageUiOutcome::Ignored;
        };
        if page.category == category {
            PageUiOutcome::Updated
        } else {
            PageUiOutcome::Intent(PageIntent::SelectBagCategory(category))
        }
    }

    fn move_focus(&mut self, direction: PageKey, model: &PageModel) -> PageUiOutcome {
        match (self.focus, model) {
            (PageFocus::PauseMenu(index), PageModel::Pause(PausePageModel::Menu)) => {
                self.focus = PageFocus::PauseMenu(move_menu(index, direction));
                PageUiOutcome::Updated
            }
            (PageFocus::Party(index), PageModel::Pause(PausePageModel::Party(page))) => {
                let Some(next) = move_linear(index, page.members.len(), direction) else {
                    return PageUiOutcome::Ignored;
                };
                self.focus = PageFocus::Party(next);
                page.members
                    .get(next)
                    .map(|member| {
                        PageUiOutcome::Intent(PageIntent::SelectPartyMember(member.id.clone()))
                    })
                    .unwrap_or(PageUiOutcome::Ignored)
            }
            (PageFocus::BagCategory(index), PageModel::Pause(PausePageModel::Bag(_))) => {
                match direction {
                    PageKey::Left | PageKey::Right => {
                        self.focus = PageFocus::BagCategory(step_index(
                            index,
                            4,
                            direction == PageKey::Right,
                        ));
                        PageUiOutcome::Updated
                    }
                    PageKey::Down if !matches!(model, PageModel::Pause(PausePageModel::Bag(page)) if page.entries.is_empty()) =>
                    {
                        self.focus = PageFocus::BagItem(0);
                        PageUiOutcome::Updated
                    }
                    _ => PageUiOutcome::Ignored,
                }
            }
            (PageFocus::BagItem(index), PageModel::Pause(PausePageModel::Bag(page))) => {
                let Some(next) = move_grid(index, page.entries.len(), direction, 5) else {
                    if direction == PageKey::Up && index < 5 {
                        self.focus = PageFocus::BagCategory(0);
                        return PageUiOutcome::Updated;
                    }
                    return PageUiOutcome::Ignored;
                };
                self.focus = PageFocus::BagItem(next);
                page.entries
                    .get(next)
                    .map(|entry| {
                        PageUiOutcome::Intent(PageIntent::SelectBagItem(entry.item.clone()))
                    })
                    .unwrap_or(PageUiOutcome::Ignored)
            }
            (PageFocus::PokedexBrowse(index), PageModel::Pause(PausePageModel::Pokedex(page))) => {
                let mut scroll = KeyboardSingleColumnFixedHeightScrollView::new(
                    self.visible_entry_indices.len(),
                    POKEDEX_VISIBLE_ITEMS,
                    POKEDEX_ITEM_HEIGHT,
                );
                scroll.select(index);
                let changed = match direction {
                    PageKey::Up => scroll.move_up(),
                    PageKey::Down => scroll.move_down(),
                    PageKey::Right => {
                        self.focus = PageFocus::PokedexDetailFacts;
                        self.set_pokedex_scene(PokedexScene::Detail);
                        return PageUiOutcome::Updated;
                    }
                    _ => false,
                };
                if !changed {
                    return PageUiOutcome::Ignored;
                }
                let next = scroll.selected_index();
                self.focus = PageFocus::PokedexBrowse(next);
                self.set_pokedex_wheel(next);
                self.visible_entry_indices
                    .get(next)
                    .and_then(|visible| page.entries.get(*visible))
                    .map_or(PageUiOutcome::Updated, |entry| {
                        PageUiOutcome::Intent(PageIntent::SelectPokedexEntry(entry.number))
                    })
            }
            (PageFocus::PokedexDetailFacts, PageModel::Pause(PausePageModel::Pokedex(page))) => {
                match direction {
                    PageKey::Left => {
                        self.focus =
                            PageFocus::PokedexBrowse(self.selected_visible_entry_index(model));
                        self.set_pokedex_scene(PokedexScene::Browse);
                        PageUiOutcome::Updated
                    }
                    PageKey::Right => {
                        self.focus = PageFocus::PokedexDetailMoves(
                            self.selected_visible_move_index(page).unwrap_or(0),
                        );
                        PageUiOutcome::Updated
                    }
                    PageKey::Up | PageKey::Down => {
                        let mut scroll = KeyboardSingleColumnFixedHeightScrollView::new(
                            self.visible_entry_indices.len(),
                            POKEDEX_VISIBLE_ITEMS,
                            POKEDEX_ITEM_HEIGHT,
                        );
                        scroll.select(self.selected_visible_entry_index(model));
                        let changed = match direction {
                            PageKey::Up => scroll.move_up(),
                            PageKey::Down => scroll.move_down(),
                            _ => false,
                        };
                        if !changed {
                            return PageUiOutcome::Ignored;
                        }
                        let next = scroll.selected_index();
                        self.set_pokedex_wheel(next);
                        self.visible_entry_indices
                            .get(next)
                            .and_then(|visible| page.entries.get(*visible))
                            .map_or(PageUiOutcome::Updated, |entry| {
                                PageUiOutcome::Intent(PageIntent::SelectPokedexEntry(entry.number))
                            })
                    }
                    _ => PageUiOutcome::Ignored,
                }
            }
            (
                PageFocus::PokedexDetailMoves(index),
                PageModel::Pause(PausePageModel::Pokedex(_)),
            ) => {
                if direction == PageKey::Left {
                    self.focus = PageFocus::PokedexDetailFacts;
                    return PageUiOutcome::Updated;
                }
                let mut scroll = KeyboardSingleColumnFixedHeightScrollView::new(
                    self.visible_move_indices.len(),
                    POKEDEX_VISIBLE_ITEMS,
                    POKEDEX_MOVE_ITEM_HEIGHT,
                );
                scroll.select(index);
                let changed = match direction {
                    PageKey::Up => scroll.move_up(),
                    PageKey::Down => scroll.move_down(),
                    _ => false,
                };
                if !changed {
                    return PageUiOutcome::Ignored;
                }
                let next = scroll.selected_index();
                self.focus = PageFocus::PokedexDetailMoves(next);
                self.visible_move_indices
                    .get(next)
                    .map_or(PageUiOutcome::Updated, |visible| {
                        PageUiOutcome::Intent(PageIntent::SelectPokedexMove(*visible))
                    })
            }
            (PageFocus::Shop(index), PageModel::Shop(_)) => {
                let next = match direction {
                    PageKey::Left | PageKey::Up => index.checked_sub(1),
                    PageKey::Right | PageKey::Down => index.checked_add(1),
                    _ => None,
                };
                let Some(next) = next.filter(|next| *next < 3) else {
                    return PageUiOutcome::Ignored;
                };
                self.focus = PageFocus::Shop(next);
                PageUiOutcome::Updated
            }
            _ => PageUiOutcome::Ignored,
        }
    }
}

fn default_focus(model: &PageModel) -> PageFocus {
    match model {
        PageModel::World(_) => PageFocus::World,
        PageModel::Pause(PausePageModel::Menu) => PageFocus::PauseMenu(0),
        PageModel::Pause(PausePageModel::Party(page)) => PageFocus::Party(
            page.selected
                .as_ref()
                .and_then(|selected| {
                    page.members
                        .iter()
                        .position(|member| &member.id == selected)
                })
                .unwrap_or(0),
        ),
        PageModel::Pause(PausePageModel::Bag(page)) => page
            .selected
            .as_ref()
            .and_then(|selected| {
                page.entries
                    .iter()
                    .position(|entry| &entry.item == selected)
            })
            .map(PageFocus::BagItem)
            .unwrap_or(PageFocus::BagCategory(0)),
        PageModel::Pause(PausePageModel::Pokedex(page)) => PageFocus::PokedexBrowse(
            page.entries
                .iter()
                .position(|entry| entry.number == page.selected.number)
                .map_or(0, |index| index),
        ),
        PageModel::Pause(PausePageModel::TrainerCard(_)) => PageFocus::TrainerCard,
        PageModel::Shop(_) => PageFocus::Shop(2),
        PageModel::SaveConfirm(_) => PageFocus::SaveConfirm,
    }
}

fn is_pokedex_focus(focus: PageFocus) -> bool {
    matches!(
        focus,
        PageFocus::PokedexBrowse(_)
            | PageFocus::PokedexDetailFacts
            | PageFocus::PokedexDetailMoves(_)
    )
}

fn scene_for_focus(focus: PageFocus) -> PokedexScene {
    match focus {
        PageFocus::PokedexBrowse(_) => PokedexScene::Browse,
        PageFocus::PokedexDetailFacts | PageFocus::PokedexDetailMoves(_) => PokedexScene::Detail,
        _ => PokedexScene::Browse,
    }
}

fn index_position(index: usize) -> i32 {
    i32::try_from(index).map_or(i32::MAX, |value| value.saturating_mul(PokedexMotion::STEP))
}

fn page_key(key: &KeyEvent) -> Option<PageKey> {
    match GameControl::from_key_event(key)? {
        GameControl::Up => Some(PageKey::Up),
        GameControl::Down => Some(PageKey::Down),
        GameControl::Left => Some(PageKey::Left),
        GameControl::Right => Some(PageKey::Right),
        GameControl::A => Some(PageKey::Confirm),
        GameControl::B => Some(PageKey::Cancel),
        GameControl::L => Some(PageKey::PreviousCategory),
        GameControl::R => Some(PageKey::NextCategory),
        GameControl::Start => Some(PageKey::OpenPause),
        GameControl::Select => None,
    }
}

fn append_text(current: &str, text: &str) -> String {
    let mut value = String::from(current);
    value.push_str(text);
    value
}

fn append_number_text(current: &str, text: &str) -> String {
    let mut value = String::from(current);
    for character in text.chars() {
        if character.is_ascii_digit() || (character == '.' && !value.contains('.')) {
            value.push(character);
        }
    }
    value
}

fn remove_last(value: &str) -> String {
    let mut next = String::from(value);
    next.pop();
    next
}

fn pokedex_type_options(model: &PageModel) -> Vec<game_data::TypeId> {
    let PageModel::Pause(PausePageModel::Pokedex(page)) = model else {
        return Vec::new();
    };
    page.entries
        .iter()
        .filter(|entry| entry.known)
        .flat_map(|entry| entry.type_ids.iter().copied())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn move_type_options(model: &PageModel) -> Vec<game_data::TypeId> {
    let PageModel::Pause(PausePageModel::Pokedex(page)) = model else {
        return Vec::new();
    };
    page.moves
        .iter()
        .map(|item| item.type_id)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn pokedex_ability_options(model: &PageModel, query: &str) -> Vec<Option<game_data::AbilityId>> {
    let PageModel::Pause(PausePageModel::Pokedex(page)) = model else {
        return Vec::new();
    };
    let mut abilities = page
        .entries
        .iter()
        .filter(|entry| entry.known)
        .flat_map(|entry| entry.abilities.iter())
        .filter(|ability| ability.name.contains(query))
        .map(|ability| ability.id)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(Some)
        .collect::<Vec<_>>();
    abilities.insert(0, None);
    abilities
}

fn move_accuracy_options(model: &PageModel) -> Vec<Option<Option<u8>>> {
    let PageModel::Pause(PausePageModel::Pokedex(page)) = model else {
        return Vec::new();
    };
    let mut accuracies = page
        .moves
        .iter()
        .map(|item| item.accuracy)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(Some)
        .collect::<Vec<_>>();
    accuracies.insert(0, None);
    accuracies
}

fn move_category_at(index: usize) -> Option<PokedexMoveCategory> {
    [
        None,
        Some(PokedexMoveCategory::Physical),
        Some(PokedexMoveCategory::Special),
        Some(PokedexMoveCategory::Status),
    ]
    .get(index)
    .copied()
    .flatten()
}

fn move_cursor(index: usize, length: usize, key: PageKey) -> Option<usize> {
    if length == 0 {
        return None;
    }
    match key {
        PageKey::Left | PageKey::Up => index.checked_sub(1),
        PageKey::Right | PageKey::Down => index.checked_add(1).filter(|next| *next < length),
        PageKey::Confirm => Some(index.min(length - 1)),
        _ => None,
    }
}

const POKEDEX_VISIBLE_ITEMS: usize = 7;
const POKEDEX_ITEM_HEIGHT: u32 = 52;
const POKEDEX_MOVE_ITEM_HEIGHT: u32 = 44;

fn move_menu(index: usize, direction: PageKey) -> usize {
    let row = index / 2;
    let column = index % 2;
    match direction {
        PageKey::Left if column > 0 => index - 1,
        PageKey::Right if column < 1 => index + 1,
        PageKey::Up if row > 0 => index - 2,
        PageKey::Down if row < 1 => index + 2,
        _ => index,
    }
}

fn move_linear(index: usize, length: usize, direction: PageKey) -> Option<usize> {
    if length == 0 {
        return None;
    }
    match direction {
        PageKey::Left | PageKey::Up => index.checked_sub(1),
        PageKey::Right | PageKey::Down => index.checked_add(1).filter(|next| *next < length),
        _ => None,
    }
}

fn move_grid(index: usize, length: usize, direction: PageKey, columns: usize) -> Option<usize> {
    if length == 0 {
        return None;
    }
    let candidate = match direction {
        PageKey::Left => index.checked_sub(1),
        PageKey::Right => index.checked_add(1),
        PageKey::Up => index.checked_sub(columns),
        PageKey::Down => index.checked_add(columns),
        _ => None,
    }?;
    (candidate < length).then_some(candidate)
}

fn step_index(index: usize, length: usize, next: bool) -> usize {
    if next {
        index.saturating_add(1).min(length.saturating_sub(1))
    } else {
        index.saturating_sub(1)
    }
}

fn bag_filter(focus: PageFocus) -> Option<BagFilter> {
    let PageFocus::BagCategory(index) = focus else {
        return None;
    };
    Some(match index {
        0 => BagFilter::All,
        1 => BagFilter::Category(game_foundation::ItemCategory::Medicine),
        2 => BagFilter::Category(game_foundation::ItemCategory::Key),
        3 => BagFilter::Category(game_foundation::ItemCategory::General),
        _ => return None,
    })
}

fn bag_category_key(index: usize) -> Option<&'static str> {
    [
        "page-bag-category-all",
        "page-bag-category-medicine",
        "page-bag-category-key",
        "page-bag-category-general",
    ]
    .get(index)
    .copied()
}
