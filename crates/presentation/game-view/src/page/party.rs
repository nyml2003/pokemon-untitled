use super::super::common::FOUNDATION_THEME;
use super::assets::page_party_pokemon_asset;
use super::common::{page_detail, page_notice, page_slot_with_image};
use game_page_model::{PageIntent, PartyPageModel};
use game_ui_kit::{PanelTone, panel as ui_panel, row as ui_row, screen as ui_screen};
use punctum_ui::{Dimension, Insets, UiBuildError, UiStyle, UiTree};

pub(super) fn project_pause_party(
    party: &PartyPageModel,
    notice: Option<&str>,
) -> Result<UiTree<PageIntent>, UiBuildError> {
    let selected = party
        .selected
        .as_ref()
        .and_then(|id| party.members.iter().find(|member| &member.id == id));
    let members = party
        .members
        .iter()
        .map(|member| {
            page_slot_with_image(
                format!(
                    "{}\nHP {}/{}",
                    member.species, member.current_hp, member.max_hp
                ),
                format!("page-party-{}", member.id.as_str()),
                party.selected.as_ref() == Some(&member.id),
                Some(PageIntent::SelectPartyMember(member.id.clone())),
                Dimension::Fill,
                Dimension::Px(132),
                page_party_pokemon_asset(&member.species),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let detail = selected.map_or_else(
        || page_detail("队伍", "选择一个槽位"),
        |member| {
            page_detail(
                member.species.as_str(),
                format!(
                    "HP {}/{}    PP {}/{}    EXP {}",
                    member.current_hp,
                    member.max_hp,
                    member.current_pp,
                    member.max_pp,
                    member.experience
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
                gap: 16,
                padding: Insets::all(24),
                ..UiStyle::default()
            },
            [
                ui_row(
                    UiStyle {
                        width: Dimension::Fill,
                        height: Dimension::Px(132),
                        gap: 10,
                        ..UiStyle::default()
                    },
                    members,
                ),
                detail,
                page_notice(notice),
            ],
        )],
    ))
}
