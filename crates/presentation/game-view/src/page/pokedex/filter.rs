use std::collections::BTreeMap;

use super::*;
use game_data::{AbilityId, TypeId};
use game_page_model::PokedexFilterIntent;
use game_ui::{MoveFilterItem, PokedexFilterItem, PokedexFilterOverlay, TypeMatch};
use game_ui_kit::{
    FormOption, checkbox_group, filter_summary, form_section, form_shell, icon_button, label,
    number_input, radio_group, range_input, select, text_input, toggle,
};
use punctum_ui::{Dimension, Insets, UiBuildError, UiKey, UiNode, UiStyle};

pub(super) fn compact_entry(
    visual: &PokedexVisualState,
) -> Result<UiNode<PageIntent>, UiBuildError> {
    let move_filter = visual.detail_mode == PokedexDetailMode::Moves;
    Ok(filter_summary(
        &POKEDEX_THEME,
        icon_button(
            &POKEDEX_THEME,
            UiKey::new("page-pokedex-filter-toggle")?,
            "⌕",
            false,
            PageIntent::TogglePokedexFilter,
        ),
        compact_summary(visual, move_filter),
    ))
}

pub(super) fn expanded(
    pokedex: &PokedexPageModel,
    visual: &PokedexVisualState,
) -> Result<Option<UiNode<PageIntent>>, UiBuildError> {
    match &visual.filter_overlay {
        PokedexFilterOverlay::Compact => Ok(None),
        PokedexFilterOverlay::Pokedex(state) => Ok(Some(pokedex_form(pokedex, visual, state)?)),
        PokedexFilterOverlay::Moves(state) => Ok(Some(move_form(pokedex, visual, state)?)),
    }
}

fn pokedex_form(
    pokedex: &PokedexPageModel,
    visual: &PokedexVisualState,
    state: &punctum_ui::KeyboardFormState<PokedexFilterItem>,
) -> Result<UiNode<PageIntent>, UiBuildError> {
    let types = pokemon_type_options(pokedex);
    let abilities = ability_options(pokedex);
    let focused = state.focused_item();
    let (height_minimum, height_maximum) = visual.pokedex_filter.height_draft();
    let (weight_minimum, weight_maximum) = visual.pokedex_filter.weight_draft();
    let selected_ability = visual
        .pokedex_filter
        .ability
        .and_then(|id| abilities.get(&id))
        .cloned()
        .unwrap_or_else(|| String::from("所有特性"));
    let ability = if state
        .opened_select()
        .is_some_and(|item| *item == PokedexFilterItem::Ability)
        && !visual.pokedex_filter.ability_query().is_empty()
    {
        String::from(visual.pokedex_filter.ability_query())
    } else {
        selected_ability
    };
    let ability_options = if state
        .opened_select()
        .is_some_and(|item| *item == PokedexFilterItem::Ability)
    {
        let mut values = vec![FormOption {
            key: UiKey::new("page-pokedex-filter-ability-all")?,
            label: String::from("所有特性"),
            selected: visual.pokedex_filter.ability.is_none(),
            focused: visual.pokedex_ability_cursor == 0,
            action: PageIntent::PokedexFilter(PokedexFilterIntent::SelectAbility(None)),
        }];
        for (index, (id, name)) in abilities
            .iter()
            .filter(|(_, name)| name.contains(visual.pokedex_filter.ability_query()))
            .enumerate()
        {
            values.push(FormOption {
                key: UiKey::new(format!("page-pokedex-filter-ability-{}", id.0))?,
                label: name.clone(),
                selected: visual.pokedex_filter.ability == Some(*id),
                focused: visual.pokedex_ability_cursor == index.saturating_add(1),
                action: PageIntent::PokedexFilter(PokedexFilterIntent::SelectAbility(Some(*id))),
            });
        }
        Some(values)
    } else {
        None
    };
    Ok(form_shell(
        &POKEDEX_THEME,
        visual.form_scroll_y,
        [
            form_header(
                PageIntent::PokedexFilter(PokedexFilterIntent::ResetPokedex),
                "page-pokedex-filter-reset",
            )?,
            form_section(
                &POKEDEX_THEME,
                [
                    label(&POKEDEX_THEME, "属性", None::<String>),
                    checkbox_group(
                        &POKEDEX_THEME,
                        types
                            .iter()
                            .map(|(id, name)| {
                                Ok(FormOption {
                                    key: UiKey::new(format!("page-pokedex-filter-type-{}", id.0))?,
                                    label: name.clone(),
                                    selected: visual.pokedex_filter.type_ids.contains(id),
                                    focused: focused == Some(&PokedexFilterItem::Types),
                                    action: PageIntent::PokedexFilter(
                                        PokedexFilterIntent::ToggleType(*id),
                                    ),
                                })
                            })
                            .collect::<Result<Vec<_>, UiBuildError>>()?,
                    ),
                    radio_group(
                        &POKEDEX_THEME,
                        [
                            FormOption {
                                key: UiKey::new("page-pokedex-filter-type-any")?,
                                label: String::from("任一"),
                                selected: visual.pokedex_filter.type_match == TypeMatch::Any,
                                focused: focused == Some(&PokedexFilterItem::TypeMatch),
                                action: PageIntent::PokedexFilter(
                                    PokedexFilterIntent::SetTypeMatchAll(false),
                                ),
                            },
                            FormOption {
                                key: UiKey::new("page-pokedex-filter-type-all")?,
                                label: String::from("全部"),
                                selected: visual.pokedex_filter.type_match == TypeMatch::All,
                                focused: focused == Some(&PokedexFilterItem::TypeMatch),
                                action: PageIntent::PokedexFilter(
                                    PokedexFilterIntent::SetTypeMatchAll(true),
                                ),
                            },
                        ],
                    ),
                ],
            ),
            form_section(
                &POKEDEX_THEME,
                [
                    label(&POKEDEX_THEME, "世代", None::<String>),
                    checkbox_group(
                        &POKEDEX_THEME,
                        [(1_u8, "关都"), (2, "城都"), (3, "丰缘")]
                            .into_iter()
                            .map(|(generation, name)| {
                                Ok(FormOption {
                                    key: UiKey::new(format!(
                                        "page-pokedex-filter-generation-{generation}"
                                    ))?,
                                    label: String::from(name),
                                    selected: visual
                                        .pokedex_filter
                                        .generations
                                        .contains(&generation),
                                    focused: focused == Some(&PokedexFilterItem::Generations),
                                    action: PageIntent::PokedexFilter(
                                        PokedexFilterIntent::ToggleGeneration(generation),
                                    ),
                                })
                            })
                            .collect::<Result<Vec<_>, UiBuildError>>()?,
                    ),
                ],
            ),
            form_section(
                &POKEDEX_THEME,
                [
                    label(&POKEDEX_THEME, "身高 (m)", None::<String>),
                    range_input(
                        &POKEDEX_THEME,
                        number_input(
                            &POKEDEX_THEME,
                            UiKey::new("page-pokedex-filter-height-min")?,
                            height_minimum,
                            focused == Some(&PokedexFilterItem::HeightMinimum),
                            PageIntent::PokedexFilter(PokedexFilterIntent::SetHeightMinimum(
                                String::from(height_minimum),
                            )),
                        ),
                        number_input(
                            &POKEDEX_THEME,
                            UiKey::new("page-pokedex-filter-height-max")?,
                            height_maximum,
                            focused == Some(&PokedexFilterItem::HeightMaximum),
                            PageIntent::PokedexFilter(PokedexFilterIntent::SetHeightMaximum(
                                String::from(height_maximum),
                            )),
                        ),
                    ),
                ],
            ),
            form_section(
                &POKEDEX_THEME,
                [
                    label(&POKEDEX_THEME, "体重 (kg)", None::<String>),
                    range_input(
                        &POKEDEX_THEME,
                        number_input(
                            &POKEDEX_THEME,
                            UiKey::new("page-pokedex-filter-weight-min")?,
                            weight_minimum,
                            focused == Some(&PokedexFilterItem::WeightMinimum),
                            PageIntent::PokedexFilter(PokedexFilterIntent::SetWeightMinimum(
                                String::from(weight_minimum),
                            )),
                        ),
                        number_input(
                            &POKEDEX_THEME,
                            UiKey::new("page-pokedex-filter-weight-max")?,
                            weight_maximum,
                            focused == Some(&PokedexFilterItem::WeightMaximum),
                            PageIntent::PokedexFilter(PokedexFilterIntent::SetWeightMaximum(
                                String::from(weight_maximum),
                            )),
                        ),
                    ),
                ],
            ),
            form_section(
                &POKEDEX_THEME,
                [
                    label(&POKEDEX_THEME, "特性", None::<String>),
                    select(
                        &POKEDEX_THEME,
                        UiKey::new("page-pokedex-filter-ability")?,
                        ability,
                        focused == Some(&PokedexFilterItem::Ability),
                        PageIntent::TogglePokedexAbilitySelect,
                        ability_options,
                    ),
                ],
            ),
        ],
    ))
}

fn move_form(
    pokedex: &PokedexPageModel,
    visual: &PokedexVisualState,
    state: &punctum_ui::KeyboardFormState<MoveFilterItem>,
) -> Result<UiNode<PageIntent>, UiBuildError> {
    let types = move_type_options(pokedex);
    let focused = state.focused_item();
    let (power_minimum, power_maximum) = visual.move_filter.power_draft();
    Ok(form_shell(
        &POKEDEX_THEME,
        visual.form_scroll_y,
        [
            form_header(
                PageIntent::PokedexFilter(PokedexFilterIntent::ResetMove),
                "page-pokedex-move-filter-reset",
            )?,
            form_section(
                &POKEDEX_THEME,
                [
                    label(&POKEDEX_THEME, "技能名称", None::<String>),
                    text_input(
                        &POKEDEX_THEME,
                        UiKey::new("page-pokedex-move-filter-name")?,
                        visual.move_filter.name_query.as_str(),
                        focused == Some(&MoveFilterItem::Name),
                        PageIntent::PokedexFilter(PokedexFilterIntent::SetMoveName(
                            visual.move_filter.name_query.clone(),
                        )),
                    ),
                ],
            ),
            form_section(
                &POKEDEX_THEME,
                [
                    label(&POKEDEX_THEME, "属性", None::<String>),
                    checkbox_group(
                        &POKEDEX_THEME,
                        types
                            .iter()
                            .map(|(id, name)| {
                                Ok(FormOption {
                                    key: UiKey::new(format!(
                                        "page-pokedex-move-filter-type-{}",
                                        id.0
                                    ))?,
                                    label: name.clone(),
                                    selected: visual.move_filter.type_ids.contains(id),
                                    focused: focused == Some(&MoveFilterItem::Types),
                                    action: PageIntent::PokedexFilter(
                                        PokedexFilterIntent::ToggleMoveType(*id),
                                    ),
                                })
                            })
                            .collect::<Result<Vec<_>, UiBuildError>>()?,
                    ),
                ],
            ),
            form_section(
                &POKEDEX_THEME,
                [
                    label(&POKEDEX_THEME, "分类", None::<String>),
                    radio_group(
                        &POKEDEX_THEME,
                        move_category_options(
                            focused == Some(&MoveFilterItem::Category),
                            visual.move_filter.category,
                        )?,
                    ),
                ],
            ),
            form_section(
                &POKEDEX_THEME,
                [
                    label(&POKEDEX_THEME, "威力", None::<String>),
                    range_input(
                        &POKEDEX_THEME,
                        number_input(
                            &POKEDEX_THEME,
                            UiKey::new("page-pokedex-move-filter-power-min")?,
                            power_minimum,
                            focused == Some(&MoveFilterItem::PowerMinimum),
                            PageIntent::PokedexFilter(PokedexFilterIntent::SetMovePowerMinimum(
                                String::from(power_minimum),
                            )),
                        ),
                        number_input(
                            &POKEDEX_THEME,
                            UiKey::new("page-pokedex-move-filter-power-max")?,
                            power_maximum,
                            focused == Some(&MoveFilterItem::PowerMaximum),
                            PageIntent::PokedexFilter(PokedexFilterIntent::SetMovePowerMaximum(
                                String::from(power_maximum),
                            )),
                        ),
                    ),
                ],
            ),
            form_section(
                &POKEDEX_THEME,
                [
                    label(&POKEDEX_THEME, "命中", None::<String>),
                    select(
                        &POKEDEX_THEME,
                        UiKey::new("page-pokedex-move-filter-accuracy")?,
                        accuracy_label(visual.move_filter.accuracy),
                        focused == Some(&MoveFilterItem::Accuracy),
                        PageIntent::TogglePokedexMoveAccuracySelect,
                        state
                            .opened_select()
                            .is_some_and(|item| *item == MoveFilterItem::Accuracy)
                            .then(|| {
                                accuracy_options(
                                    pokedex,
                                    visual.move_filter.accuracy,
                                    visual.move_accuracy_cursor,
                                )
                            })
                            .transpose()?,
                    ),
                ],
            ),
            toggle(
                &POKEDEX_THEME,
                UiKey::new("page-pokedex-move-filter-priority")?,
                "仅看有先制度",
                visual.move_filter.priority_only,
                focused == Some(&MoveFilterItem::Priority),
                PageIntent::PokedexFilter(PokedexFilterIntent::ToggleMovePriority),
            ),
        ],
    ))
}

fn form_header(reset: PageIntent, reset_key: &str) -> Result<UiNode<PageIntent>, UiBuildError> {
    Ok(game_ui_kit::row(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(34),
            padding: Insets::symmetric(0, 1),
            ..UiStyle::default()
        },
        [game_ui_kit::button(
            &POKEDEX_THEME,
            UiStyle::fixed(54, 30),
            false,
            [text_node("重置", TextTone::Default, 14)],
        )
        .with_key(UiKey::new(reset_key)?)
        .with_action(reset)],
    ))
}

fn compact_summary(visual: &PokedexVisualState, move_filter: bool) -> String {
    if move_filter {
        if visual.move_filter.name_query.is_empty() && visual.move_filter.type_ids.is_empty() {
            String::new()
        } else {
            String::from("筛选")
        }
    } else if visual.pokedex_filter.type_ids.is_empty()
        && visual.pokedex_filter.generations.is_empty()
        && visual.pokedex_filter.ability.is_none()
    {
        String::new()
    } else {
        String::from("筛选")
    }
}

fn pokemon_type_options(pokedex: &PokedexPageModel) -> BTreeMap<TypeId, String> {
    pokedex
        .entries
        .iter()
        .filter(|entry| entry.known)
        .flat_map(|entry| {
            entry
                .type_ids
                .iter()
                .copied()
                .zip(entry.types.iter().cloned())
        })
        .collect()
}

fn move_type_options(pokedex: &PokedexPageModel) -> BTreeMap<TypeId, String> {
    pokedex
        .moves
        .iter()
        .map(|item| (item.type_id, item.move_type.clone()))
        .collect()
}

fn ability_options(pokedex: &PokedexPageModel) -> BTreeMap<AbilityId, String> {
    pokedex
        .entries
        .iter()
        .filter(|entry| entry.known)
        .flat_map(|entry| entry.abilities.iter())
        .map(|ability| (ability.id, ability.name.clone()))
        .collect()
}

fn move_category_options(
    focused: bool,
    selected: Option<PokedexMoveCategory>,
) -> Result<Vec<FormOption<PageIntent>>, UiBuildError> {
    [
        ("全部", None, "all"),
        ("物理", Some(PokedexMoveCategory::Physical), "physical"),
        ("特殊", Some(PokedexMoveCategory::Special), "special"),
        ("变化", Some(PokedexMoveCategory::Status), "status"),
    ]
    .into_iter()
    .map(|(label, category, key)| {
        Ok(FormOption {
            key: UiKey::new(format!("page-pokedex-move-filter-category-{key}"))?,
            label: String::from(label),
            selected: selected == category,
            focused,
            action: PageIntent::PokedexFilter(PokedexFilterIntent::SelectMoveCategory(category)),
        })
    })
    .collect()
}

fn accuracy_options(
    pokedex: &PokedexPageModel,
    selected: Option<Option<u8>>,
    cursor: usize,
) -> Result<Vec<FormOption<PageIntent>>, UiBuildError> {
    let mut values = vec![None];
    values.extend(
        pokedex
            .moves
            .iter()
            .map(|move_model| move_model.accuracy)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .map(Some),
    );
    values
        .into_iter()
        .enumerate()
        .map(|(index, accuracy)| {
            let (label, key) = match accuracy {
                None => (String::from("所有命中"), String::from("all")),
                Some(None) => (String::from("必定命中"), String::from("guaranteed")),
                Some(Some(value)) => (format!("{value}%"), value.to_string()),
            };
            Ok(FormOption {
                key: UiKey::new(format!("page-pokedex-move-filter-accuracy-{key}"))?,
                label,
                selected: selected == accuracy,
                focused: cursor == index,
                action: PageIntent::PokedexFilter(PokedexFilterIntent::SelectMoveAccuracy(
                    accuracy,
                )),
            })
        })
        .collect()
}

fn accuracy_label(accuracy: Option<Option<u8>>) -> String {
    match accuracy {
        None => String::from("所有命中"),
        Some(None) => String::from("必定命中"),
        Some(Some(value)) => format!("{value}%"),
    }
}
