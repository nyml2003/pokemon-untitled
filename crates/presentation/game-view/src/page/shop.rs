use super::super::common::FOUNDATION_THEME;
use super::common::{page_detail, page_notice, page_slot};
use game_page_model::{PageIntent, ShopPageModel};
use game_ui_kit::{
    PanelTone, TextTone, panel as ui_panel, row as ui_row, screen as ui_screen, text as ui_text,
};
use punctum_ui::{CrossAlign, Dimension, Insets, MainAlign, UiBuildError, UiStyle, UiTree};

pub(super) fn project_page_shop(
    shop: &ShopPageModel,
    notice: Option<&str>,
) -> Result<UiTree<PageIntent>, UiBuildError> {
    let detail = shop.selected_item.as_ref();
    let item_label = detail.map_or("尚未选择物品", |item| item.item.as_str());
    let quantity = detail.map_or(1, |item| item.quantity);
    let previous = quantity
        .checked_sub(1)
        .filter(|next| *next > 0)
        .map(PageIntent::SetShopQuantity);
    let next = quantity.checked_add(1).map(PageIntent::SetShopQuantity);
    let purchase = detail
        .filter(|item| item.affordable)
        .map(|_| PageIntent::ConfirmShopPurchase);
    let price = detail.map_or(String::from("--"), |item| {
        item.total_price.amount().to_string()
    });
    let owned = detail.map_or(String::from("--"), |item| item.owned_quantity.to_string());
    let purchase_hint = match detail {
        Some(item) if item.affordable => "确认后更新余额与背包",
        Some(_) => "余额不足，无法购买",
        None => "选择物品后确认购买",
    };
    let less = page_slot(
        "-",
        "page-shop-less",
        false,
        previous,
        Dimension::Fill,
        Dimension::Px(64),
    )?;
    let more = page_slot(
        "+",
        "page-shop-more",
        false,
        next,
        Dimension::Fill,
        Dimension::Px(64),
    )?;
    let confirm = page_slot(
        if detail.is_some_and(|item| item.affordable) {
            "买"
        } else {
            "x"
        },
        "page-shop-confirm",
        false,
        purchase,
        Dimension::Px(120),
        Dimension::Px(64),
    )?;
    UiTree::new(ui_screen(
        &FOUNDATION_THEME,
        [ui_panel(
            &FOUNDATION_THEME,
            PanelTone::Screen,
            UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                gap: 16,
                padding: Insets::all(24),
                main_align: MainAlign::Center,
                cross_align: CrossAlign::Center,
                ..UiStyle::default()
            },
            [
                page_slot(
                    item_label,
                    "page-shop-item",
                    true,
                    None,
                    Dimension::Px(220),
                    Dimension::Px(120),
                )?,
                page_detail(
                    format!("{}  x{}", item_label, quantity),
                    format!(
                        "余额 {}    价格 {}    持有 {}    容量 {}/{}",
                        shop.money.amount(),
                        price,
                        owned,
                        shop.inventory_slots_used,
                        shop.inventory_capacity
                    ),
                ),
                ui_row(
                    UiStyle {
                        width: Dimension::Px(260),
                        height: Dimension::Px(64),
                        gap: 10,
                        ..UiStyle::default()
                    },
                    [less, page_detail("数量", quantity.to_string()), more],
                ),
                ui_text(
                    &FOUNDATION_THEME,
                    TextTone::Muted,
                    purchase_hint,
                    14,
                    Dimension::Px(260),
                ),
                confirm,
                page_notice(notice),
            ],
        )],
    ))
}
