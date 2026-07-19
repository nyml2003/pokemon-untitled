//! Pure state transitions for the tile semantics editor.

#![forbid(unsafe_code)]

use std::{collections::BTreeSet, error::Error, fmt};

use map_project::AtomicTileId;
use map_tile_semantics::{
    CellRequirement, Direction8, LayerScope, NeighbourRule, Neighbours8, StackRule, TileDefinition,
    TileHardRules, TileMatcher, TileSemanticsCatalog, TileSemanticsError, TileStatus, TileTag,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StackControl {
    MustBeBase,
    RequiresMeadowBelow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TileEditorAction {
    Select(AtomicTileId),
    Previous,
    Next,
    ToggleApproved,
    ToggleMeadowTag,
    ToggleStack(StackControl),
    CycleNeighbour(Direction8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NeighbourRuleKind {
    Any,
    Requires,
    Forbids,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NeighbourPreview {
    pub kind: NeighbourRuleKind,
    pub accepted_tiles: Vec<AtomicTileId>,
    pub locked_by_pattern: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileEditorSnapshot {
    pub id: AtomicTileId,
    pub approved: bool,
    pub meadow: bool,
    pub must_be_base: bool,
    pub requires_meadow_below: bool,
    pub neighbours: Neighbours8<NeighbourPreview>,
    pub dirty: bool,
}

#[derive(Clone)]
pub struct TileSemanticsEditor {
    catalog: TileSemanticsCatalog,
    ids: Vec<AtomicTileId>,
    known: BTreeSet<AtomicTileId>,
    selected: usize,
    dirty: bool,
}

impl TileSemanticsEditor {
    pub fn new(
        catalog: TileSemanticsCatalog,
        ids: impl IntoIterator<Item = AtomicTileId>,
    ) -> Result<Self, TileEditorError> {
        let ids = ids.into_iter().collect::<Vec<_>>();
        let known = ids.iter().cloned().collect::<BTreeSet<_>>();
        if ids.is_empty() {
            return Err(TileEditorError::EmptyPalette);
        }
        catalog.validate(&known).map_err(TileEditorError::Catalog)?;
        Ok(Self {
            catalog,
            ids,
            known,
            selected: 0,
            dirty: false,
        })
    }

    pub fn ids(&self) -> &[AtomicTileId] {
        &self.ids
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn snapshot(&self) -> TileEditorSnapshot {
        let definition = self.selected_definition();
        let (approved, tags, rules) = match &definition.status {
            TileStatus::Approved { tags, rules } => (true, Some(tags), Some(rules.as_ref())),
            TileStatus::Blocked { .. } => (false, None, None),
        };
        let meadow = tags.is_some_and(|tags| tags.contains(&meadow_tag()));
        let must_be_base = rules.is_some_and(|rules| {
            rules
                .stack
                .iter()
                .any(|rule| matches!(rule, StackRule::MustBeBase))
        });
        let requires_meadow_below = rules.is_some_and(|rules| {
            rules.stack.iter().any(|rule| {
                matches!(
                    rule,
                    StackRule::RequiresBelow { matcher } if is_meadow_matcher(matcher)
                )
            })
        });
        let neighbours = Neighbours8 {
            north: self.preview(
                &definition.id,
                Direction8::North,
                rules.map(|rules| &rules.neighbours.north),
            ),
            north_east: self.preview(
                &definition.id,
                Direction8::NorthEast,
                rules.map(|rules| &rules.neighbours.north_east),
            ),
            east: self.preview(
                &definition.id,
                Direction8::East,
                rules.map(|rules| &rules.neighbours.east),
            ),
            south_east: self.preview(
                &definition.id,
                Direction8::SouthEast,
                rules.map(|rules| &rules.neighbours.south_east),
            ),
            south: self.preview(
                &definition.id,
                Direction8::South,
                rules.map(|rules| &rules.neighbours.south),
            ),
            south_west: self.preview(
                &definition.id,
                Direction8::SouthWest,
                rules.map(|rules| &rules.neighbours.south_west),
            ),
            west: self.preview(
                &definition.id,
                Direction8::West,
                rules.map(|rules| &rules.neighbours.west),
            ),
            north_west: self.preview(
                &definition.id,
                Direction8::NorthWest,
                rules.map(|rules| &rules.neighbours.north_west),
            ),
        };
        TileEditorSnapshot {
            id: definition.id.clone(),
            approved,
            meadow,
            must_be_base,
            requires_meadow_below,
            neighbours,
            dirty: self.dirty,
        }
    }

    pub fn apply(&mut self, action: TileEditorAction) -> Result<(), TileEditorError> {
        match action {
            TileEditorAction::Select(id) => {
                self.selected = self
                    .ids
                    .iter()
                    .position(|candidate| candidate == &id)
                    .ok_or(TileEditorError::UnknownTile(id))?;
            }
            TileEditorAction::Previous => {
                self.selected = self.selected.checked_sub(1).unwrap_or(self.ids.len() - 1);
            }
            TileEditorAction::Next => {
                self.selected = (self.selected + 1) % self.ids.len();
            }
            TileEditorAction::ToggleApproved => {
                let definition = self.selected_definition_mut();
                definition.status = match &definition.status {
                    TileStatus::Approved { .. } => TileStatus::Blocked {
                        reason: "not reviewed for map authoring".into(),
                    },
                    TileStatus::Blocked { .. } => TileStatus::Approved {
                        tags: BTreeSet::new(),
                        rules: Box::new(default_rules()),
                    },
                };
                self.dirty = true;
            }
            TileEditorAction::ToggleMeadowTag => {
                let tags = self.approved_parts_mut()?.0;
                if !tags.remove(&meadow_tag()) {
                    tags.insert(meadow_tag());
                }
                self.dirty = true;
            }
            TileEditorAction::ToggleStack(control) => {
                let rules = self.approved_parts_mut()?.1;
                match control {
                    StackControl::MustBeBase => toggle_rule(
                        &mut rules.stack,
                        |rule| matches!(rule, StackRule::MustBeBase),
                        StackRule::MustBeBase,
                    ),
                    StackControl::RequiresMeadowBelow => toggle_rule(
                        &mut rules.stack,
                        |rule| matches!(rule, StackRule::RequiresBelow { matcher } if is_meadow_matcher(matcher)),
                        StackRule::RequiresBelow {
                            matcher: meadow_matcher(),
                        },
                    ),
                }
                self.dirty = true;
            }
            TileEditorAction::CycleNeighbour(direction) => {
                let id = self.ids[self.selected].clone();
                if !self.pattern_neighbour_tiles(&id, direction).is_empty() {
                    return Err(TileEditorError::PatternNeighbourLocked { id, direction });
                }
                let rules = self.approved_parts_mut()?.1;
                let rule = match direction {
                    Direction8::North => &mut rules.neighbours.north,
                    Direction8::NorthEast => &mut rules.neighbours.north_east,
                    Direction8::East => &mut rules.neighbours.east,
                    Direction8::SouthEast => &mut rules.neighbours.south_east,
                    Direction8::South => &mut rules.neighbours.south,
                    Direction8::SouthWest => &mut rules.neighbours.south_west,
                    Direction8::West => &mut rules.neighbours.west,
                    Direction8::NorthWest => &mut rules.neighbours.north_west,
                };
                *rule = match rule {
                    NeighbourRule::Any => meadow_requirement_rule(true),
                    NeighbourRule::Requires { .. } => meadow_requirement_rule(false),
                    NeighbourRule::Forbids { .. } => NeighbourRule::Any,
                };
                self.dirty = true;
            }
        }
        Ok(())
    }

    pub fn catalog_json(&self) -> Result<String, TileEditorError> {
        self.catalog
            .validate(&self.known)
            .map_err(TileEditorError::Catalog)?;
        serde_json::to_string_pretty(&self.catalog)
            .map_err(|error| TileEditorError::Serialization(error.to_string()))
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    fn selected_definition(&self) -> &TileDefinition {
        let selected = &self.ids[self.selected];
        self.catalog
            .tiles
            .iter()
            .find(|definition| &definition.id == selected)
            .expect("validated catalog contains every palette tile")
    }

    fn selected_definition_mut(&mut self) -> &mut TileDefinition {
        let selected = &self.ids[self.selected];
        self.catalog
            .tiles
            .iter_mut()
            .find(|definition| &definition.id == selected)
            .expect("validated catalog contains every palette tile")
    }

    fn approved_parts_mut(
        &mut self,
    ) -> Result<(&mut BTreeSet<TileTag>, &mut TileHardRules), TileEditorError> {
        let id = self.ids[self.selected].clone();
        let definition = self.selected_definition_mut();
        match &mut definition.status {
            TileStatus::Approved { tags, rules } => Ok((tags, rules)),
            TileStatus::Blocked { .. } => Err(TileEditorError::TileBlocked(id)),
        }
    }

    fn preview(
        &self,
        tile: &AtomicTileId,
        direction: Direction8,
        rule: Option<&NeighbourRule>,
    ) -> NeighbourPreview {
        let pattern_tiles = self.pattern_neighbour_tiles(tile, direction);
        if !pattern_tiles.is_empty() {
            return NeighbourPreview {
                kind: NeighbourRuleKind::Requires,
                accepted_tiles: pattern_tiles,
                locked_by_pattern: true,
            };
        }
        let approved_tiles = self.approved_tiles();
        match rule.unwrap_or(&NeighbourRule::Any) {
            NeighbourRule::Any => NeighbourPreview {
                kind: NeighbourRuleKind::Any,
                accepted_tiles: approved_tiles,
                locked_by_pattern: false,
            },
            NeighbourRule::Requires { requirement } => NeighbourPreview {
                kind: NeighbourRuleKind::Requires,
                accepted_tiles: self.matching_tiles(&requirement.matcher),
                locked_by_pattern: false,
            },
            NeighbourRule::Forbids { requirement } => {
                let forbidden = self.matching_tiles(&requirement.matcher);
                NeighbourPreview {
                    kind: NeighbourRuleKind::Forbids,
                    accepted_tiles: approved_tiles
                        .into_iter()
                        .filter(|id| !forbidden.contains(id))
                        .collect(),
                    locked_by_pattern: false,
                }
            }
        }
    }

    fn approved_tiles(&self) -> Vec<AtomicTileId> {
        self.catalog
            .tiles
            .iter()
            .filter(|definition| matches!(definition.status, TileStatus::Approved { .. }))
            .map(|definition| definition.id.clone())
            .collect()
    }

    fn pattern_neighbour_tiles(
        &self,
        tile: &AtomicTileId,
        direction: Direction8,
    ) -> Vec<AtomicTileId> {
        self.catalog
            .patterns
            .iter()
            .flat_map(|pattern| {
                pattern
                    .parts
                    .iter()
                    .filter(move |part| &part.tile == tile)
                    .flat_map(move |part| {
                        pattern.parts.iter().filter_map(move |candidate| {
                            let delta_x = i32::from(candidate.coord.0) - i32::from(part.coord.0);
                            let delta_y = i32::from(candidate.coord.1) - i32::from(part.coord.1);
                            (direction_from_delta(delta_x, delta_y) == Some(direction))
                                .then(|| candidate.tile.clone())
                        })
                    })
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn matching_tiles(&self, matcher: &TileMatcher) -> Vec<AtomicTileId> {
        let mut ids = BTreeSet::new();
        self.collect_matching_tiles(matcher, &mut ids);
        ids.into_iter().collect()
    }

    fn collect_matching_tiles(&self, matcher: &TileMatcher, output: &mut BTreeSet<AtomicTileId>) {
        match matcher {
            TileMatcher::AtomicTile { tile } => {
                output.insert(tile.clone());
            }
            TileMatcher::Tagged { tag } => {
                output.extend(self.catalog.tiles.iter().filter_map(|definition| {
                    match &definition.status {
                        TileStatus::Approved { tags, .. } if tags.contains(tag) => {
                            Some(definition.id.clone())
                        }
                        _ => None,
                    }
                }));
            }
            TileMatcher::PatternPart { pattern, part } => {
                output.extend(self.catalog.patterns.iter().filter_map(|definition| {
                    (definition.id == *pattern)
                        .then(|| {
                            definition
                                .parts
                                .iter()
                                .find(|candidate| candidate.coord == *part)
                        })
                        .flatten()
                        .map(|candidate| candidate.tile.clone())
                }));
            }
            TileMatcher::AnyOf { matchers } => {
                for matcher in matchers {
                    self.collect_matching_tiles(matcher, output);
                }
            }
        }
    }
}

fn default_rules() -> TileHardRules {
    TileHardRules {
        stack: Vec::new(),
        neighbours: Neighbours8::filled(NeighbourRule::Any),
    }
}

fn meadow_tag() -> TileTag {
    TileTag::new("meadow").expect("fixed meadow tag is valid")
}

fn meadow_matcher() -> TileMatcher {
    TileMatcher::Tagged { tag: meadow_tag() }
}

fn is_meadow_matcher(matcher: &TileMatcher) -> bool {
    matches!(matcher, TileMatcher::Tagged { tag } if tag == &meadow_tag())
}

fn meadow_requirement_rule(requires: bool) -> NeighbourRule {
    let requirement = CellRequirement {
        scope: LayerScope::Base,
        matcher: meadow_matcher(),
    };
    if requires {
        NeighbourRule::Requires { requirement }
    } else {
        NeighbourRule::Forbids { requirement }
    }
}

fn direction_from_delta(delta_x: i32, delta_y: i32) -> Option<Direction8> {
    match (delta_x, delta_y) {
        (0, -1) => Some(Direction8::North),
        (1, -1) => Some(Direction8::NorthEast),
        (1, 0) => Some(Direction8::East),
        (1, 1) => Some(Direction8::SouthEast),
        (0, 1) => Some(Direction8::South),
        (-1, 1) => Some(Direction8::SouthWest),
        (-1, 0) => Some(Direction8::West),
        (-1, -1) => Some(Direction8::NorthWest),
        _ => None,
    }
}

fn toggle_rule(
    rules: &mut Vec<StackRule>,
    matches: impl Fn(&StackRule) -> bool,
    replacement: StackRule,
) {
    if let Some(index) = rules.iter().position(matches) {
        rules.remove(index);
    } else {
        rules.push(replacement);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TileEditorError {
    EmptyPalette,
    UnknownTile(AtomicTileId),
    TileBlocked(AtomicTileId),
    PatternNeighbourLocked {
        id: AtomicTileId,
        direction: Direction8,
    },
    Catalog(TileSemanticsError),
    Serialization(String),
}

impl fmt::Display for TileEditorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPalette => formatter.write_str("tile palette cannot be empty"),
            Self::UnknownTile(id) => write!(formatter, "unknown palette tile {id}"),
            Self::TileBlocked(id) => write!(formatter, "tile {id} is blocked"),
            Self::PatternNeighbourLocked { id, direction } => write!(
                formatter,
                "tile {id} has a pattern-defined {direction:?} neighbour"
            ),
            Self::Catalog(error) => write!(formatter, "invalid tile semantics catalog: {error}"),
            Self::Serialization(error) => write!(
                formatter,
                "cannot serialize tile semantics catalog: {error}"
            ),
        }
    }
}

impl Error for TileEditorError {}

#[cfg(test)]
mod tests {
    use super::*;
    use map_tile_semantics::{
        FORMAT_VERSION, PatternCoord, PatternDefinition, PatternId, PatternPart,
    };

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
        assert_eq!(editor.snapshot().id, tile("leaf"));
        editor.apply(TileEditorAction::Next).unwrap();
        assert_eq!(editor.snapshot().id, tile("ground"));
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
        let snapshot = editor.snapshot();
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
            editor.snapshot().neighbours.north,
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
            editor.snapshot().neighbours.north,
            NeighbourPreview {
                kind: NeighbourRuleKind::Any,
                accepted_tiles: vec![tile("ground"), tile("leaf")],
                locked_by_pattern: false,
            }
        );
        assert!(editor.catalog_json().unwrap().contains("ground"));
        editor.mark_saved();
        assert!(!editor.snapshot().dirty);
    }

    #[test]
    fn blocked_tiles_require_approval_before_property_edits() {
        let mut editor = editor();
        editor.apply(TileEditorAction::ToggleApproved).unwrap();
        assert!(!editor.snapshot().approved);
        assert!(matches!(
            editor.apply(TileEditorAction::ToggleMeadowTag),
            Err(TileEditorError::TileBlocked(_))
        ));
        editor.apply(TileEditorAction::ToggleApproved).unwrap();
        assert!(editor.snapshot().approved);
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
                        meadow_matcher(),
                    ],
                },
            },
        };
        let rules = editor.approved_parts_mut().unwrap().1;
        rules.neighbours.east = rule;
        assert_eq!(
            editor.snapshot().neighbours.east,
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
        let snapshot = editor.snapshot();
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
        assert!(!editor.snapshot().must_be_base);
        assert!(!editor.snapshot().requires_meadow_below);

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
            editor.snapshot().neighbours.east,
            NeighbourPreview {
                kind: NeighbourRuleKind::Requires,
                accepted_tiles: vec![tile("leaf")],
                locked_by_pattern: true,
            }
        );
        editor.apply(TileEditorAction::Next).unwrap();
        assert_eq!(
            editor.snapshot().neighbours.west,
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
}
