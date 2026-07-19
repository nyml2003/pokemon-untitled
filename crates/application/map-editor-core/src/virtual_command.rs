use std::{error::Error, fmt};

use map_project::{
    AtomicTileId, Collision, CompositeTile, CompositeTileId, MapError, MapEventKind, TilePosition,
};
use map_tile_semantics::MapSemanticDiagnostic;
use serde::{Deserialize, Serialize};

use crate::{EditorEffect, EditorIntent, EditorModel, EditorTool};

/// A coordinate and ID based editor API for agents and non-window clients.
/// It deliberately has no pointer, keyboard, viewport, or rendering concepts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorVirtualCommand {
    Inspect,
    ValidateSemantics,
    SelectAtomic {
        tile: AtomicTileId,
    },
    SelectMaterial {
        material: CompositeTileId,
    },
    PaintVisual {
        cells: Vec<TilePosition>,
        material: Option<CompositeTileId>,
    },
    PaintCollision {
        cells: Vec<TilePosition>,
        collision: Collision,
    },
    PaintEvent {
        cells: Vec<TilePosition>,
        event: Option<MapEventKind>,
    },
    CreateMaterial {
        material: CompositeTile,
    },
    AppendAtomicLayer {
        material: CompositeTileId,
        tile: AtomicTileId,
    },
    RemoveTopLayer {
        material: CompositeTileId,
    },
    DeleteMaterial {
        material: CompositeTileId,
    },
    Undo,
    Redo,
    Save,
}

/// Read-only state returned to an agent. It intentionally excludes undo history
/// and other editor implementation details.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EditorVirtualState {
    pub project: map_project::MapProject,
    pub atomic_tiles: Vec<AtomicTileId>,
    pub selected_atomic: Option<AtomicTileId>,
    pub selected_material: Option<CompositeTileId>,
    pub tool: EditorTool,
    pub dirty: bool,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum EditorVirtualCommandResult {
    State(Box<EditorVirtualState>),
    Diagnostics(Vec<MapSemanticDiagnostic>),
    Effect(EditorEffect),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorVirtualCommandError {
    EmptyCells,
    CellOutOfBounds(TilePosition),
    UnknownAtomicTile(AtomicTileId),
    UnknownMaterial(CompositeTileId),
    SemanticCatalogUnavailable,
    Map(MapError),
}

impl fmt::Display for EditorVirtualCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCells => formatter.write_str("a paint command requires at least one cell"),
            Self::CellOutOfBounds(position) => {
                write!(
                    formatter,
                    "cell {}, {} is outside the map",
                    position.x(),
                    position.y()
                )
            }
            Self::UnknownAtomicTile(tile) => {
                write!(formatter, "atomic tile {tile} is not available")
            }
            Self::UnknownMaterial(material) => {
                write!(formatter, "material {material} is not available")
            }
            Self::SemanticCatalogUnavailable => {
                formatter.write_str("tile semantic catalog is not configured")
            }
            Self::Map(error) => error.fmt(formatter),
        }
    }
}

impl Error for EditorVirtualCommandError {}

impl EditorModel {
    /// Executes either a read-only inspection or a state-changing virtual command.
    pub fn execute_virtual_command(
        &self,
        command: EditorVirtualCommand,
    ) -> Result<(Self, EditorVirtualCommandResult), EditorVirtualCommandError> {
        match command {
            EditorVirtualCommand::Inspect => Ok((
                self.clone(),
                EditorVirtualCommandResult::State(Box::new(self.virtual_state())),
            )),
            EditorVirtualCommand::ValidateSemantics => Ok((
                self.clone(),
                EditorVirtualCommandResult::Diagnostics(
                    self.semantic_diagnostics()
                        .ok_or(EditorVirtualCommandError::SemanticCatalogUnavailable)?,
                ),
            )),
            command => {
                let (model, effect) = self.apply_virtual_command(command)?;
                Ok((model, EditorVirtualCommandResult::Effect(effect)))
            }
        }
    }

    /// Applies a virtual command through the same reducer used by the UI.
    pub fn apply_virtual_command(
        &self,
        command: EditorVirtualCommand,
    ) -> Result<(Self, EditorEffect), EditorVirtualCommandError> {
        match command {
            EditorVirtualCommand::Inspect => Ok((self.clone(), EditorEffect::None)),
            EditorVirtualCommand::ValidateSemantics => Ok((self.clone(), EditorEffect::None)),
            EditorVirtualCommand::SelectAtomic { tile } => {
                let index = self.atomic_index(&tile)?;
                self.reduce(EditorIntent::SelectAtomic(index))
                    .map_err(EditorVirtualCommandError::Map)
            }
            EditorVirtualCommand::SelectMaterial { material } => {
                let index = self.material_index(&material)?;
                self.reduce(EditorIntent::SelectMaterial(index))
                    .map_err(EditorVirtualCommandError::Map)
            }
            EditorVirtualCommand::PaintVisual { cells, material } => {
                self.paint_virtual(cells, material, EditorTool::Visual)
            }
            EditorVirtualCommand::PaintCollision { cells, collision } => {
                self.paint_virtual(cells, None, EditorTool::Collision(collision))
            }
            EditorVirtualCommand::PaintEvent { cells, event } => {
                self.paint_virtual(cells, None, EditorTool::Event(event))
            }
            EditorVirtualCommand::CreateMaterial { material } => self
                .reduce(EditorIntent::CreateMaterial(material))
                .map_err(EditorVirtualCommandError::Map),
            EditorVirtualCommand::AppendAtomicLayer { material, tile } => {
                let material = self.material_index(&material)?;
                let tile = self.atomic_index(&tile)?;
                self.reduce_many([
                    EditorIntent::SelectMaterial(material),
                    EditorIntent::SelectAtomic(tile),
                    EditorIntent::AddLayer,
                ])
            }
            EditorVirtualCommand::RemoveTopLayer { material } => {
                let material = self.material_index(&material)?;
                self.reduce_many([
                    EditorIntent::SelectMaterial(material),
                    EditorIntent::RemoveLayer,
                ])
            }
            EditorVirtualCommand::DeleteMaterial { material } => {
                let material = self.material_index(&material)?;
                self.reduce_many([
                    EditorIntent::SelectMaterial(material),
                    EditorIntent::DeleteMaterial,
                ])
            }
            EditorVirtualCommand::Undo => self
                .reduce(EditorIntent::Undo)
                .map_err(EditorVirtualCommandError::Map),
            EditorVirtualCommand::Redo => self
                .reduce(EditorIntent::Redo)
                .map_err(EditorVirtualCommandError::Map),
            EditorVirtualCommand::Save => self
                .reduce(EditorIntent::Save)
                .map_err(EditorVirtualCommandError::Map),
        }
    }

    fn virtual_state(&self) -> EditorVirtualState {
        EditorVirtualState {
            project: self.project.clone(),
            atomic_tiles: self.atomic_ids.clone(),
            selected_atomic: self.atomic_ids.get(self.selected_atomic).cloned(),
            selected_material: self
                .project
                .materials
                .get(self.selected_material)
                .map(|material| material.id.clone()),
            tool: self.tool,
            dirty: self.dirty,
            status: self.status.clone(),
        }
    }

    fn paint_virtual(
        &self,
        cells: Vec<TilePosition>,
        material: Option<CompositeTileId>,
        tool: EditorTool,
    ) -> Result<(Self, EditorEffect), EditorVirtualCommandError> {
        validate_cells(self, &cells)?;
        let mut intents = Vec::with_capacity(cells.len().saturating_add(2));
        let erase = matches!(tool, EditorTool::Visual) && material.is_none();
        if let Some(material) = material {
            intents.push(EditorIntent::SelectMaterial(
                self.material_index(&material)?,
            ));
        } else {
            intents.push(EditorIntent::SelectTool(tool));
        }
        intents.extend(
            cells
                .into_iter()
                .map(|position| EditorIntent::Paint { position, erase }),
        );
        self.reduce_many(intents)
    }

    fn reduce_many(
        &self,
        intents: impl IntoIterator<Item = EditorIntent>,
    ) -> Result<(Self, EditorEffect), EditorVirtualCommandError> {
        let mut model = self.clone();
        let mut effect = EditorEffect::None;
        for intent in intents {
            let (next, next_effect) = model
                .reduce(intent)
                .map_err(EditorVirtualCommandError::Map)?;
            model = next;
            effect = next_effect;
        }
        Ok((model, effect))
    }

    fn atomic_index(&self, tile: &AtomicTileId) -> Result<usize, EditorVirtualCommandError> {
        self.atomic_ids
            .iter()
            .position(|candidate| candidate == tile)
            .ok_or_else(|| EditorVirtualCommandError::UnknownAtomicTile(tile.clone()))
    }

    fn material_index(
        &self,
        material: &CompositeTileId,
    ) -> Result<usize, EditorVirtualCommandError> {
        self.project
            .materials
            .iter()
            .position(|candidate| candidate.id == *material)
            .ok_or_else(|| EditorVirtualCommandError::UnknownMaterial(material.clone()))
    }
}

fn validate_cells(
    model: &EditorModel,
    cells: &[TilePosition],
) -> Result<(), EditorVirtualCommandError> {
    let Some(first) = cells.first() else {
        return Err(EditorVirtualCommandError::EmptyCells);
    };
    if model.project.cell_index(*first).is_none() {
        return Err(EditorVirtualCommandError::CellOutOfBounds(*first));
    }
    for position in &cells[1..] {
        if model.project.cell_index(*position).is_none() {
            return Err(EditorVirtualCommandError::CellOutOfBounds(*position));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use map_project::{CompositeTile, MapProject, MapProjectId};
    use map_tile_semantics::{
        FORMAT_VERSION, Neighbours8, TileDefinition, TileHardRules, TileSemanticsCatalog,
        TileStatus,
    };

    use super::*;

    fn model() -> EditorModel {
        let grass = AtomicTileId::new("grass").unwrap();
        let rock = AtomicTileId::new("rock").unwrap();
        let base = CompositeTile::new(CompositeTileId::new("base").unwrap(), vec![grass.clone()]);
        EditorModel::new(
            MapProject::blank(MapProjectId::new("test").unwrap(), 3, 2, Some(base)),
            vec![grass, rock],
        )
    }

    fn model_with_semantics() -> EditorModel {
        let model = model();
        let semantics = TileSemanticsCatalog {
            format_version: FORMAT_VERSION.into(),
            tiles: vec![
                TileDefinition {
                    id: AtomicTileId::new("grass").unwrap(),
                    status: TileStatus::Approved {
                        tags: Default::default(),
                        rules: Box::new(TileHardRules {
                            stack: Vec::new(),
                            neighbours: Neighbours8::filled(map_tile_semantics::NeighbourRule::Any),
                        }),
                    },
                },
                TileDefinition {
                    id: AtomicTileId::new("rock").unwrap(),
                    status: TileStatus::Blocked {
                        reason: "fixture".into(),
                    },
                },
            ],
            patterns: Vec::new(),
        };
        EditorModel::with_semantics(model.project, model.atomic_ids, semantics)
    }

    #[test]
    fn paints_batches_without_window_or_grid_coordinates() {
        let model = model();
        let (model, effect) = model
            .apply_virtual_command(EditorVirtualCommand::PaintCollision {
                cells: vec![TilePosition::new(0, 0), TilePosition::new(2, 1)],
                collision: Collision::Blocked,
            })
            .unwrap();
        assert_eq!(effect, EditorEffect::None);
        assert_eq!(
            model
                .project
                .cell(TilePosition::new(0, 0))
                .unwrap()
                .collision,
            Collision::Blocked
        );
        assert_eq!(
            model
                .project
                .cell(TilePosition::new(2, 1))
                .unwrap()
                .collision,
            Collision::Blocked
        );

        let (model, _) = model
            .apply_virtual_command(EditorVirtualCommand::PaintEvent {
                cells: vec![TilePosition::new(1, 0)],
                event: Some(MapEventKind::Encounter),
            })
            .unwrap();
        assert_eq!(
            model.project.cell(TilePosition::new(1, 0)).unwrap().event,
            Some(MapEventKind::Encounter)
        );
    }

    #[test]
    fn resolves_resources_by_id_and_keeps_save_as_an_effect() {
        let model = model();
        let (model, _) = model
            .apply_virtual_command(EditorVirtualCommand::AppendAtomicLayer {
                material: CompositeTileId::new("base").unwrap(),
                tile: AtomicTileId::new("rock").unwrap(),
            })
            .unwrap();
        assert_eq!(model.project.materials.len(), 2);
        assert_eq!(model.project.materials[1].layers.len(), 2);
        let (_, effect) = model
            .apply_virtual_command(EditorVirtualCommand::Save)
            .unwrap();
        assert_eq!(effect, EditorEffect::SaveRequested);
    }

    #[test]
    fn creates_a_named_composition_for_non_window_clients() {
        let model = model();
        let material = CompositeTile::new(
            CompositeTileId::new("tall-grass").unwrap(),
            vec![
                AtomicTileId::new("grass").unwrap(),
                AtomicTileId::new("rock").unwrap(),
            ],
        );
        let (model, _) = model
            .apply_virtual_command(EditorVirtualCommand::CreateMaterial {
                material: material.clone(),
            })
            .unwrap();
        assert_eq!(model.project.material(&material.id), Some(&material));
        assert!(matches!(
            model.apply_virtual_command(EditorVirtualCommand::CreateMaterial { material }),
            Err(EditorVirtualCommandError::Map(MapError::DuplicateMaterial(
                _
            )))
        ));
    }

    #[test]
    fn returns_semantic_diagnostics_without_mutating_the_editor() {
        let semantic_model = model_with_semantics();
        let (next, result) = semantic_model
            .execute_virtual_command(EditorVirtualCommand::ValidateSemantics)
            .unwrap();
        assert_eq!(next.project, semantic_model.project);
        assert!(
            matches!(result, EditorVirtualCommandResult::Diagnostics(ref diagnostics) if diagnostics.is_empty())
        );

        let (_, result) = semantic_model
            .execute_virtual_command(EditorVirtualCommand::PaintVisual {
                cells: vec![TilePosition::new(0, 0)],
                material: Some(CompositeTileId::new("base").unwrap()),
            })
            .unwrap();
        assert!(matches!(
            result,
            EditorVirtualCommandResult::Effect(EditorEffect::None)
        ));

        assert!(matches!(
            model().execute_virtual_command(EditorVirtualCommand::ValidateSemantics),
            Err(EditorVirtualCommandError::SemanticCatalogUnavailable)
        ));
    }

    #[test]
    fn rejects_invalid_agent_commands_before_reducing() {
        let model = model();
        assert!(matches!(
            model.apply_virtual_command(EditorVirtualCommand::PaintVisual {
                cells: Vec::new(),
                material: Some(CompositeTileId::new("base").unwrap()),
            }),
            Err(EditorVirtualCommandError::EmptyCells)
        ));
        assert!(matches!(
            model.apply_virtual_command(EditorVirtualCommand::PaintCollision {
                cells: vec![TilePosition::new(9, 9)],
                collision: Collision::Blocked,
            }),
            Err(EditorVirtualCommandError::CellOutOfBounds(_))
        ));
        assert!(matches!(
            model.apply_virtual_command(EditorVirtualCommand::SelectAtomic {
                tile: AtomicTileId::new("missing").unwrap(),
            }),
            Err(EditorVirtualCommandError::UnknownAtomicTile(_))
        ));
    }

    #[test]
    fn inspect_returns_a_read_only_agent_snapshot() {
        let model = model();
        let (unchanged, result) = model
            .execute_virtual_command(EditorVirtualCommand::Inspect)
            .unwrap();
        let EditorVirtualCommandResult::State(state) = result else {
            panic!("inspect must return state");
        };
        assert_eq!(unchanged.project, model.project);
        assert_eq!(state.project.id.as_str(), "test");
        assert_eq!(state.project.width, 3);
        assert_eq!(state.project.height, 2);
        assert_eq!(state.atomic_tiles.len(), 2);
        assert_eq!(state.selected_atomic.unwrap().as_str(), "grass");
        assert_eq!(state.selected_material.unwrap().as_str(), "base");
        assert!(!state.dirty);
    }
}
