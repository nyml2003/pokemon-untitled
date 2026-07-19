macro_rules! string_id {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// 以非空且非纯空白字符串创建语义标识。
            pub fn new(value: impl Into<String>) -> Result<Self, TileSemanticsError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(TileSemanticsError::EmptyId(stringify!($name)));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

string_id!(PatternId);
string_id!(TileTag);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PatternCoord(pub u16, pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction8 {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl Direction8 {
    pub const ALL: [Self; 8] = [
        Self::North,
        Self::NorthEast,
        Self::East,
        Self::SouthEast,
        Self::South,
        Self::SouthWest,
        Self::West,
        Self::NorthWest,
    ];

    pub(crate) const fn delta(self) -> (i32, i32) {
        match self {
            Self::North => (0, -1),
            Self::NorthEast => (1, -1),
            Self::East => (1, 0),
            Self::SouthEast => (1, 1),
            Self::South => (0, 1),
            Self::SouthWest => (-1, 1),
            Self::West => (-1, 0),
            Self::NorthWest => (-1, -1),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Neighbours8<T> {
    pub north: T,
    pub north_east: T,
    pub east: T,
    pub south_east: T,
    pub south: T,
    pub south_west: T,
    pub west: T,
    pub north_west: T,
}

impl<T> Neighbours8<T> {
    pub fn get(&self, direction: Direction8) -> &T {
        match direction {
            Direction8::North => &self.north,
            Direction8::NorthEast => &self.north_east,
            Direction8::East => &self.east,
            Direction8::SouthEast => &self.south_east,
            Direction8::South => &self.south,
            Direction8::SouthWest => &self.south_west,
            Direction8::West => &self.west,
            Direction8::NorthWest => &self.north_west,
        }
    }
}

impl<T: Clone> Neighbours8<T> {
    pub fn filled(value: T) -> Self {
        Self {
            north: value.clone(),
            north_east: value.clone(),
            east: value.clone(),
            south_east: value.clone(),
            south: value.clone(),
            south_west: value.clone(),
            west: value.clone(),
            north_west: value,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerScope {
    Any,
    Base,
    Top,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TileMatcher {
    AtomicTile {
        tile: AtomicTileId,
    },
    Tagged {
        tag: TileTag,
    },
    PatternPart {
        pattern: PatternId,
        part: PatternCoord,
    },
    AnyOf {
        matchers: Vec<TileMatcher>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellRequirement {
    pub scope: LayerScope,
    pub matcher: TileMatcher,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StackRule {
    MustBeBase,
    RequiresBelow { matcher: TileMatcher },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NeighbourRule {
    Any,
    Requires { requirement: CellRequirement },
    Forbids { requirement: CellRequirement },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileHardRules {
    #[serde(default)]
    pub stack: Vec<StackRule>,
    pub neighbours: Neighbours8<NeighbourRule>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TileStatus {
    Approved {
        #[serde(default)]
        tags: BTreeSet<TileTag>,
        rules: Box<TileHardRules>,
    },
    Blocked {
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileDefinition {
    pub id: AtomicTileId,
    #[serde(flatten)]
    pub status: TileStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternPart {
    pub coord: PatternCoord,
    pub tile: AtomicTileId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternDefinition {
    pub id: PatternId,
    pub parts: Vec<PatternPart>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileSemanticsCatalog {
    pub format_version: String,
    pub tiles: Vec<TileDefinition>,
    #[serde(default)]
    pub patterns: Vec<PatternDefinition>,
}

#[cfg(test)]
#[path = "../tests/unit/model.rs"]
mod tests;
use std::{collections::BTreeSet, fmt};

use map_project::AtomicTileId;
use serde::{Deserialize, Serialize};

use crate::TileSemanticsError;
