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
    meadow_tag: TileTag,
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
        let meadow_tag = TileTag::new("meadow").map_err(TileEditorError::DefaultTag)?;
        Ok(Self {
            catalog,
            ids,
            known,
            meadow_tag,
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

    pub const fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Returns the selected tile's editor state.
    ///
    /// A catalog mutated outside this editor can invalidate the selected tile;
    /// that condition is reported instead of being treated as an invariant panic.
    pub fn snapshot(&self) -> Result<TileEditorSnapshot, TileEditorError> {
        let definition = self.selected_definition()?;
        let (approved, tags, rules) = match &definition.status {
            TileStatus::Approved { tags, rules } => (true, Some(tags), Some(rules.as_ref())),
            TileStatus::Blocked { .. } => (false, None, None),
        };
        let meadow = tags.is_some_and(|tags| tags.contains(&self.meadow_tag));
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
        Ok(TileEditorSnapshot {
            id: definition.id.clone(),
            approved,
            meadow,
            must_be_base,
            requires_meadow_below,
            neighbours,
            dirty: self.dirty,
        })
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
                let definition = self.selected_definition_mut()?;
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
                let meadow_tag = self.meadow_tag.clone();
                let tags = self.approved_parts_mut()?.0;
                if !tags.remove(&meadow_tag) {
                    tags.insert(meadow_tag);
                }
                self.dirty = true;
            }
            TileEditorAction::ToggleStack(control) => {
                let meadow_matcher = self.meadow_matcher();
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
                            matcher: meadow_matcher,
                        },
                    ),
                }
                self.dirty = true;
            }
            TileEditorAction::CycleNeighbour(direction) => {
                let id = self.selected_id()?.clone();
                if !self.pattern_neighbour_tiles(&id, direction).is_empty() {
                    return Err(TileEditorError::PatternNeighbourLocked { id, direction });
                }
                let meadow_matcher = self.meadow_matcher();
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
                    NeighbourRule::Any => meadow_requirement_rule(meadow_matcher.clone(), true),
                    NeighbourRule::Requires { .. } => {
                        meadow_requirement_rule(meadow_matcher, false)
                    }
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

    fn selected_definition(&self) -> Result<&TileDefinition, TileEditorError> {
        let selected = self.selected_id()?;
        self.catalog
            .tiles
            .iter()
            .find(|definition| &definition.id == selected)
            .ok_or_else(|| TileEditorError::SelectedTileMissing(selected.clone()))
    }

    fn selected_definition_mut(&mut self) -> Result<&mut TileDefinition, TileEditorError> {
        let selected = self.selected_id()?.clone();
        self.catalog
            .tiles
            .iter_mut()
            .find(|definition| definition.id == selected)
            .ok_or(TileEditorError::SelectedTileMissing(selected))
    }

    fn selected_id(&self) -> Result<&AtomicTileId, TileEditorError> {
        self.ids
            .get(self.selected)
            .ok_or(TileEditorError::SelectedIndexMissing {
                index: self.selected,
            })
    }

    fn approved_parts_mut(
        &mut self,
    ) -> Result<(&mut BTreeSet<TileTag>, &mut TileHardRules), TileEditorError> {
        let id = self.selected_id()?.clone();
        let definition = self.selected_definition_mut()?;
        match &mut definition.status {
            TileStatus::Approved { tags, rules } => Ok((tags, rules)),
            TileStatus::Blocked { .. } => Err(TileEditorError::TileBlocked(id)),
        }
    }

    fn meadow_matcher(&self) -> TileMatcher {
        TileMatcher::Tagged {
            tag: self.meadow_tag.clone(),
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

fn is_meadow_matcher(matcher: &TileMatcher) -> bool {
    matches!(matcher, TileMatcher::Tagged { tag } if tag.as_str() == "meadow")
}

fn meadow_requirement_rule(matcher: TileMatcher, requires: bool) -> NeighbourRule {
    let requirement = CellRequirement {
        scope: LayerScope::Base,
        matcher,
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
    DefaultTag(TileSemanticsError),
    SelectedIndexMissing {
        index: usize,
    },
    SelectedTileMissing(AtomicTileId),
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
            Self::DefaultTag(error) => write!(formatter, "invalid default tile tag: {error}"),
            Self::SelectedIndexMissing { index } => {
                write!(formatter, "selected palette index {index} is missing")
            }
            Self::SelectedTileMissing(id) => {
                write!(
                    formatter,
                    "selected palette tile {id} is missing from the catalog"
                )
            }
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
#[path = "../tests/unit/lib.rs"]
mod tests;
