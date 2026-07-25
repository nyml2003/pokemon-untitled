use super::super::common::FOUNDATION_THEME;
use super::common::{page_detail, page_notice, page_slot};
use game_foundation::ItemCategory;
use game_page_model::{BagFilter, BagPageModel, PageIntent};
use game_ui_kit::{
    PanelTone, column as ui_column, panel as ui_panel, row as ui_row, screen as ui_screen,
};
use punctum_ui::{Dimension, Insets, UiBuildError, UiStyle, UiTree};

pub(super) fn project_pause_bag(
    bag: &BagPageModel,
    notice: Option<&str>,
) -> Result<UiTree<PageIntent>, UiBuildError> {
    let selected = bag
        .selected
        .as_ref()
        .and_then(|id| bag.entries.iter().find(|entry| &entry.item == id));
    let categories = [
        ("全", "page-bag-category-all", BagFilter::All),
        (
            "药",
            "page-bag-category-medicine",
            BagFilter::Category(ItemCategory::Medicine),
        ),
        (
            "键",
            "page-bag-category-key",
            BagFilter::Category(ItemCategory::Key),
        ),
        (
            "杂",
            "page-bag-category-general",
            BagFilter::Category(ItemCategory::General),
        ),
    ]
    .into_iter()
    .map(|(label, key, category)| {
        page_slot(
            label,
            key,
            bag.category == category,
            Some(PageIntent::SelectBagCategory(category)),
            Dimension::Fill,
            Dimension::Px(58),
        )
    })
    .collect::<Result<Vec<_>, _>>()?;
    let mut entries = bag
        .entries
        .iter()
        .map(|entry| {
            page_slot(
                format!("{}\nx{}", entry.item.as_str(), entry.quantity),
                format!("page-bag-{}", entry.item.as_str()),
                bag.selected.as_ref() == Some(&entry.item),
                Some(PageIntent::SelectBagItem(entry.item.clone())),
                Dimension::Fill,
                Dimension::Px(96),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    while entries.len() < 10 {
        let index = entries.len();
        entries.push(page_slot(
            "",
            format!("page-bag-empty-{index}"),
            false,
            None,
            Dimension::Fill,
            Dimension::Px(96),
        )?);
    }
    let rows = entries
        .chunks(5)
        .map(|row| {
            ui_row(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(96),
                    gap: 10,
                    ..UiStyle::default()
                },
                row.iter().cloned(),
            )
        })
        .collect::<Vec<_>>();
    let selected_detail = selected.map_or_else(
        || {
            page_detail(
                "背包",
                format!(
                    "容量 {}/{}    金钱 {}",
                    bag.slots_used,
                    bag.capacity,
                    bag.money.amount()
                ),
            )
        },
        |entry| {
            page_detail(
                entry.item.as_str(),
                format!(
                    "{:?}    数量 {}/{}",
                    entry.category, entry.quantity, entry.stack_limit
                ),
            )
        },
    );
    UiTree::new(ui_screen(
        &FOUNDATION_THEME,
        [ui_panel(
            &FOUNDATION_THEME,
            PanelTone::Screen,
            UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                gap: 12,
                padding: Insets::all(24),
                ..UiStyle::default()
            },
            [
                ui_row(
                    UiStyle {
                        width: Dimension::Fill,
                        height: Dimension::Px(58),
                        gap: 10,
                        ..UiStyle::default()
                    },
                    categories,
                ),
                ui_column(
                    UiStyle {
                        width: Dimension::Fill,
                        height: Dimension::Px(202),
                        gap: 10,
                        ..UiStyle::default()
                    },
                    rows,
                ),
                selected_detail,
                page_notice(notice),
            ],
        )],
    ))
}
