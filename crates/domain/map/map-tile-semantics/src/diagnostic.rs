#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MapSemanticDiagnostic {
    pub position: TilePosition,
    pub layer_index: usize,
    pub tile: AtomicTileId,
    pub rule: SemanticRuleLocation,
    pub expected: String,
    pub actual_layers: Vec<AtomicTileId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticRuleLocation {
    Catalog,
    Stack,
    Neighbour(Direction8),
    Pattern {
        pattern: PatternId,
        direction: Direction8,
    },
}

impl MapSemanticDiagnostic {
    pub(crate) fn missing_definition(
        position: TilePosition,
        layer_index: usize,
        tile: AtomicTileId,
    ) -> Self {
        Self {
            position,
            layer_index,
            tile,
            rule: SemanticRuleLocation::Catalog,
            expected: "a semantic definition".into(),
            actual_layers: Vec::new(),
        }
    }
    pub(crate) fn blocked(position: TilePosition, layer_index: usize, tile: AtomicTileId) -> Self {
        Self {
            position,
            layer_index,
            tile,
            rule: SemanticRuleLocation::Catalog,
            expected: "an approved tile".into(),
            actual_layers: Vec::new(),
        }
    }
    pub(crate) fn stack(
        position: TilePosition,
        layer_index: usize,
        tile: AtomicTileId,
        rule: StackRule,
        actual_layers: &[AtomicTileId],
    ) -> Self {
        Self {
            position,
            layer_index,
            tile,
            rule: SemanticRuleLocation::Stack,
            expected: format!("{rule:?}"),
            actual_layers: actual_layers.to_vec(),
        }
    }
    pub(crate) fn neighbour(
        position: TilePosition,
        layer_index: usize,
        tile: AtomicTileId,
        direction: Direction8,
        rule: NeighbourRule,
        actual_layers: &[AtomicTileId],
    ) -> Self {
        Self {
            position,
            layer_index,
            tile,
            rule: SemanticRuleLocation::Neighbour(direction),
            expected: format!("{rule:?}"),
            actual_layers: actual_layers.to_vec(),
        }
    }
    pub(crate) fn pattern(
        position: TilePosition,
        layer_index: usize,
        tile: AtomicTileId,
        pattern: PatternId,
        direction: Direction8,
        expected: AtomicTileId,
    ) -> Self {
        Self {
            position,
            layer_index,
            tile,
            rule: SemanticRuleLocation::Pattern { pattern, direction },
            expected: expected.to_string(),
            actual_layers: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TileSemanticsError {
    EmptyId(&'static str),
    UnsupportedFormat(String),
    UnknownTile(AtomicTileId),
    DuplicateTile(AtomicTileId),
    CoverageMismatch {
        expected: usize,
        actual: usize,
    },
    DuplicatePattern(PatternId),
    DuplicatePatternPart {
        pattern: PatternId,
        coord: PatternCoord,
    },
    EmptyAnyOf,
    Json(String),
}

impl fmt::Display for TileSemanticsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId(kind) => write!(formatter, "{kind} must not be empty"),
            Self::UnsupportedFormat(version) => {
                write!(formatter, "unsupported tile semantics format {version}")
            }
            Self::UnknownTile(tile) => {
                write!(formatter, "tile {tile} is not known to the asset catalog")
            }
            Self::DuplicateTile(tile) => {
                write!(formatter, "tile {tile} has multiple semantic definitions")
            }
            Self::CoverageMismatch { expected, actual } => write!(
                formatter,
                "tile semantics cover {actual} tiles, expected {expected}"
            ),
            Self::DuplicatePattern(pattern) => {
                write!(formatter, "pattern {pattern} is defined more than once")
            }
            Self::DuplicatePatternPart { pattern, coord } => write!(
                formatter,
                "pattern {pattern} defines part {coord:?} more than once"
            ),
            Self::EmptyAnyOf => formatter.write_str("any_of matcher must not be empty"),
            Self::Json(message) => write!(formatter, "invalid tile semantics JSON: {message}"),
        }
    }
}

impl Error for TileSemanticsError {}
use std::{error::Error, fmt};

use map_project::{AtomicTileId, TilePosition};
use serde::Serialize;

use crate::{Direction8, NeighbourRule, PatternCoord, PatternId, StackRule};
