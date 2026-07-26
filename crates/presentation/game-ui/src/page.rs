//! 玩家页面的键盘焦点和语义输入状态。

use std::time::Duration;

use game_page_model::{
    BagFilter, PageIntent, PageModel, PausePageModel, PokedexDetailView, PokedexSection,
};
use punctum_input::{KeyEvent, KeyPhase, LogicalKey, NamedKey, PhysicalKeyCode};
use punctum_ui::KeyboardSingleColumnFixedHeightScrollView;

/// 玩家页面当前可见的键盘焦点。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageFocus {
    World,
    PauseMenu(usize),
    Party(usize),
    BagCategory(usize),
    BagItem(usize),
    Pokedex(usize),
    PokedexDetail,
    PokedexStats,
    PokedexMoves(usize),
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

/// 图鉴页面投影所需的逻辑层级和连续轨道位置。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PokedexVisualState {
    pub section: PokedexSection,
    pub position: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PokedexMotion {
    current: i32,
    target: i32,
}

impl PokedexMotion {
    const SECTION_STEP: i32 = 1000;

    const fn position(section: PokedexSection) -> i32 {
        match section {
            PokedexSection::Index => 0,
            PokedexSection::Detail => Self::SECTION_STEP,
            PokedexSection::Stats => Self::SECTION_STEP * 2,
            PokedexSection::Moves => Self::SECTION_STEP * 3,
        }
    }

    fn set_target(&mut self, section: PokedexSection) {
        self.target = Self::position(section);
    }

    fn snap(&mut self, section: PokedexSection) {
        let position = Self::position(section);
        self.current = position;
        self.target = position;
    }

    fn advance(&mut self, elapsed: Duration) -> bool {
        if self.current == self.target {
            return false;
        }
        let milliseconds = elapsed.as_millis().min(50) as i32;
        let distance = self.target.saturating_sub(self.current);
        let step = ((i64::from(distance.unsigned_abs()) * i64::from(milliseconds)) / 220)
            .clamp(1, i64::from(distance.unsigned_abs())) as i32;
        if distance.unsigned_abs() <= step as u32 {
            self.current = self.target;
        } else {
            self.current = self.current.saturating_add(distance.signum() * step);
        }
        true
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
    OpenSave,
    PreviousCategory,
    NextCategory,
    Home,
    End,
}

/// 不依赖窗口或渲染器的玩家页面输入状态。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageUiState {
    focus: PageFocus,
    pokedex_motion: PokedexMotion,
}

impl Default for PageUiState {
    fn default() -> Self {
        Self {
            focus: PageFocus::World,
            pokedex_motion: PokedexMotion::default(),
        }
    }
}

impl PageUiState {
    pub const fn focus(self) -> PageFocus {
        self.focus
    }

    pub const fn pokedex_section(self) -> PokedexSection {
        match self.focus {
            PageFocus::Pokedex(_) => PokedexSection::Index,
            PageFocus::PokedexDetail => PokedexSection::Detail,
            PageFocus::PokedexStats => PokedexSection::Stats,
            PageFocus::PokedexMoves(_) => PokedexSection::Moves,
            _ => PokedexSection::Index,
        }
    }

    pub const fn pokedex_visual_state(self) -> PokedexVisualState {
        PokedexVisualState {
            section: self.pokedex_section(),
            position: self.pokedex_motion.current,
        }
    }

    /// 推进图鉴转场；返回 true 表示需要重新绘制。
    pub fn advance(&mut self, elapsed: Duration) -> bool {
        self.pokedex_motion.advance(elapsed)
    }

    pub const fn pokedex_motion_active(self) -> bool {
        self.pokedex_motion.current != self.pokedex_motion.target
    }

    fn set_pokedex_section(&mut self, section: PokedexSection) {
        self.pokedex_motion.set_target(section);
    }

    fn snap_pokedex_section(&mut self, section: PokedexSection) {
        self.pokedex_motion.snap(section);
    }

    pub fn sync(&mut self, model: &PageModel) {
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
            (PageFocus::Pokedex(index), PageModel::Pause(PausePageModel::Pokedex(page))) => {
                PageFocus::Pokedex(index.min(page.entries.len().saturating_sub(1)))
            }
            (PageFocus::PokedexDetail, PageModel::Pause(PausePageModel::Pokedex(_))) => {
                PageFocus::PokedexDetail
            }
            (PageFocus::PokedexStats, PageModel::Pause(PausePageModel::Pokedex(_))) => {
                PageFocus::PokedexStats
            }
            (PageFocus::PokedexMoves(index), PageModel::Pause(PausePageModel::Pokedex(page))) => {
                PageFocus::PokedexMoves(index.min(page.moves.len().saturating_sub(1)))
            }
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
            self.snap_pokedex_section(section_for_focus(next_focus));
        }
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
                {
                    self.focus = PageFocus::Pokedex(index);
                    self.set_pokedex_section(section_for_focus(self.focus));
                }
            }
            (
                PageIntent::SelectPokedexDetail(detail_view),
                PageModel::Pause(PausePageModel::Pokedex(page)),
            ) => {
                self.focus = match detail_view {
                    PokedexDetailView::Overview => PageFocus::PokedexDetail,
                    PokedexDetailView::Moves => PageFocus::PokedexMoves(page.selected_move),
                };
                self.set_pokedex_section(section_for_focus(self.focus));
            }
            (
                PageIntent::SelectPokedexMove(index),
                PageModel::Pause(PausePageModel::Pokedex(page)),
            ) => {
                self.focus =
                    PageFocus::PokedexMoves((*index).min(page.moves.len().saturating_sub(1)));
                self.set_pokedex_section(PokedexSection::Moves);
            }
            (PageIntent::TogglePokedexStatsView, PageModel::Pause(PausePageModel::Pokedex(_))) => {
                self.focus = PageFocus::PokedexStats;
                self.set_pokedex_section(PokedexSection::Stats);
            }
            _ => {}
        }
    }

    pub fn action_key(self, model: &PageModel) -> Option<String> {
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
            (PageFocus::Pokedex(index), PageModel::Pause(PausePageModel::Pokedex(page))) => page
                .entries
                .get(index)
                .map(|entry| format!("page-pokedex-index-{}", entry.number.value())),
            (PageFocus::PokedexStats, PageModel::Pause(PausePageModel::Pokedex(_))) => {
                Some(String::from("page-pokedex-stats-toggle"))
            }
            (PageFocus::PokedexDetail, PageModel::Pause(PausePageModel::Pokedex(_))) => None,
            (PageFocus::PokedexMoves(index), PageModel::Pause(PausePageModel::Pokedex(_))) => {
                Some(format!("page-pokedex-move-{index}"))
            }
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
        self.sync(model);
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
            PageKey::OpenSave => match model {
                PageModel::World(world) if key.phase == KeyPhase::Press && world.save_available => {
                    PageUiOutcome::Intent(PageIntent::OpenSaveConfirm)
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
            PageKey::Up
            | PageKey::Down
            | PageKey::Left
            | PageKey::Right
            | PageKey::Home
            | PageKey::End => self.move_focus(action, model),
            PageKey::Confirm | PageKey::Cancel => PageUiOutcome::Ignored,
        }
    }

    fn confirm(&self, model: &PageModel) -> PageUiOutcome {
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
            (PageFocus::Pokedex(index), PageModel::Pause(PausePageModel::Pokedex(page))) => page
                .entries
                .get(index)
                .map(|entry| PageUiOutcome::Intent(PageIntent::SelectPokedexEntry(entry.number)))
                .unwrap_or(PageUiOutcome::Ignored),
            (PageFocus::PokedexStats, PageModel::Pause(PausePageModel::Pokedex(_))) => {
                PageUiOutcome::Intent(PageIntent::TogglePokedexStatsView)
            }
            (PageFocus::PokedexMoves(index), PageModel::Pause(PausePageModel::Pokedex(page))) => {
                page.moves
                    .get(index)
                    .map(|_| PageUiOutcome::Intent(PageIntent::SelectPokedexMove(index)))
                    .unwrap_or(PageUiOutcome::Ignored)
            }
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
            (PageFocus::Pokedex(index), PageModel::Pause(PausePageModel::Pokedex(page))) => {
                let mut scroll = KeyboardSingleColumnFixedHeightScrollView::new(
                    page.entries.len(),
                    POKEDEX_VISIBLE_ITEMS,
                    POKEDEX_ITEM_HEIGHT,
                );
                scroll.select(index);
                let changed = match direction {
                    PageKey::Up => scroll.move_up(),
                    PageKey::Down => scroll.move_down(),
                    PageKey::Home => scroll.move_to_top(),
                    PageKey::End => scroll.move_to_bottom(),
                    PageKey::Right => {
                        self.focus = PageFocus::PokedexDetail;
                        self.set_pokedex_section(PokedexSection::Detail);
                        return PageUiOutcome::Updated;
                    }
                    _ => false,
                };
                if !changed {
                    return PageUiOutcome::Ignored;
                }
                let next = scroll.selected_index();
                self.focus = PageFocus::Pokedex(next);
                page.entries
                    .get(next)
                    .map_or(PageUiOutcome::Updated, |entry| {
                        PageUiOutcome::Intent(PageIntent::SelectPokedexEntry(entry.number))
                    })
            }
            (PageFocus::PokedexDetail, PageModel::Pause(PausePageModel::Pokedex(_))) => {
                match direction {
                    PageKey::Left => {
                        self.focus = PageFocus::Pokedex(current_pokedex_index(model));
                        self.set_pokedex_section(PokedexSection::Index);
                        PageUiOutcome::Updated
                    }
                    PageKey::Right => {
                        self.focus = PageFocus::PokedexStats;
                        self.set_pokedex_section(PokedexSection::Stats);
                        PageUiOutcome::Updated
                    }
                    _ => PageUiOutcome::Ignored,
                }
            }
            (PageFocus::PokedexStats, PageModel::Pause(PausePageModel::Pokedex(page))) => {
                if direction == PageKey::Left {
                    self.focus = PageFocus::PokedexDetail;
                    self.set_pokedex_section(PokedexSection::Detail);
                    PageUiOutcome::Updated
                } else if direction == PageKey::Right {
                    self.focus = PageFocus::PokedexMoves(page.selected_move);
                    self.set_pokedex_section(PokedexSection::Moves);
                    PageUiOutcome::Intent(PageIntent::SelectPokedexDetail(PokedexDetailView::Moves))
                } else {
                    PageUiOutcome::Ignored
                }
            }
            (PageFocus::PokedexMoves(index), PageModel::Pause(PausePageModel::Pokedex(page))) => {
                if direction == PageKey::Left {
                    self.focus = PageFocus::PokedexStats;
                    self.set_pokedex_section(PokedexSection::Stats);
                    return PageUiOutcome::Updated;
                }
                let mut scroll = KeyboardSingleColumnFixedHeightScrollView::new(
                    page.moves.len(),
                    POKEDEX_VISIBLE_ITEMS,
                    POKEDEX_MOVE_ITEM_HEIGHT,
                );
                scroll.select(index);
                let changed = match direction {
                    PageKey::Up => scroll.move_up(),
                    PageKey::Down => scroll.move_down(),
                    PageKey::Home => scroll.move_to_top(),
                    PageKey::End => scroll.move_to_bottom(),
                    _ => false,
                };
                if !changed {
                    return PageUiOutcome::Ignored;
                }
                let next = scroll.selected_index();
                self.focus = PageFocus::PokedexMoves(next);
                PageUiOutcome::Intent(PageIntent::SelectPokedexMove(next))
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
        PageModel::Pause(PausePageModel::Pokedex(page)) => PageFocus::Pokedex(
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
        PageFocus::Pokedex(_)
            | PageFocus::PokedexDetail
            | PageFocus::PokedexStats
            | PageFocus::PokedexMoves(_)
    )
}

fn section_for_focus(focus: PageFocus) -> PokedexSection {
    match focus {
        PageFocus::Pokedex(_) => PokedexSection::Index,
        PageFocus::PokedexDetail => PokedexSection::Detail,
        PageFocus::PokedexStats => PokedexSection::Stats,
        PageFocus::PokedexMoves(_) => PokedexSection::Moves,
        _ => PokedexSection::Index,
    }
}

fn current_pokedex_index(model: &PageModel) -> usize {
    let PageModel::Pause(PausePageModel::Pokedex(page)) = model else {
        return 0;
    };
    page.entries
        .iter()
        .position(|entry| entry.number == page.selected.number)
        .unwrap_or(0)
}

fn page_key(key: &KeyEvent) -> Option<PageKey> {
    let physical = key.physical;
    match key.logical {
        LogicalKey::Named(NamedKey::ArrowUp) => Some(PageKey::Up),
        LogicalKey::Named(NamedKey::ArrowDown) => Some(PageKey::Down),
        LogicalKey::Named(NamedKey::ArrowLeft) => Some(PageKey::Left),
        LogicalKey::Named(NamedKey::ArrowRight) => Some(PageKey::Right),
        LogicalKey::Named(NamedKey::Home) => Some(PageKey::Home),
        LogicalKey::Named(NamedKey::End) => Some(PageKey::End),
        LogicalKey::Named(NamedKey::Enter) => Some(PageKey::Confirm),
        LogicalKey::Named(NamedKey::Escape) => Some(PageKey::Cancel),
        LogicalKey::Named(NamedKey::Tab) => Some(PageKey::OpenPause),
        LogicalKey::Named(NamedKey::Function(5)) => Some(PageKey::OpenSave),
        _ if physical == Some(PhysicalKeyCode::ArrowUp)
            || physical == Some(PhysicalKeyCode::KeyW) =>
        {
            Some(PageKey::Up)
        }
        _ if physical == Some(PhysicalKeyCode::ArrowDown)
            || physical == Some(PhysicalKeyCode::KeyS) =>
        {
            Some(PageKey::Down)
        }
        _ if physical == Some(PhysicalKeyCode::ArrowLeft)
            || physical == Some(PhysicalKeyCode::KeyA) =>
        {
            Some(PageKey::Left)
        }
        _ if physical == Some(PhysicalKeyCode::ArrowRight)
            || physical == Some(PhysicalKeyCode::KeyD) =>
        {
            Some(PageKey::Right)
        }
        _ if physical == Some(PhysicalKeyCode::Home) => Some(PageKey::Home),
        _ if physical == Some(PhysicalKeyCode::End) => Some(PageKey::End),
        _ if physical == Some(PhysicalKeyCode::Enter) || character_is(key, "z") => {
            Some(PageKey::Confirm)
        }
        _ if physical == Some(PhysicalKeyCode::Escape) || character_is(key, "x") => {
            Some(PageKey::Cancel)
        }
        _ if physical == Some(PhysicalKeyCode::Tab) => Some(PageKey::OpenPause),
        _ if physical == Some(PhysicalKeyCode::F5) => Some(PageKey::OpenSave),
        _ if physical == Some(PhysicalKeyCode::KeyQ) => Some(PageKey::PreviousCategory),
        _ if physical == Some(PhysicalKeyCode::KeyE) => Some(PageKey::NextCategory),
        _ => None,
    }
}

const POKEDEX_VISIBLE_ITEMS: usize = 7;
const POKEDEX_ITEM_HEIGHT: u32 = 52;
const POKEDEX_MOVE_ITEM_HEIGHT: u32 = 44;

fn character_is(key: &KeyEvent, expected: &str) -> bool {
    matches!(&key.logical, LogicalKey::Character(value) if value.eq_ignore_ascii_case(expected))
}

fn move_menu(index: usize, direction: PageKey) -> usize {
    let row = index / 2;
    let column = index % 2;
    match direction {
        PageKey::Left => row * 2 + (column + 1) % 2,
        PageKey::Right => row * 2 + (column + 1) % 2,
        PageKey::Up => ((row + 1) % 2) * 2 + column,
        PageKey::Down => ((row + 1) % 2) * 2 + column,
        _ => index,
    }
}

fn move_linear(index: usize, length: usize, direction: PageKey) -> Option<usize> {
    if length == 0 {
        return None;
    }
    match direction {
        PageKey::Left | PageKey::Up => Some((index + length - 1) % length),
        PageKey::Right | PageKey::Down => Some((index + 1) % length),
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
        (index + 1) % length
    } else {
        (index + length - 1) % length
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
