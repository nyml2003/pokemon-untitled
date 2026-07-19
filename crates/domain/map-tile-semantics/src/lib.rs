//! Pure semantic rules for layered map tiles.

#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use map_project::{AtomicTileId, MapProject, TilePosition};
use serde::{Deserialize, Serialize};

pub const FORMAT_VERSION: &str = "map-tile-semantics-v1";

macro_rules! string_id {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
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

    const fn delta(self) -> (i32, i32) {
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

impl TileSemanticsCatalog {
    pub fn from_json(
        json: &str,
        known: &BTreeSet<AtomicTileId>,
    ) -> Result<Self, TileSemanticsError> {
        let catalog: Self = serde_json::from_str(json)
            .map_err(|error| TileSemanticsError::Json(error.to_string()))?;
        catalog.validate(known)?;
        Ok(catalog)
    }

    pub fn validate(&self, known: &BTreeSet<AtomicTileId>) -> Result<(), TileSemanticsError> {
        if self.format_version != FORMAT_VERSION {
            return Err(TileSemanticsError::UnsupportedFormat(
                self.format_version.clone(),
            ));
        }
        let mut definitions = BTreeMap::new();
        for definition in &self.tiles {
            if !known.contains(&definition.id) {
                return Err(TileSemanticsError::UnknownTile(definition.id.clone()));
            }
            if definitions
                .insert(definition.id.clone(), definition)
                .is_some()
            {
                return Err(TileSemanticsError::DuplicateTile(definition.id.clone()));
            }
            if let TileStatus::Approved { tags, rules } = &definition.status {
                if tags.iter().any(|tag| tag.as_str().trim().is_empty()) {
                    return Err(TileSemanticsError::EmptyId("TileTag"));
                }
                validate_rules(rules)?;
            }
        }
        if definitions.len() != known.len()
            || known.iter().any(|tile| !definitions.contains_key(tile))
        {
            return Err(TileSemanticsError::CoverageMismatch {
                expected: known.len(),
                actual: definitions.len(),
            });
        }
        let mut patterns = BTreeSet::new();
        for pattern in &self.patterns {
            if !patterns.insert(pattern.id.clone()) {
                return Err(TileSemanticsError::DuplicatePattern(pattern.id.clone()));
            }
            let mut coordinates = BTreeSet::new();
            for part in &pattern.parts {
                if !known.contains(&part.tile) {
                    return Err(TileSemanticsError::UnknownTile(part.tile.clone()));
                }
                if !coordinates.insert(part.coord) {
                    return Err(TileSemanticsError::DuplicatePatternPart {
                        pattern: pattern.id.clone(),
                        coord: part.coord,
                    });
                }
            }
        }
        Ok(())
    }

    pub fn lint(&self, project: &MapProject) -> Vec<MapSemanticDiagnostic> {
        let definitions = self
            .tiles
            .iter()
            .map(|definition| (definition.id.clone(), definition))
            .collect::<BTreeMap<_, _>>();
        let mut diagnostics = Vec::new();
        for row in 0..project.height {
            for column in 0..project.width {
                let position = TilePosition::new(column, row);
                let layers = cell_layers(project, position);
                for (layer_index, tile) in layers.iter().enumerate() {
                    let Some(definition) = definitions.get(tile) else {
                        diagnostics.push(MapSemanticDiagnostic::missing_definition(
                            position,
                            layer_index,
                            (*tile).clone(),
                        ));
                        continue;
                    };
                    let TileStatus::Approved { rules, .. } = &definition.status else {
                        diagnostics.push(MapSemanticDiagnostic::blocked(
                            position,
                            layer_index,
                            (*tile).clone(),
                        ));
                        continue;
                    };
                    for rule in &rules.stack {
                        if !matches_stack_rule(rule, layers, layer_index, &definitions, self) {
                            diagnostics.push(MapSemanticDiagnostic::stack(
                                position,
                                layer_index,
                                (*tile).clone(),
                                rule.clone(),
                                layers,
                            ));
                        }
                    }
                    for direction in Direction8::ALL {
                        let rule = rules.neighbours.get(direction);
                        if !matches_neighbour_rule(
                            rule,
                            neighbour_layers(project, position, direction),
                            &definitions,
                            self,
                        ) {
                            diagnostics.push(MapSemanticDiagnostic::neighbour(
                                position,
                                layer_index,
                                (*tile).clone(),
                                direction,
                                rule.clone(),
                                neighbour_layers(project, position, direction),
                            ));
                        }
                    }
                    lint_patterns(self, project, position, layer_index, tile, &mut diagnostics);
                }
            }
        }
        diagnostics
    }
}

fn validate_rules(rules: &TileHardRules) -> Result<(), TileSemanticsError> {
    for rule in &rules.stack {
        if let StackRule::RequiresBelow { matcher } = rule {
            validate_matcher(matcher)?;
        }
    }
    for direction in Direction8::ALL {
        match rules.neighbours.get(direction) {
            NeighbourRule::Any => {}
            NeighbourRule::Requires { requirement } | NeighbourRule::Forbids { requirement } => {
                validate_matcher(&requirement.matcher)?
            }
        }
    }
    Ok(())
}

fn validate_matcher(matcher: &TileMatcher) -> Result<(), TileSemanticsError> {
    match matcher {
        TileMatcher::AnyOf { matchers } if matchers.is_empty() => {
            Err(TileSemanticsError::EmptyAnyOf)
        }
        TileMatcher::AnyOf { matchers } => {
            for matcher in matchers {
                validate_matcher(matcher)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn cell_layers(project: &MapProject, position: TilePosition) -> &[AtomicTileId] {
    project
        .cell_index(position)
        .and_then(|index| project.visual_cells[index].material.as_ref())
        .and_then(|id| project.material(id))
        .map_or(&[], |material| material.layers.as_slice())
}

fn neighbour_layers(
    project: &MapProject,
    position: TilePosition,
    direction: Direction8,
) -> &[AtomicTileId] {
    let (delta_x, delta_y) = direction.delta();
    let x = i32::from(position.x()) + delta_x;
    let y = i32::from(position.y()) + delta_y;
    if x < 0 || y < 0 || x >= i32::from(project.width) || y >= i32::from(project.height) {
        return &[];
    }
    cell_layers(project, TilePosition::new(x as u16, y as u16))
}

fn matches_stack_rule(
    rule: &StackRule,
    layers: &[AtomicTileId],
    layer_index: usize,
    definitions: &BTreeMap<AtomicTileId, &TileDefinition>,
    catalog: &TileSemanticsCatalog,
) -> bool {
    match rule {
        StackRule::MustBeBase => layer_index == 0,
        StackRule::RequiresBelow { matcher } => layers[..layer_index]
            .iter()
            .any(|tile| matches_tile(matcher, tile, definitions, catalog)),
    }
}

fn matches_neighbour_rule(
    rule: &NeighbourRule,
    layers: &[AtomicTileId],
    definitions: &BTreeMap<AtomicTileId, &TileDefinition>,
    catalog: &TileSemanticsCatalog,
) -> bool {
    match rule {
        NeighbourRule::Any => true,
        NeighbourRule::Requires { requirement } => scoped_layers(layers, requirement.scope)
            .iter()
            .any(|tile| matches_tile(&requirement.matcher, tile, definitions, catalog)),
        NeighbourRule::Forbids { requirement } => !scoped_layers(layers, requirement.scope)
            .iter()
            .any(|tile| matches_tile(&requirement.matcher, tile, definitions, catalog)),
    }
}

fn scoped_layers(layers: &[AtomicTileId], scope: LayerScope) -> &[AtomicTileId] {
    match scope {
        LayerScope::Any => layers,
        LayerScope::Base => layers.first().map_or(&[], std::slice::from_ref),
        LayerScope::Top => layers.last().map_or(&[], std::slice::from_ref),
    }
}

fn matches_tile(
    matcher: &TileMatcher,
    tile: &AtomicTileId,
    definitions: &BTreeMap<AtomicTileId, &TileDefinition>,
    catalog: &TileSemanticsCatalog,
) -> bool {
    match matcher {
        TileMatcher::AtomicTile { tile: expected } => expected == tile,
        TileMatcher::Tagged { tag } => {
            matches!(definitions.get(tile).map(|definition| &definition.status), Some(TileStatus::Approved { tags, .. }) if tags.contains(tag))
        }
        TileMatcher::PatternPart { pattern, part } => catalog.patterns.iter().any(|candidate| {
            candidate.id == *pattern
                && candidate
                    .parts
                    .iter()
                    .any(|candidate| candidate.coord == *part && candidate.tile == *tile)
        }),
        TileMatcher::AnyOf { matchers } => matchers
            .iter()
            .any(|matcher| matches_tile(matcher, tile, definitions, catalog)),
    }
}

fn lint_patterns(
    catalog: &TileSemanticsCatalog,
    project: &MapProject,
    position: TilePosition,
    layer_index: usize,
    tile: &AtomicTileId,
    diagnostics: &mut Vec<MapSemanticDiagnostic>,
) {
    for pattern in &catalog.patterns {
        let roles = pattern
            .parts
            .iter()
            .filter(|part| part.tile == *tile)
            .collect::<Vec<_>>();
        if roles.is_empty() {
            continue;
        }
        if roles
            .iter()
            .any(|role| pattern_role_matches(project, position, pattern, role))
        {
            continue;
        }
        let role = roles[0];
        for expected in &pattern.parts {
            let dx = i32::from(expected.coord.0) - i32::from(role.coord.0);
            let dy = i32::from(expected.coord.1) - i32::from(role.coord.1);
            if dx == 0 && dy == 0 || dx.unsigned_abs() > 1 || dy.unsigned_abs() > 1 {
                continue;
            }
            let direction = direction_for(dx, dy).expect("adjacent delta has a direction");
            if !neighbour_layers(project, position, direction).contains(&expected.tile) {
                diagnostics.push(MapSemanticDiagnostic::pattern(
                    position,
                    layer_index,
                    tile.clone(),
                    pattern.id.clone(),
                    direction,
                    expected.tile.clone(),
                ));
            }
        }
    }
}

fn pattern_role_matches(
    project: &MapProject,
    position: TilePosition,
    pattern: &PatternDefinition,
    role: &PatternPart,
) -> bool {
    pattern.parts.iter().all(|expected| {
        let x = i32::from(position.x()) + i32::from(expected.coord.0) - i32::from(role.coord.0);
        let y = i32::from(position.y()) + i32::from(expected.coord.1) - i32::from(role.coord.1);
        x >= 0
            && y >= 0
            && x < i32::from(project.width)
            && y < i32::from(project.height)
            && cell_layers(project, TilePosition::new(x as u16, y as u16)).contains(&expected.tile)
    })
}

fn direction_for(dx: i32, dy: i32) -> Option<Direction8> {
    match (dx, dy) {
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
    fn missing_definition(position: TilePosition, layer_index: usize, tile: AtomicTileId) -> Self {
        Self {
            position,
            layer_index,
            tile,
            rule: SemanticRuleLocation::Catalog,
            expected: "a semantic definition".into(),
            actual_layers: Vec::new(),
        }
    }
    fn blocked(position: TilePosition, layer_index: usize, tile: AtomicTileId) -> Self {
        Self {
            position,
            layer_index,
            tile,
            rule: SemanticRuleLocation::Catalog,
            expected: "an approved tile".into(),
            actual_layers: Vec::new(),
        }
    }
    fn stack(
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
    fn neighbour(
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
    fn pattern(
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

#[cfg(test)]
mod tests {
    use super::*;
    use map_project::{CompositeTile, CompositeTileId, MapProjectId};

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
}
