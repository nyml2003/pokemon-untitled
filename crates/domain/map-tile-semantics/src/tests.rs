use super::*;
use std::collections::BTreeMap;

use crate::{
    FORMAT_VERSION, SemanticRuleLocation,
    catalog::{
        direction_for, matches_neighbour_rule, matches_tile, scoped_layers, validate_matcher,
    },
};
use map_project::{CompositeTile, CompositeTileId, MapProject, MapProjectId};

fn tile(value: &str) -> AtomicTileId {
    AtomicTileId::new(value).unwrap()
}
fn tag(value: &str) -> TileTag {
    TileTag::new(value).unwrap()
}
fn pattern(value: &str) -> PatternId {
    PatternId::new(value).unwrap()
}
fn rules(stack: Vec<StackRule>) -> TileHardRules {
    TileHardRules {
        stack,
        neighbours: Neighbours8::filled(NeighbourRule::Any),
    }
}
fn catalog() -> TileSemanticsCatalog {
    TileSemanticsCatalog {
        format_version: FORMAT_VERSION.into(),
        tiles: vec![
            TileDefinition {
                id: tile("ground"),
                status: TileStatus::Approved {
                    tags: [tag("ground")].into_iter().collect(),
                    rules: Box::new(rules(vec![StackRule::MustBeBase])),
                },
            },
            TileDefinition {
                id: tile("tree-a"),
                status: TileStatus::Approved {
                    tags: BTreeSet::new(),
                    rules: Box::new(rules(vec![StackRule::RequiresBelow {
                        matcher: TileMatcher::Tagged { tag: tag("ground") },
                    }])),
                },
            },
            TileDefinition {
                id: tile("tree-b"),
                status: TileStatus::Approved {
                    tags: BTreeSet::new(),
                    rules: Box::new(rules(vec![StackRule::RequiresBelow {
                        matcher: TileMatcher::Tagged { tag: tag("ground") },
                    }])),
                },
            },
            TileDefinition {
                id: tile("blocked"),
                status: TileStatus::Blocked {
                    reason: "review pending".into(),
                },
            },
        ],
        patterns: vec![PatternDefinition {
            id: pattern("tree"),
            parts: vec![
                PatternPart {
                    coord: PatternCoord(0, 0),
                    tile: tile("tree-a"),
                },
                PatternPart {
                    coord: PatternCoord(1, 0),
                    tile: tile("tree-b"),
                },
            ],
        }],
    }
}
fn known() -> BTreeSet<AtomicTileId> {
    [
        tile("ground"),
        tile("tree-a"),
        tile("tree-b"),
        tile("blocked"),
    ]
    .into_iter()
    .collect()
}
fn project(layers: Vec<Vec<AtomicTileId>>) -> MapProject {
    let materials = layers
        .into_iter()
        .enumerate()
        .map(|(index, layers)| {
            CompositeTile::new(CompositeTileId::new(format!("m{index}")).unwrap(), layers)
        })
        .collect::<Vec<_>>();
    let mut project = MapProject::blank(MapProjectId::new("map").unwrap(), 2, 1, None);
    project.materials = materials;
    project.visual_cells[0].material = Some(CompositeTileId::new("m0").unwrap());
    project.visual_cells[1].material = Some(CompositeTileId::new("m1").unwrap());
    project
}
#[test]
fn validates_complete_catalog_and_json() {
    let catalog = catalog();
    catalog.validate(&known()).unwrap();
    let json = serde_json::to_string(&catalog).unwrap();
    assert_eq!(
        TileSemanticsCatalog::from_json(&json, &known()).unwrap(),
        catalog
    );
    let mut incomplete = catalog.clone();
    incomplete.tiles.pop();
    assert!(matches!(
        incomplete.validate(&known()),
        Err(TileSemanticsError::CoverageMismatch { .. })
    ));
}
#[test]
fn validates_stack_and_full_pattern_through_layered_cells() {
    let complete = project(vec![
        vec![tile("ground"), tile("tree-a")],
        vec![tile("ground"), tile("tree-b")],
    ]);
    assert!(catalog().lint(&complete).is_empty());
    let missing = project(vec![
        vec![tile("ground"), tile("tree-a")],
        vec![tile("ground")],
    ]);
    assert!(catalog().lint(&missing).iter().any(|diagnostic| matches!(
        diagnostic.rule,
        SemanticRuleLocation::Pattern {
            direction: Direction8::East,
            ..
        }
    )));
    let reversed = project(vec![
        vec![tile("tree-a"), tile("ground")],
        vec![tile("ground"), tile("tree-b")],
    ]);
    assert!(
        catalog()
            .lint(&reversed)
            .iter()
            .any(|diagnostic| diagnostic.rule == SemanticRuleLocation::Stack)
    );
}
#[test]
fn reports_blocked_tiles_and_matcher_edges() {
    let project = project(vec![
        vec![tile("ground"), tile("blocked")],
        vec![tile("ground")],
    ]);
    assert!(
        catalog()
            .lint(&project)
            .iter()
            .any(|diagnostic| diagnostic.rule == SemanticRuleLocation::Catalog)
    );
    let matcher = TileMatcher::AnyOf {
        matchers: vec![
            TileMatcher::AtomicTile {
                tile: tile("ground"),
            },
            TileMatcher::Tagged {
                tag: tag("missing"),
            },
        ],
    };
    assert!(matches_tile(
        &matcher,
        &tile("ground"),
        &catalog()
            .tiles
            .iter()
            .map(|definition| (definition.id.clone(), definition))
            .collect(),
        &catalog()
    ));
    assert!(matches!(
        validate_matcher(&TileMatcher::AnyOf {
            matchers: Vec::new()
        }),
        Err(TileSemanticsError::EmptyAnyOf)
    ));
    assert_eq!(Direction8::North.delta(), (0, -1));
}

#[test]
fn covers_catalog_failures_matchers_and_diagnostics() {
    assert!(PatternId::new(" ").is_err());
    assert!(TileTag::new(" ").is_err());

    let source = catalog();
    let mut unsupported = source.clone();
    unsupported.format_version = "wrong".into();
    assert!(matches!(
        unsupported.validate(&known()),
        Err(TileSemanticsError::UnsupportedFormat(_))
    ));
    let mut unknown_definition = source.clone();
    unknown_definition.tiles[0].id = tile("missing");
    assert!(matches!(
        unknown_definition.validate(&known()),
        Err(TileSemanticsError::UnknownTile(_))
    ));
    let mut duplicate_definition = source.clone();
    duplicate_definition
        .tiles
        .push(duplicate_definition.tiles[0].clone());
    assert!(matches!(
        duplicate_definition.validate(&known()),
        Err(TileSemanticsError::DuplicateTile(_))
    ));
    let mut empty_tag = source.clone();
    if let TileStatus::Approved { tags, .. } = &mut empty_tag.tiles[0].status {
        tags.insert(TileTag(String::new()));
    }
    assert!(matches!(
        empty_tag.validate(&known()),
        Err(TileSemanticsError::EmptyId("TileTag"))
    ));
    let mut duplicate_pattern = source.clone();
    duplicate_pattern
        .patterns
        .push(duplicate_pattern.patterns[0].clone());
    assert!(matches!(
        duplicate_pattern.validate(&known()),
        Err(TileSemanticsError::DuplicatePattern(_))
    ));
    let mut unknown_part = source.clone();
    unknown_part.patterns[0].parts[0].tile = tile("missing");
    assert!(matches!(
        unknown_part.validate(&known()),
        Err(TileSemanticsError::UnknownTile(_))
    ));
    let mut duplicate_part = source.clone();
    let duplicate = duplicate_part.patterns[0].parts[0].clone();
    duplicate_part.patterns[0].parts.push(duplicate);
    assert!(matches!(
        duplicate_part.validate(&known()),
        Err(TileSemanticsError::DuplicatePatternPart { .. })
    ));
    assert!(matches!(
        TileSemanticsCatalog::from_json("{", &known()),
        Err(TileSemanticsError::Json(_))
    ));

    let definitions = source
        .tiles
        .iter()
        .map(|definition| (definition.id.clone(), definition))
        .collect::<BTreeMap<_, _>>();
    let nested = TileMatcher::AnyOf {
        matchers: vec![TileMatcher::AnyOf {
            matchers: vec![TileMatcher::PatternPart {
                pattern: pattern("tree"),
                part: PatternCoord(0, 0),
            }],
        }],
    };
    assert!(validate_matcher(&nested).is_ok());
    assert!(matches_tile(
        &nested,
        &tile("tree-a"),
        &definitions,
        &source
    ));
    for scope in [LayerScope::Any, LayerScope::Base, LayerScope::Top] {
        assert!(!scoped_layers(&[tile("ground"), tile("tree-a")], scope).is_empty());
    }
    let required = NeighbourRule::Requires {
        requirement: CellRequirement {
            scope: LayerScope::Base,
            matcher: TileMatcher::AtomicTile {
                tile: tile("ground"),
            },
        },
    };
    let forbidden = NeighbourRule::Forbids {
        requirement: CellRequirement {
            scope: LayerScope::Top,
            matcher: TileMatcher::AtomicTile {
                tile: tile("tree-a"),
            },
        },
    };
    assert!(matches_neighbour_rule(
        &required,
        &[tile("ground"), tile("tree-a")],
        &definitions,
        &source
    ));
    assert!(!matches_neighbour_rule(
        &forbidden,
        &[tile("ground"), tile("tree-a")],
        &definitions,
        &source
    ));
    let mut strict = source.clone();
    if let TileStatus::Approved { rules, .. } = &mut strict.tiles[1].status {
        rules.neighbours.north = required;
    }
    assert!(strict.validate(&known()).is_ok());
    let missing_neighbour = project(vec![
        vec![tile("ground"), tile("tree-a")],
        vec![tile("ground"), tile("tree-b")],
    ]);
    assert!(strict.lint(&missing_neighbour).iter().any(|diagnostic| {
        diagnostic.rule == SemanticRuleLocation::Neighbour(Direction8::North)
    }));
    let incomplete = TileSemanticsCatalog {
        format_version: FORMAT_VERSION.into(),
        tiles: vec![],
        patterns: vec![],
    };
    assert!(
        incomplete
            .lint(&missing_neighbour)
            .iter()
            .any(|diagnostic| diagnostic.expected == "a semantic definition")
    );
    for (delta, direction) in [
        ((0, -1), Direction8::North),
        ((1, -1), Direction8::NorthEast),
        ((1, 0), Direction8::East),
        ((1, 1), Direction8::SouthEast),
        ((0, 1), Direction8::South),
        ((-1, 1), Direction8::SouthWest),
        ((-1, 0), Direction8::West),
        ((-1, -1), Direction8::NorthWest),
    ] {
        assert_eq!(direction_for(delta.0, delta.1), Some(direction));
    }
    assert_eq!(direction_for(2, 0), None);

    for error in [
        TileSemanticsError::EmptyId("id"),
        TileSemanticsError::UnsupportedFormat("v0".into()),
        TileSemanticsError::UnknownTile(tile("unknown")),
        TileSemanticsError::DuplicateTile(tile("duplicate")),
        TileSemanticsError::CoverageMismatch {
            expected: 2,
            actual: 1,
        },
        TileSemanticsError::DuplicatePattern(pattern("duplicate")),
        TileSemanticsError::DuplicatePatternPart {
            pattern: pattern("duplicate"),
            coord: PatternCoord(0, 0),
        },
        TileSemanticsError::EmptyAnyOf,
        TileSemanticsError::Json("bad".into()),
    ] {
        assert!(!error.to_string().is_empty());
    }
}
