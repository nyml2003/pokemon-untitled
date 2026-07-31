use std::time::Duration;

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FormId {
    Query,
    Type,
    Ability,
    Reset,
}

#[test]
fn keyboard_form_keeps_schema_and_interaction_state_separate()
-> Result<(), Box<dyn std::error::Error>> {
    let form = KeyboardForm::try_new([
        FormItem::new(FormId::Query, FormItemKind::Field),
        FormItem::new(FormId::Type, FormItemKind::Group),
        FormItem::new(FormId::Ability, FormItemKind::Select),
        FormItem::new(FormId::Reset, FormItemKind::Command),
    ])?;
    let mut state = KeyboardFormState::default();

    assert_eq!(state.presentation(), FormPresentation::Compact);
    assert!(state.toggle_presentation(&form));
    assert_eq!(state.focused_item(), Some(&FormId::Query));
    assert!(state.focus_next(&form));
    assert_eq!(state.focused_item(), Some(&FormId::Type));
    assert!(state.focus_next(&form));
    assert_eq!(state.focused_item(), Some(&FormId::Ability));
    assert!(state.open_select(&form, &FormId::Ability));
    assert_eq!(state.opened_select(), Some(&FormId::Ability));
    assert!(state.focus_next(&form));
    assert_eq!(state.focused_item(), Some(&FormId::Reset));
    assert_eq!(state.opened_select(), None);
    assert!(state.compact());
    assert_eq!(state.presentation(), FormPresentation::Compact);
    Ok(())
}

#[test]
fn keyboard_form_skips_hidden_or_disabled_fields_and_rejects_duplicate_ids() {
    let form = KeyboardForm::try_new([
        FormItem::new(FormId::Query, FormItemKind::Field).with_visible(false),
        FormItem::new(FormId::Type, FormItemKind::Group).with_enabled(false),
        FormItem::new(FormId::Ability, FormItemKind::Select),
    ]);
    assert!(form.is_ok());
    let Ok(form) = form else {
        return;
    };
    let mut state = KeyboardFormState::default();
    assert!(state.expand(&form));
    assert_eq!(state.focused_item(), Some(&FormId::Ability));
    assert!(!state.open_select(&form, &FormId::Type));

    let duplicate = KeyboardForm::try_new([
        FormItem::new(FormId::Query, FormItemKind::Field),
        FormItem::new(FormId::Query, FormItemKind::Command),
    ]);
    assert_eq!(
        duplicate,
        Err(KeyboardFormError::DuplicateItemId(FormId::Query))
    );
}
fn fill(_id: u32, style: UiStyle) -> UiNode {
    UiNode::auto()
        .with_style(style)
        .with_content(UiContent::Fill(UiColor::new(1, 2, 3, 255)))
}
#[test]
fn row_allocates_fill_and_aligns_children() {
    let tree = UiTree::new(
        UiNode::auto()
            .with_style(UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                direction: FlexDirection::Row,
                gap: 10,
                ..UiStyle::default()
            })
            .with_children([
                fill(2, UiStyle::fixed(20, 20)),
                fill(
                    3,
                    UiStyle {
                        width: Dimension::Fill,
                        height: Dimension::Px(20),
                        ..UiStyle::default()
                    },
                ),
            ]),
    )
    .unwrap();
    let frame = tree.resolve(UiSize::new(100, 40)).unwrap();
    match &frame.commands()[1] {
        UiDrawCommand::Fill { bounds, .. } => assert_eq!(*bounds, UiRect::new(30, 0, 70, 20)),
        _ => panic!(),
    }
}

#[test]
fn automatic_container_size_includes_flow_children() -> Result<(), Box<dyn std::error::Error>> {
    let tree = UiTree::new(
        UiNode::auto()
            .with_style(UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                direction: FlexDirection::Column,
                ..UiStyle::default()
            })
            .with_children([UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Auto,
                    height: Dimension::Auto,
                    direction: FlexDirection::Column,
                    gap: 4,
                    ..UiStyle::default()
                })
                .with_children([
                    fill(1, UiStyle::fixed(20, 10)),
                    fill(2, UiStyle::fixed(30, 8)),
                ])]),
    )?;
    let frame = tree.resolve(UiSize::new(80, 60))?;
    let fills = frame
        .commands()
        .iter()
        .filter_map(|command| match command {
            UiDrawCommand::Fill { bounds, .. } => Some(*bounds),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        fills,
        [UiRect::new(0, 0, 20, 10), UiRect::new(0, 14, 30, 8)]
    );
    Ok(())
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum TestAction {
    Back,
    Front,
}

fn action_fill(style: UiStyle) -> UiNode<TestAction> {
    UiNode::auto()
        .with_style(style)
        .with_content(UiContent::Fill(UiColor::new(1, 2, 3, 255)))
}

#[test]
fn clipping_and_topmost_hit_are_deterministic() {
    let interactive = UiStyle {
        width: Dimension::Px(30),
        height: Dimension::Px(30),
        position: Position::Absolute { left: 10, top: 10 },
        ..UiStyle::default()
    };
    let tree = UiTree::new(
        UiNode::auto()
            .with_style(UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                clip: true,
                direction: FlexDirection::Stack,
                ..UiStyle::default()
            })
            .with_children([
                action_fill(interactive).with_action(TestAction::Back),
                action_fill(UiStyle { ..interactive }).with_action(TestAction::Front),
            ]),
    )
    .unwrap();
    let frame = tree.resolve(UiSize::new(20, 20)).unwrap();
    assert_eq!(frame.action_hit_at(15, 15).map(|hit| hit.id), Some(UiId(2)));
    assert_eq!(frame.action_hit_at(25, 15), None);
}

#[test]
fn visual_offset_moves_paint_and_hits_without_reflowing_siblings()
-> Result<(), Box<dyn std::error::Error>> {
    let shifted = action_fill(UiStyle {
        width: Dimension::Px(10),
        height: Dimension::Px(10),
        visual_offset: UiPixelOffset::new(5, 0),
        ..UiStyle::default()
    })
    .with_action(TestAction::Back);
    let tree = UiTree::new(
        UiNode::auto()
            .with_style(UiStyle {
                width: Dimension::Px(30),
                height: Dimension::Px(12),
                direction: FlexDirection::Row,
                gap: 2,
                clip: true,
                ..UiStyle::default()
            })
            .with_children([
                shifted,
                action_fill(UiStyle::fixed(10, 10)).with_action(TestAction::Front),
            ]),
    )?;
    let frame = tree.resolve(UiSize::new(30, 12))?;
    assert!(matches!(
        frame.commands()[0],
        UiDrawCommand::Fill {
            bounds: UiRect { x: 5, .. },
            ..
        }
    ));
    assert!(matches!(
        frame.commands()[1],
        UiDrawCommand::Fill {
            bounds: UiRect { x: 12, .. },
            ..
        }
    ));
    assert_eq!(
        frame.action_hit_at(6, 5).map(|hit| hit.bounds),
        Some(UiRect::new(5, 0, 10, 10))
    );
    Ok(())
}

#[test]
fn negative_visual_offset_is_clipped_without_coordinate_underflow()
-> Result<(), Box<dyn std::error::Error>> {
    let tree = UiTree::new(
        UiNode::auto()
            .with_style(UiStyle {
                width: Dimension::Px(20),
                height: Dimension::Px(10),
                clip: true,
                ..UiStyle::default()
            })
            .with_children([action_fill(UiStyle {
                width: Dimension::Px(10),
                height: Dimension::Px(10),
                visual_offset: UiPixelOffset::new(-4, 0),
                ..UiStyle::default()
            })
            .with_action(TestAction::Front)]),
    )?;
    let frame = tree.resolve(UiSize::new(20, 10))?;
    assert_eq!(
        frame.action_hit_at(0, 5).map(|hit| hit.bounds),
        Some(UiRect::new(0, 0, 6, 10))
    );
    Ok(())
}

#[test]
fn button_states_are_orthogonal_and_ripple_is_transient() -> Result<(), Box<dyn std::error::Error>>
{
    let button = UiButtonStyle {
        selected: true,
        hover_color: UiColor::new(1, 2, 3, 40),
        pressed_color: UiColor::new(4, 5, 6, 80),
        focus_color: UiColor::new(7, 8, 9, 255),
        ripple_color: UiColor::new(10, 11, 12, 160),
        focus_width: 1,
        ripple_duration_ms: 160,
        ..UiButtonStyle::new(true, false)
    };
    let tree = UiTree::new(
        UiNode::auto()
            .with_style(UiStyle::fixed(40, 20))
            .with_content(UiContent::Fill(UiColor::new(20, 20, 20, 255)))
            .with_button(button)
            .with_action(TestAction::Front),
    )?;
    let frame = tree.resolve(UiSize::new(40, 20))?;
    let mut interaction = UiInteraction::default();
    assert!(interaction.pointer_move(frame.interaction_targets(), 10, 10));
    assert!(interaction.press(frame.interaction_targets(), 10, 10));
    assert_eq!(interaction.snapshot().hovered, Some(UiId(0)));
    assert_eq!(interaction.snapshot().pressed, Some(UiId(0)));
    assert_eq!(interaction.snapshot().focused, Some(UiId(0)));
    assert_eq!(interaction.snapshot().ripples.len(), 1);

    let active = frame.with_interaction(interaction.snapshot());
    assert!(
        active
            .commands()
            .iter()
            .any(|command| matches!(command, UiDrawCommand::Ripple { .. }))
    );
    assert_eq!(
        interaction.release(frame.interaction_targets(), 40, 19),
        None
    );
    assert_eq!(interaction.snapshot().pressed, None);
    assert!(interaction.press(frame.interaction_targets(), 10, 10));
    assert_eq!(
        interaction.release(frame.interaction_targets(), 10, 10),
        Some(UiId(0))
    );
    assert!(interaction.advance(Duration::from_millis(200)));
    assert!(interaction.snapshot().ripples.is_empty());
    Ok(())
}

#[test]
fn disabled_button_is_visual_but_not_actionable() -> Result<(), Box<dyn std::error::Error>> {
    let tree = UiTree::new(
        UiNode::auto()
            .with_style(UiStyle::fixed(40, 20))
            .with_content(UiContent::Fill(UiColor::new(20, 20, 20, 255)))
            .with_button(UiButtonStyle::new(false, true))
            .with_action(TestAction::Front),
    )?;
    let frame = tree.resolve(UiSize::new(40, 20))?;
    assert_eq!(frame.action_hits().len(), 0);
    let mut interaction = UiInteraction::default();
    assert!(!interaction.press(frame.interaction_targets(), 10, 10));
    Ok(())
}

#[test]
fn automatic_ids_and_typed_actions_are_deterministic() {
    let item_style = UiStyle {
        width: Dimension::Px(30),
        height: Dimension::Px(30),
        position: Position::Absolute { left: 10, top: 10 },
        ..UiStyle::default()
    };
    let tree = UiTree::<TestAction>::new(
        UiNode::auto()
            .with_style(UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                direction: FlexDirection::Stack,
                ..UiStyle::default()
            })
            .with_children([
                UiNode::auto()
                    .with_key(UiKey::new("back").unwrap())
                    .with_style(item_style)
                    .with_action(TestAction::Back),
                UiNode::auto()
                    .with_key(UiKey::new("front").unwrap())
                    .with_style(item_style)
                    .with_action(TestAction::Front),
                UiNode::auto().with_style(UiStyle::fixed(1, 1)),
            ]),
    )
    .unwrap();

    assert_eq!(tree.root().id, UiId(0));
    assert_eq!(tree.root().children[0].id, UiId(1));
    assert_eq!(tree.root().children[1].id, UiId(2));
    assert_eq!(tree.root().children[2].id, UiId(3));

    let frame = tree.resolve(UiSize::new(40, 40)).unwrap();
    assert_eq!(frame.hit_action(15, 15), Some(&TestAction::Front));
    assert_eq!(frame.action_hits().len(), 2);
    assert_eq!(
        frame.action_hits()[1].key,
        Some(UiKey::new("front").unwrap())
    );
}

#[test]
fn a_large_automatic_tree_needs_no_caller_supplied_ids() {
    let tree = UiTree::new(
        UiNode::<()>::auto()
            .with_children((0..1_000).map(|_| UiNode::auto().with_style(UiStyle::fixed(1, 1)))),
    )
    .unwrap();

    assert_eq!(tree.root().id, UiId(0));
    assert_eq!(tree.root().children.len(), 1_000);
    assert_eq!(tree.root().children[999].id, UiId(1_000));
}

#[test]
fn keyboard_scroll_view_moves_one_item_and_limits_virtual_nodes()
-> Result<(), Box<dyn std::error::Error>> {
    let mut view = KeyboardSingleColumnFixedHeightScrollView::new(100, 3, 10)
        .with_gap(2)
        .with_overscan(1);
    assert_eq!(view.visible_range(), 0..3);
    assert_eq!(view.render_range(), 0..4);
    assert!(view.move_down());
    assert_eq!(view.selected_index(), 1);
    assert_eq!(view.first_visible(), 0);
    assert!(view.move_down());
    assert!(view.move_down());
    assert_eq!(view.selected_index(), 3);
    assert_eq!(view.first_visible(), 1);
    assert_eq!(view.render_range(), 1..5);

    assert!(view.move_to_bottom());
    assert_eq!(view.selected_index(), 99);
    assert_eq!(view.visible_range(), 97..100);
    assert!(!view.move_down());
    assert!(view.move_to_top());
    assert_eq!(view.selected_index(), 0);
    assert_eq!(view.visible_range(), 0..3);
    assert!(!view.move_up());

    let children = view
        .render_range()
        .map(|_| UiNode::<()>::auto().with_style(UiStyle::fixed(20, 10)));
    let tree = UiTree::new(view.node(
        UiStyle {
            width: Dimension::Px(20),
            height: Dimension::Px(30),
            ..UiStyle::default()
        },
        children,
    ))?;
    assert_eq!(tree.root().children.len(), 4);
    assert!(tree.resolve(UiSize::new(20, 30)).is_ok());
    Ok(())
}

#[test]
fn duplicate_ui_keys_are_build_errors() {
    let duplicate = UiTree::<TestAction>::new(UiNode::auto().with_children([
        UiNode::auto().with_key(UiKey::new("entry").unwrap()),
        UiNode::auto().with_key(UiKey::new("entry").unwrap()),
    ]));
    assert!(matches!(
        duplicate,
        Err(UiBuildError::DuplicateKey(key)) if key == UiKey::new("entry").unwrap()
    ));
    assert_eq!(UiKey::new(""), Err(UiBuildError::EmptyKey));
}

#[test]
fn logical_canvas_coordinates_resolve_without_grid_types() {
    let tree = UiTree::new(
        UiNode::auto()
            .with_style(UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                direction: FlexDirection::Stack,
                ..UiStyle::default()
            })
            .with_children([fill(
                2,
                UiStyle {
                    width: Dimension::Ratio {
                        units: 10,
                        base: 32,
                    },
                    height: Dimension::Ratio { units: 4, base: 24 },
                    position: Position::AbsoluteRatio {
                        left: 10,
                        top: 3,
                        base: UiSize::new(32, 24),
                    },
                    ..UiStyle::default()
                },
            )]),
    )
    .unwrap();

    let frame = tree.resolve(UiSize::new(640, 480)).unwrap();
    match &frame.commands()[0] {
        UiDrawCommand::Fill { bounds, .. } => {
            assert_eq!(*bounds, UiRect::new(200, 60, 200, 80));
        }
        _ => panic!(),
    }
}
#[test]
fn duplicate_ids_and_conflicting_minimum_rows_are_errors() {
    assert!(UiTree::<()>::new(UiNode::auto().with_children([UiNode::auto()])).is_ok());
    let tree = UiTree::new(
        UiNode::auto()
            .with_style(UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                direction: FlexDirection::Row,
                ..UiStyle::default()
            })
            .with_children([
                fill(
                    2,
                    UiStyle {
                        min_size: UiSize::new(8, 1),
                        ..UiStyle::fixed(8, 1)
                    },
                ),
                fill(
                    3,
                    UiStyle {
                        min_size: UiSize::new(8, 1),
                        ..UiStyle::fixed(8, 1)
                    },
                ),
            ]),
    )
    .unwrap();
    assert!(matches!(
        tree.resolve(UiSize::new(10, 2)),
        Err(UiLayoutError::InsufficientSpace { .. })
    ));
}

#[test]
fn scalar_values_and_build_failures_are_explicit() {
    assert_eq!(UiPixelOffset::new(-2, 3), UiPixelOffset { x: -2, y: 3 });
    assert_eq!(UiContentId::new(""), Err(UiBuildError::EmptyContentId));
    assert!(UiBorderRadius::default().is_zero());
    assert_eq!(
        UiTextSize::Ratio {
            units: 3,
            base: 4,
            minimum: 5,
            maximum: 7,
        }
        .resolve(20),
        7
    );

    let root = UiTree::new(UiNode::<()>::auto()).unwrap();
    assert_eq!(root.root().id, UiId(0));
    let invalid = [
        UiNode::<()>::auto().with_content(UiContent::TextScaled {
            content: "x".into(),
            color: UiColor::default(),
            font_size: UiTextSize::Ratio {
                units: 1,
                base: 0,
                minimum: 1,
                maximum: 2,
            },
        }),
        UiNode::auto().with_style(UiStyle {
            width: Dimension::Ratio { units: 1, base: 0 },
            ..UiStyle::default()
        }),
        UiNode::auto().with_style(UiStyle {
            position: Position::AbsoluteRatio {
                left: 0,
                top: 0,
                base: UiSize::new(0, 1),
            },
            ..UiStyle::default()
        }),
        UiNode::auto().with_style(UiStyle {
            logical_canvas: Some(UiSize::new(1, 0)),
            ..UiStyle::default()
        }),
    ];
    for node in invalid {
        assert!(UiTree::new(node).is_err());
    }
    for error in [
        UiBuildError::EmptyContentId,
        UiBuildError::DuplicateId(UiId(1)),
        UiBuildError::ZeroRatioBase(UiId(1)),
        UiBuildError::ZeroLogicalCanvas(UiId(1)),
        UiBuildError::ZeroTextSizeBase(UiId(1)),
    ] {
        assert!(!error.to_string().is_empty());
    }
    assert!(
        !UiLayoutError::InsufficientSpace { id: UiId(1) }
            .to_string()
            .is_empty()
    );
}

#[test]
fn uncommon_content_and_layout_modes_resolve_deterministically() {
    let styled = UiTree::<()>::new(
        UiNode::auto()
            .with_style(UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                logical_canvas: Some(UiSize::new(10, 5)),
                max_size: Some(UiSize::new(30, 20)),
                ..UiStyle::default()
            })
            .with_content(UiContent::ImageStyled {
                content: UiContentId::new("panel").unwrap(),
                tint: UiColor::new(1, 2, 3, 4),
                pixel_offset: UiPixelOffset::new(2, -1),
            }),
    )
    .unwrap();
    let styled_frame = styled.resolve(UiSize::new(40, 20)).unwrap();
    assert!(matches!(
        styled_frame.commands()[0],
        UiDrawCommand::Image {
            bounds: UiRect {
                x: 0,
                y: 0,
                width: 40,
                height: 20
            },
            pixel_offset: UiPixelOffset { x: 2, y: -1 },
            ..
        }
    ));

    let text = |_id| {
        UiNode::<()>::auto().with_content(UiContent::Text {
            content: "wide".into(),
            color: UiColor::default(),
            font_size: 4,
        })
    };
    let horizontal = UiTree::new(
        UiNode::auto()
            .with_style(UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                direction: FlexDirection::Row,
                main_align: MainAlign::End,
                cross_align: CrossAlign::Stretch,
                ..UiStyle::default()
            })
            .with_children([text(11)]),
    )
    .unwrap();
    assert!(horizontal.resolve(UiSize::new(30, 12)).is_ok());

    let vertical = UiTree::new(
        UiNode::auto()
            .with_style(UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                cross_align: CrossAlign::End,
                ..UiStyle::default()
            })
            .with_children([text(21)]),
    )
    .unwrap();
    assert!(vertical.resolve(UiSize::new(30, 12)).is_ok());
}

#[test]
fn remaining_content_and_flex_branches_stay_pure() {
    assert_eq!(UiContentId::new("asset").unwrap().as_str(), "asset");
    assert_eq!(UiBorderRadius::all(3).bottom_left, 3);
    assert_eq!(Insets::all(2), Insets::symmetric(2, 2));
    assert_eq!(UiTextSize::Px(6).resolve(99), 6);

    let image = UiContentId::new("asset").unwrap();
    let bordered_image = UiTree::new(
        UiNode::auto()
            .with_style(UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                border: UiBorder {
                    widths: Insets::all(1),
                    color: UiColor::new(9, 8, 7, 6),
                },
                ..UiStyle::default()
            })
            .with_content(UiContent::Image(image.clone()))
            .with_action(()),
    )
    .unwrap();
    let frame = bordered_image.resolve(UiSize::new(10, 8)).unwrap();
    assert_eq!(frame.viewport(), UiSize::new(10, 8));
    assert_eq!(frame.commands().len(), 2);
    assert_eq!(
        frame.action_hit_at(1, 1).map(|hit| (hit.id, hit.bounds)),
        Some((UiId(0), UiRect::new(0, 0, 10, 8)))
    );

    let tinted = UiTree::<()>::new(
        UiNode::auto()
            .with_style(UiStyle::fixed(10, 8))
            .with_content(UiContent::ImageTinted {
                content: image.clone(),
                tint: UiColor::new(1, 2, 3, 4),
            }),
    )
    .unwrap();
    assert!(matches!(
        tinted.resolve(UiSize::new(10, 8)).unwrap().commands()[0],
        UiDrawCommand::Image { .. }
    ));

    let scaled = UiTree::<()>::new(
        UiNode::auto()
            .with_style(UiStyle::fixed(20, 10))
            .with_content(UiContent::TextScaled {
                content: "x".into(),
                color: UiColor::default(),
                font_size: UiTextSize::Ratio {
                    units: 1,
                    base: 2,
                    minimum: 1,
                    maximum: 8,
                },
            }),
    )
    .unwrap();
    assert!(matches!(
        scaled.resolve(UiSize::new(20, 10)).unwrap().commands()[0],
        UiDrawCommand::Text { font_size: 5, .. }
    ));
    assert!(
        UiTree::<()>::new(UiNode::auto().with_style(UiStyle::fixed(0, 1)))
            .unwrap()
            .resolve(UiSize::new(10, 10))
            .unwrap()
            .commands()
            .is_empty()
    );

    let ratio_row = UiTree::new(
        UiNode::auto()
            .with_style(UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                direction: FlexDirection::Row,
                main_align: MainAlign::Center,
                cross_align: CrossAlign::Center,
                ..UiStyle::default()
            })
            .with_children([fill(
                11,
                UiStyle {
                    width: Dimension::Ratio { units: 1, base: 2 },
                    height: Dimension::Ratio { units: 1, base: 2 },
                    ..UiStyle::default()
                },
            )]),
    )
    .unwrap();
    assert!(ratio_row.resolve(UiSize::new(20, 10)).is_ok());

    let ratio_column = UiTree::new(
        UiNode::auto()
            .with_style(UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                ..UiStyle::default()
            })
            .with_children([fill(
                16,
                UiStyle {
                    width: Dimension::Ratio { units: 1, base: 2 },
                    height: Dimension::Ratio { units: 1, base: 2 },
                    ..UiStyle::default()
                },
            )]),
    )
    .unwrap();
    assert!(ratio_column.resolve(UiSize::new(20, 10)).is_ok());

    let spaced_stack = UiTree::new(
        UiNode::auto()
            .with_style(UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                direction: FlexDirection::Stack,
                main_align: MainAlign::SpaceBetween,
                ..UiStyle::default()
            })
            .with_children([
                fill(21, UiStyle::fixed(2, 2)),
                fill(22, UiStyle::fixed(2, 2)),
            ]),
    )
    .unwrap();
    assert!(spaced_stack.resolve(UiSize::new(10, 10)).is_ok());
}
