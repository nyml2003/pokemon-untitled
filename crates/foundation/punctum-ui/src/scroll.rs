use std::ops::Range;

use crate::{Dimension, FlexDirection, Position, UiNode, UiStyle};

/// 固定 item 高度的单列键盘滚动窗口。
///
/// 状态只保存游标和窗口起点，树构建时只需要传入 `render_range` 对应的节点。
/// `overscan` 节点会在视口外提前建立，连续按键时不会构建整列数据。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyboardSingleColumnFixedHeightScrollView {
    item_count: usize,
    selected_index: usize,
    first_visible: usize,
    visible_items: usize,
    item_height: u32,
    gap: u32,
    overscan: usize,
}

impl KeyboardSingleColumnFixedHeightScrollView {
    /// 创建一个固定可见 item 数量的单列滚动窗口。
    pub fn new(item_count: usize, visible_items: usize, item_height: u32) -> Self {
        Self {
            item_count,
            selected_index: 0,
            first_visible: 0,
            visible_items: visible_items.max(1),
            item_height: item_height.max(1),
            gap: 0,
            overscan: 1,
        }
    }

    /// 设置相邻 item 之间的固定像素间距。
    pub const fn with_gap(mut self, gap: u32) -> Self {
        self.gap = gap;
        self
    }

    /// 设置视口外预构建的 item 数量。
    pub const fn with_overscan(mut self, overscan: usize) -> Self {
        self.overscan = overscan;
        self
    }

    pub const fn item_count(self) -> usize {
        self.item_count
    }

    pub const fn selected_index(self) -> usize {
        self.selected_index
    }

    pub const fn first_visible(self) -> usize {
        self.first_visible
    }

    /// 返回严格位于视口内的 item 索引。
    pub fn visible_range(self) -> Range<usize> {
        let end = self
            .first_visible
            .saturating_add(self.visible_items)
            .min(self.item_count);
        self.first_visible.min(end)..end
    }

    /// 返回需要构建的 item 索引，包含视口下方的预构建缓冲。
    pub fn render_range(self) -> Range<usize> {
        let visible = self.visible_range();
        let end = visible
            .end
            .saturating_add(self.overscan)
            .min(self.item_count);
        visible.start..end
    }

    /// 同步数据源数量，并把游标和窗口限制在有效范围内。
    pub fn set_item_count(&mut self, item_count: usize) -> bool {
        let old = (self.item_count, self.selected_index, self.first_visible);
        self.item_count = item_count;
        if item_count == 0 {
            self.selected_index = 0;
            self.first_visible = 0;
        } else {
            self.selected_index = self.selected_index.min(item_count - 1);
            self.first_visible = self.first_visible.min(self.max_first_visible());
            self.ensure_selected_visible();
        }
        old != (self.item_count, self.selected_index, self.first_visible)
    }

    /// 直接同步游标，适用于页面模型切换到指定 item 的场景。
    pub fn select(&mut self, index: usize) -> bool {
        let next = index.min(self.item_count.saturating_sub(1));
        let old = (self.selected_index, self.first_visible);
        self.selected_index = next;
        self.ensure_selected_visible();
        old != (self.selected_index, self.first_visible)
    }

    /// 向上移动一个 item，到顶后保持在第一项。
    pub fn move_up(&mut self) -> bool {
        let next = self.selected_index.saturating_sub(1);
        self.select(next)
    }

    /// 向下移动一个 item，到底后保持在最后一项。
    pub fn move_down(&mut self) -> bool {
        let next = self
            .selected_index
            .saturating_add(1)
            .min(self.item_count.saturating_sub(1));
        self.select(next)
    }

    /// 将游标和窗口移动到顶部。
    pub fn move_to_top(&mut self) -> bool {
        let old = (self.selected_index, self.first_visible);
        self.selected_index = 0;
        self.first_visible = 0;
        old != (self.selected_index, self.first_visible)
    }

    /// 将游标和窗口移动到底部。
    pub fn move_to_bottom(&mut self) -> bool {
        let old = (self.selected_index, self.first_visible);
        self.selected_index = self.item_count.saturating_sub(1);
        self.first_visible = self.max_first_visible();
        old != (self.selected_index, self.first_visible)
    }

    /// 用虚拟 item 节点构建滚动窗口。
    ///
    /// `children` 必须按 `render_range` 的顺序提供。每个节点会被放入固定高度的
    /// 绝对定位槽位，窗口本身负责裁剪视口外的 item。
    pub fn node<Action>(
        self,
        mut style: UiStyle,
        children: impl IntoIterator<Item = UiNode<Action>>,
    ) -> UiNode<Action> {
        style.clip = true;
        style.direction = FlexDirection::Stack;
        let stride = self.item_height.saturating_add(self.gap);
        let child_nodes = self.render_range().zip(children).map(|(index, child)| {
            let top = u32::try_from(index.saturating_sub(self.first_visible))
                .map_or(u32::MAX, |value| value)
                .saturating_mul(stride);
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(self.item_height),
                    position: Position::Absolute { left: 0, top },
                    ..UiStyle::default()
                })
                .with_children([child])
        });
        UiNode::auto().with_style(style).with_children(child_nodes)
    }

    fn max_first_visible(self) -> usize {
        self.item_count.saturating_sub(self.visible_items)
    }

    fn ensure_selected_visible(&mut self) {
        if self.item_count == 0 {
            self.first_visible = 0;
            return;
        }
        if self.selected_index < self.first_visible {
            self.first_visible = self.selected_index;
        } else {
            let last_visible = self
                .first_visible
                .saturating_add(self.visible_items)
                .saturating_sub(1);
            if self.selected_index > last_visible {
                self.first_visible = self
                    .selected_index
                    .saturating_add(1)
                    .saturating_sub(self.visible_items)
                    .min(self.max_first_visible());
            }
        }
    }
}
