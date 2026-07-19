use super::*;
use map_tile_semantics::{FORMAT_VERSION, PatternCoord, PatternDefinition, PatternId, PatternPart};

fn tile(value: &str) -> AtomicTileId {
    AtomicTileId::new(value).unwrap()
}

fn catalog() -> TileSemanticsCatalog {
    TileSemanticsCatalog {
        format_version: FORMAT_VERSION.into(),
        tiles: [tile("ground"), tile("leaf")]
            .into_iter()
            .map(|id| TileDefinition {
                id,
                status: TileStatus::Approved {
                    tags: BTreeSet::new(),
                    rules: Box::new(default_rules()),
                },
            })
            .collect(),
        patterns: vec![PatternDefinition {
            id: PatternId::new("pair").unwrap(),
            parts: vec![PatternPart {
                coord: PatternCoord(0, 0),
                tile: tile("leaf"),
            }],
        }],
    }
}

fn editor() -> TileSemanticsEditor {
    TileSemanticsEditor::new(catalog(), [tile("ground"), tile("leaf")]).unwrap()
}

#[test]
fn selection_navigation_and_catalog_failures_are_explicit() {
    assert!(matches!(
        TileSemanticsEditor::new(catalog(), []),
        Err(TileEditorError::EmptyPalette)
    ));
    let mut editor = editor();
    editor.apply(TileEditorAction::Previous).unwrap();
    assert_eq!(editor.snapshot().unwrap().id, tile("leaf"));
    editor.apply(TileEditorAction::Next).unwrap();
    assert_eq!(editor.snapshot().unwrap().id, tile("ground"));
    assert!(matches!(
        editor.apply(TileEditorAction::Select(tile("missing"))),
        Err(TileEditorError::UnknownTile(_))
    ));
    let incomplete = TileSemanticsCatalog {
        format_version: FORMAT_VERSION.into(),
        tiles: vec![],
        patterns: vec![],
    };
    assert!(matches!(
        TileSemanticsEditor::new(incomplete, [tile("ground")]),
        Err(TileEditorError::Catalog(_))
    ));
}

#[test]
fn edits_approved_properties_and_serializes_a_valid_catalog() {
    let mut editor = editor();
    editor.apply(TileEditorAction::ToggleMeadowTag).unwrap();
    editor
        .apply(TileEditorAction::ToggleStack(StackControl::MustBeBase))
        .unwrap();
    editor
        .apply(TileEditorAction::ToggleStack(
            StackControl::RequiresMeadowBelow,
        ))
        .unwrap();
    editor
        .apply(TileEditorAction::CycleNeighbour(Direction8::North))
        .unwrap();
    let snapshot = editor.snapshot().unwrap();
    assert!(snapshot.meadow && snapshot.must_be_base && snapshot.requires_meadow_below);
    assert_eq!(
        snapshot.neighbours.north,
        NeighbourPreview {
            kind: NeighbourRuleKind::Requires,
            accepted_tiles: vec![tile("ground")],
            locked_by_pattern: false,
        }
    );
    editor
        .apply(TileEditorAction::CycleNeighbour(Direction8::North))
        .unwrap();
    assert_eq!(
        editor.snapshot().unwrap().neighbours.north,
        NeighbourPreview {
            kind: NeighbourRuleKind::Forbids,
            accepted_tiles: vec![tile("leaf")],
            locked_by_pattern: false,
        }
    );
    editor
        .apply(TileEditorAction::CycleNeighbour(Direction8::North))
        .unwrap();
    assert_eq!(
        editor.snapshot().unwrap().neighbours.north,
        NeighbourPreview {
            kind: NeighbourRuleKind::Any,
            accepted_tiles: vec![tile("ground"), tile("leaf")],
            locked_by_pattern: false,
        }
    );
    assert!(editor.catalog_json().unwrap().contains("ground"));
    editor.mark_saved();
    assert!(!editor.snapshot().unwrap().dirty);
}

#[test]
fn blocked_tiles_require_approval_before_property_edits() {
    let mut editor = editor();
    editor.apply(TileEditorAction::ToggleApproved).unwrap();
    assert!(!editor.snapshot().unwrap().approved);
    assert!(matches!(
        editor.apply(TileEditorAction::ToggleMeadowTag),
        Err(TileEditorError::TileBlocked(_))
    ));
    editor.apply(TileEditorAction::ToggleApproved).unwrap();
    assert!(editor.snapshot().unwrap().approved);
}

#[test]
fn resolves_atomic_tagged_pattern_and_any_of_neighbours() {
    let mut editor = editor();
    editor.apply(TileEditorAction::ToggleMeadowTag).unwrap();
    let rule = NeighbourRule::Requires {
        requirement: CellRequirement {
            scope: LayerScope::Any,
            matcher: TileMatcher::AnyOf {
                matchers: vec![
                    TileMatcher::AtomicTile { tile: tile("leaf") },
                    TileMatcher::PatternPart {
                        pattern: PatternId::new("pair").unwrap(),
                        part: PatternCoord(0, 0),
                    },
                    editor.meadow_matcher(),
                ],
            },
        },
    };
    let rules = editor.approved_parts_mut().unwrap().1;
    rules.neighbours.east = rule;
    assert_eq!(
        editor.snapshot().unwrap().neighbours.east,
        NeighbourPreview {
            kind: NeighbourRuleKind::Requires,
            accepted_tiles: vec![tile("ground"), tile("leaf")],
            locked_by_pattern: false,
        }
    );
    assert!(
        TileEditorError::Serialization("x".into())
            .to_string()
            .contains("serialize")
    );
}

#[test]
fn exposes_palette_navigation_all_directions_and_clear_error_messages() {
    let mut editor = editor();
    assert_eq!(editor.ids(), [tile("ground"), tile("leaf")]);
    assert_eq!(editor.selected_index(), 0);

    for direction in [
        Direction8::North,
        Direction8::NorthEast,
        Direction8::East,
        Direction8::SouthEast,
        Direction8::South,
        Direction8::SouthWest,
        Direction8::West,
        Direction8::NorthWest,
    ] {
        editor
            .apply(TileEditorAction::CycleNeighbour(direction))
            .unwrap();
    }
    let snapshot = editor.snapshot().unwrap();
    assert!(matches!(
        snapshot.neighbours.north_east.kind,
        NeighbourRuleKind::Requires
    ));
    assert!(matches!(
        snapshot.neighbours.north_west.kind,
        NeighbourRuleKind::Requires
    ));

    for control in [StackControl::MustBeBase, StackControl::RequiresMeadowBelow] {
        editor
            .apply(TileEditorAction::ToggleStack(control))
            .unwrap();
        editor
            .apply(TileEditorAction::ToggleStack(control))
            .unwrap();
    }
    assert!(!editor.snapshot().unwrap().must_be_base);
    assert!(!editor.snapshot().unwrap().requires_meadow_below);

    let invalid = TileSemanticsCatalog {
        format_version: FORMAT_VERSION.into(),
        tiles: vec![],
        patterns: vec![],
    };
    let catalog_error = TileSemanticsEditor::new(invalid, [tile("ground")])
        .err()
        .expect("incomplete catalog must be rejected");
    assert!(TileEditorError::EmptyPalette.to_string().contains("empty"));
    assert!(
        TileEditorError::UnknownTile(tile("missing"))
            .to_string()
            .contains("missing")
    );
    assert!(
        TileEditorError::TileBlocked(tile("ground"))
            .to_string()
            .contains("blocked")
    );
    assert!(catalog_error.to_string().contains("invalid tile semantics"));
    assert!(
        TileEditorError::PatternNeighbourLocked {
            id: tile("ground"),
            direction: Direction8::North,
        }
        .to_string()
        .contains("pattern-defined")
    );
}

#[test]
fn pattern_parts_render_their_required_neighbours_in_the_editor() {
    let mut editor = editor();
    editor.catalog.patterns = vec![PatternDefinition {
        id: PatternId::new("tree").unwrap(),
        parts: vec![
            PatternPart {
                coord: PatternCoord(0, 0),
                tile: tile("ground"),
            },
            PatternPart {
                coord: PatternCoord(1, 0),
                tile: tile("leaf"),
            },
        ],
    }];
    assert_eq!(
        editor.snapshot().unwrap().neighbours.east,
        NeighbourPreview {
            kind: NeighbourRuleKind::Requires,
            accepted_tiles: vec![tile("leaf")],
            locked_by_pattern: true,
        }
    );
    editor.apply(TileEditorAction::Next).unwrap();
    assert_eq!(
        editor.snapshot().unwrap().neighbours.west,
        NeighbourPreview {
            kind: NeighbourRuleKind::Requires,
            accepted_tiles: vec![tile("ground")],
            locked_by_pattern: true,
        }
    );
    assert!(matches!(
        editor.apply(TileEditorAction::CycleNeighbour(Direction8::West)),
        Err(TileEditorError::PatternNeighbourLocked { .. })
    ));
}

#[test]
fn resolves_every_adjacent_pattern_delta() {
    for (delta, direction) in [
        ((0, -1), Some(Direction8::North)),
        ((1, -1), Some(Direction8::NorthEast)),
        ((1, 0), Some(Direction8::East)),
        ((1, 1), Some(Direction8::SouthEast)),
        ((0, 1), Some(Direction8::South)),
        ((-1, 1), Some(Direction8::SouthWest)),
        ((-1, 0), Some(Direction8::West)),
        ((-1, -1), Some(Direction8::NorthWest)),
        ((0, 0), None),
    ] {
        assert_eq!(direction_from_delta(delta.0, delta.1), direction);
    }
}
