use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FormPresentation {
    #[default]
    Compact,
    Expanded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormItemKind {
    Field,
    Group,
    Select,
    Command,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormItem<ItemId> {
    id: ItemId,
    kind: FormItemKind,
    visible: bool,
    enabled: bool,
}

impl<ItemId> FormItem<ItemId> {
    pub const fn new(id: ItemId, kind: FormItemKind) -> Self {
        Self {
            id,
            kind,
            visible: true,
            enabled: true,
        }
    }

    pub const fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub const fn id(&self) -> &ItemId {
        &self.id
    }

    pub const fn kind(&self) -> FormItemKind {
        self.kind
    }

    pub const fn visible(&self) -> bool {
        self.visible
    }

    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    const fn focusable(&self) -> bool {
        self.visible && self.enabled
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyboardFormError<ItemId> {
    DuplicateItemId(ItemId),
}

impl<ItemId> Display for KeyboardFormError<ItemId> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateItemId(_) => formatter.write_str("form contains a duplicate item ID"),
        }
    }
}

impl<ItemId: std::fmt::Debug> std::error::Error for KeyboardFormError<ItemId> {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyboardForm<ItemId> {
    items: Vec<FormItem<ItemId>>,
}

impl<ItemId: Clone + Eq> KeyboardForm<ItemId> {
    pub fn try_new(
        items: impl IntoIterator<Item = FormItem<ItemId>>,
    ) -> Result<Self, KeyboardFormError<ItemId>> {
        let items = items.into_iter().collect::<Vec<_>>();
        for (index, item) in items.iter().enumerate() {
            if items[..index].iter().any(|previous| previous.id == item.id) {
                return Err(KeyboardFormError::DuplicateItemId(item.id.clone()));
            }
        }
        Ok(Self { items })
    }

    pub fn items(&self) -> &[FormItem<ItemId>] {
        &self.items
    }

    pub fn item(&self, id: &ItemId) -> Option<&FormItem<ItemId>> {
        self.items.iter().find(|item| item.id == *id)
    }

    fn edge_focusable(&self, backwards: bool) -> Option<&ItemId> {
        if backwards {
            self.items
                .iter()
                .rev()
                .find(|item| item.focusable())
                .map(|item| &item.id)
        } else {
            self.items
                .iter()
                .find(|item| item.focusable())
                .map(|item| &item.id)
        }
    }

    fn next_focusable(&self, current: Option<&ItemId>, backwards: bool) -> Option<&ItemId> {
        let items = if backwards {
            self.items.iter().rev().collect::<Vec<_>>()
        } else {
            self.items.iter().collect::<Vec<_>>()
        };
        let current_index = current.and_then(|id| items.iter().position(|item| item.id == *id));
        let start = current_index.map_or(0, |index| index.saturating_add(1));
        items
            .into_iter()
            .skip(start)
            .find(|item| item.focusable())
            .map(|item| &item.id)
            .or_else(|| current.and_then(|_| self.edge_focusable(backwards)))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyboardFormState<ItemId> {
    presentation: FormPresentation,
    focused_item: Option<ItemId>,
    opened_select: Option<ItemId>,
}

impl<ItemId> Default for KeyboardFormState<ItemId> {
    fn default() -> Self {
        Self {
            presentation: FormPresentation::Compact,
            focused_item: None,
            opened_select: None,
        }
    }
}

impl<ItemId: Clone + Eq> KeyboardFormState<ItemId> {
    pub const fn presentation(&self) -> FormPresentation {
        self.presentation
    }

    pub fn focused_item(&self) -> Option<&ItemId> {
        self.focused_item.as_ref()
    }

    pub fn opened_select(&self) -> Option<&ItemId> {
        self.opened_select.as_ref()
    }

    pub fn toggle_presentation(&mut self, form: &KeyboardForm<ItemId>) -> bool {
        match self.presentation {
            FormPresentation::Compact => self.expand(form),
            FormPresentation::Expanded => self.compact(),
        }
    }

    pub fn expand(&mut self, form: &KeyboardForm<ItemId>) -> bool {
        let changed = self.presentation != FormPresentation::Expanded;
        self.presentation = FormPresentation::Expanded;
        if !self.focus_is_valid(form) {
            self.focused_item = form.edge_focusable(false).cloned();
        }
        changed
    }

    pub fn compact(&mut self) -> bool {
        let changed =
            self.presentation != FormPresentation::Compact || self.opened_select.is_some();
        self.presentation = FormPresentation::Compact;
        self.opened_select = None;
        changed
    }

    pub fn focus_next(&mut self, form: &KeyboardForm<ItemId>) -> bool {
        self.move_focus(form, false)
    }

    pub fn focus_previous(&mut self, form: &KeyboardForm<ItemId>) -> bool {
        self.move_focus(form, true)
    }

    pub fn open_select(&mut self, form: &KeyboardForm<ItemId>, id: &ItemId) -> bool {
        let Some(item) = form.item(id) else {
            return false;
        };
        if !item.focusable() || item.kind != FormItemKind::Select {
            return false;
        }
        let changed = self.opened_select.as_ref() != Some(id);
        self.focused_item = Some(id.clone());
        self.opened_select = Some(id.clone());
        changed
    }

    pub fn close_select(&mut self) -> bool {
        self.opened_select.take().is_some()
    }

    fn move_focus(&mut self, form: &KeyboardForm<ItemId>, backwards: bool) -> bool {
        if self.presentation != FormPresentation::Expanded {
            return false;
        }
        let next = form
            .next_focusable(self.focused_item.as_ref(), backwards)
            .cloned();
        let changed = self.focused_item != next;
        self.focused_item = next;
        if changed {
            self.opened_select = None;
        }
        changed
    }

    fn focus_is_valid(&self, form: &KeyboardForm<ItemId>) -> bool {
        self.focused_item
            .as_ref()
            .and_then(|id| form.item(id))
            .is_some_and(FormItem::focusable)
    }
}
