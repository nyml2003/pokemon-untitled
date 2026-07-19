use std::collections::BTreeSet;

use crate::{
    Dimension, Position, UiBuildError, UiContent, UiFrame, UiId, UiKey, UiLayoutError, UiSize,
    UiStyle, UiTextSize, layout::resolve_tree,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiNode<Action = ()> {
    /// 节点的结构标识。
    /// 新页面代码应使用 `UiNode::auto()`，由 `UiTree::new` 自动分配此值。
    pub id: UiId,
    pub key: Option<UiKey>,
    pub action: Option<Action>,
    pub style: UiStyle,
    pub content: UiContent,
    pub children: Vec<UiNode<Action>>,
    automatic_id: bool,
}
impl UiNode<()> {
    #[deprecated(
        note = "请使用 UiNode::auto()；UiTree::new 会确定性地分配结构 ID。"
    )]
    pub fn new(id: UiId) -> Self {
        Self {
            id,
            key: None,
            action: None,
            style: UiStyle::default(),
            content: UiContent::Empty,
            children: Vec::new(),
            automatic_id: false,
        }
    }
}
impl<Action> UiNode<Action> {
    /// 创建由 `UiTree::new` 分配结构 ID 的节点。
    pub fn auto() -> Self {
        Self {
            id: UiId::default(),
            key: None,
            action: None,
            style: UiStyle::default(),
            content: UiContent::Empty,
            children: Vec::new(),
            automatic_id: true,
        }
    }
    #[deprecated(
        note = "请使用 UiNode::auto()；UiTree::new 会确定性地分配结构 ID。"
    )]
    pub fn legacy(id: UiId) -> Self {
        Self {
            id,
            key: None,
            action: None,
            style: UiStyle::default(),
            content: UiContent::Empty,
            children: Vec::new(),
            automatic_id: false,
        }
    }
    pub fn with_key(mut self, key: UiKey) -> Self {
        self.key = Some(key);
        self
    }
    /// 关联触发动作，并将节点标记为可交互。
    #[allow(deprecated)]
    pub fn with_action(mut self, action: Action) -> Self {
        self.action = Some(action);
        self.style.interactive = true;
        self
    }
    pub fn with_style(mut self, style: UiStyle) -> Self {
        self.style = style;
        self
    }
    pub fn with_content(mut self, content: UiContent) -> Self {
        self.content = content;
        self
    }
    pub fn with_children(mut self, children: impl IntoIterator<Item = UiNode<Action>>) -> Self {
        self.children = children.into_iter().collect();
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiTree<Action = ()> {
    root: UiNode<Action>,
}
impl<Action> UiTree<Action> {
    /// 构建并校验 UI 树。
    /// 自动节点会按树结构获得 ID；重复 ID 或 key、无效比例和无效逻辑画布会返回 `UiBuildError`。
    pub fn new(mut root: UiNode<Action>) -> Result<Self, UiBuildError> {
        let mut reserved_ids = BTreeSet::new();
        collect_explicit_ids(&root, &mut reserved_ids)?;
        assign_automatic_ids(&mut root, &reserved_ids)?;
        let mut ids = BTreeSet::new();
        let mut keys = BTreeSet::new();
        validate_node(&root, &mut ids, &mut keys)?;
        Ok(Self { root })
    }
    pub fn root(&self) -> &UiNode<Action> {
        &self.root
    }
}
impl<Action: Clone> UiTree<Action> {
    /// 在给定视口中解析绘制命令和命中区域。
    /// 当节点无法放入其容器时返回 `UiLayoutError`。
    pub fn resolve(&self, viewport: UiSize) -> Result<UiFrame<Action>, UiLayoutError> {
        resolve_tree(&self.root, viewport)
    }
}
fn collect_explicit_ids<Action>(
    node: &UiNode<Action>,
    ids: &mut BTreeSet<UiId>,
) -> Result<(), UiBuildError> {
    if !node.automatic_id && !ids.insert(node.id) {
        return Err(UiBuildError::DuplicateId(node.id));
    }
    for child in &node.children {
        collect_explicit_ids(child, ids)?;
    }
    Ok(())
}
fn assign_automatic_ids<Action>(
    node: &mut UiNode<Action>,
    reserved: &BTreeSet<UiId>,
) -> Result<(), UiBuildError> {
    fn next_available(
        next: &mut u32,
        reserved: &BTreeSet<UiId>,
        assigned: &BTreeSet<UiId>,
    ) -> Result<UiId, UiBuildError> {
        loop {
            let id = UiId(*next);
            if !reserved.contains(&id) && !assigned.contains(&id) {
                return Ok(id);
            }
            *next = next.checked_add(1).ok_or(UiBuildError::IdExhausted)?;
        }
    }
    fn visit<Action>(
        node: &mut UiNode<Action>,
        next: &mut u32,
        reserved: &BTreeSet<UiId>,
        assigned: &mut BTreeSet<UiId>,
    ) -> Result<(), UiBuildError> {
        if node.automatic_id {
            node.id = next_available(next, reserved, assigned)?;
            node.automatic_id = false;
            assigned.insert(node.id);
        }
        for child in &mut node.children {
            visit(child, next, reserved, assigned)?;
        }
        Ok(())
    }
    visit(node, &mut 0, reserved, &mut BTreeSet::new())
}
fn validate_node<Action>(
    node: &UiNode<Action>,
    ids: &mut BTreeSet<UiId>,
    keys: &mut BTreeSet<UiKey>,
) -> Result<(), UiBuildError> {
    if !ids.insert(node.id) {
        return Err(UiBuildError::DuplicateId(node.id));
    }
    if let Some(key) = &node.key
        && !keys.insert(key.clone())
    {
        return Err(UiBuildError::DuplicateKey(key.clone()));
    }
    validate_style(node.id, node.style)?;
    validate_content(node.id, &node.content)?;
    for child in &node.children {
        validate_node(child, ids, keys)?;
    }
    Ok(())
}

fn validate_content(id: UiId, content: &UiContent) -> Result<(), UiBuildError> {
    if let UiContent::TextScaled {
        font_size: UiTextSize::Ratio { base: 0, .. },
        ..
    } = content
    {
        return Err(UiBuildError::ZeroTextSizeBase(id));
    }
    Ok(())
}

fn validate_style(id: UiId, style: UiStyle) -> Result<(), UiBuildError> {
    for dimension in [style.width, style.height] {
        if let Dimension::Ratio { base: 0, .. } = dimension {
            return Err(UiBuildError::ZeroRatioBase(id));
        }
    }
    if let Position::AbsoluteRatio { base, .. } = style.position
        && (base.width == 0 || base.height == 0)
    {
        return Err(UiBuildError::ZeroRatioBase(id));
    }
    if let Some(canvas) = style.logical_canvas
        && (canvas.width == 0 || canvas.height == 0)
    {
        return Err(UiBuildError::ZeroLogicalCanvas(id));
    }
    Ok(())
}
