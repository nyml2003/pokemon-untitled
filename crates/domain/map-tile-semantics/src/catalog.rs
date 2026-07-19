impl TileSemanticsCatalog {
    /// 解析 JSON，并确认其为 `known` 所列图块的完整语义目录。
    /// 格式、未知图块、重复定义和规则不合法时返回 `TileSemanticsError`。
    pub fn from_json(
        json: &str,
        known: &BTreeSet<AtomicTileId>,
    ) -> Result<Self, TileSemanticsError> {
        let catalog: Self = serde_json::from_str(json)
            .map_err(|error| TileSemanticsError::Json(error.to_string()))?;
        catalog.validate(known)?;
        Ok(catalog)
    }

    /// 校验目录格式、图块覆盖范围、唯一性和规则引用。
    /// `known` 中的每个原子图块都必须恰有一个定义。
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

    /// 检查地图中每个图块层，并返回全部违反语义规则的诊断。
    /// 未定义或未批准的图块也会作为诊断返回。
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

pub(crate) fn validate_matcher(matcher: &TileMatcher) -> Result<(), TileSemanticsError> {
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

pub(crate) fn matches_neighbour_rule(
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

pub(crate) fn scoped_layers(layers: &[AtomicTileId], scope: LayerScope) -> &[AtomicTileId] {
    match scope {
        LayerScope::Any => layers,
        LayerScope::Base => layers.first().map_or(&[], std::slice::from_ref),
        LayerScope::Top => layers.last().map_or(&[], std::slice::from_ref),
    }
}

pub(crate) fn matches_tile(
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

pub(crate) fn direction_for(dx: i32, dy: i32) -> Option<Direction8> {
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
use std::collections::{BTreeMap, BTreeSet};

use map_project::{AtomicTileId, MapProject, TilePosition};

use crate::{
    Direction8, FORMAT_VERSION, LayerScope, MapSemanticDiagnostic, NeighbourRule,
    PatternDefinition, PatternPart, StackRule, TileDefinition, TileHardRules, TileMatcher,
    TileSemanticsCatalog, TileSemanticsError, TileStatus,
};
