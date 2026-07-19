use map_project::{
    AtomicTileId, CellChange, Collision, CompositeTile, CompositeTileId, EditHistory, MapCell,
    MapEditCommand, MapError, MapEventKind, MapProject, TilePosition,
};
use map_tile_semantics::{MapSemanticDiagnostic, TileSemanticsCatalog};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorTool {
    Visual,
    Collision(Collision),
    Event(Option<MapEventKind>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorIntent {
    Paint { position: TilePosition, erase: bool },
    SelectAtomic(usize),
    SelectMaterial(usize),
    SelectTool(EditorTool),
    CreateMaterial(CompositeTile),
    AddLayer,
    RemoveLayer,
    DeleteMaterial,
    Undo,
    Redo,
    Save,
    ToggleHelp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorEffect {
    None,
    SaveRequested,
}

#[derive(Clone)]
pub struct EditorModel {
    pub project: MapProject,
    pub atomic_ids: Vec<AtomicTileId>,
    pub selected_atomic: usize,
    pub selected_material: usize,
    pub tool: EditorTool,
    pub dirty: bool,
    pub show_help: bool,
    pub status: String,
    semantics: Option<TileSemanticsCatalog>,
    history: EditHistory,
    next_material: u32,
}

impl EditorModel {
    pub fn new(project: MapProject, atomic_ids: Vec<AtomicTileId>) -> Self {
        let selected_material = usize::from(!project.materials.is_empty()).saturating_sub(1);
        let next_material = project.materials.len() as u32;
        Self {
            project,
            atomic_ids,
            selected_atomic: 0,
            selected_material,
            tool: EditorTool::Visual,
            dirty: false,
            show_help: false,
            status: "就绪".into(),
            semantics: None,
            history: EditHistory::default(),
            next_material,
        }
    }

    pub fn with_semantics(
        project: MapProject,
        atomic_ids: Vec<AtomicTileId>,
        semantics: TileSemanticsCatalog,
    ) -> Self {
        let mut model = Self::new(project, atomic_ids);
        model.semantics = Some(semantics);
        model
    }

    pub fn semantic_diagnostics(&self) -> Option<Vec<MapSemanticDiagnostic>> {
        self.semantics
            .as_ref()
            .map(|semantics| semantics.lint(&self.project))
    }

    pub fn reduce(&self, intent: EditorIntent) -> Result<(Self, EditorEffect), MapError> {
        let mut next = self.clone();
        let effect = next.apply_mut(intent)?;
        Ok((next, effect))
    }

    fn apply_mut(&mut self, intent: EditorIntent) -> Result<EditorEffect, MapError> {
        match intent {
            EditorIntent::Paint { position, erase } => self.paint(position, erase)?,
            EditorIntent::SelectAtomic(index) if index < self.atomic_ids.len() => {
                self.selected_atomic = index;
                self.status = format!("已选择原子素材 {}", self.atomic_ids[index]);
            }
            EditorIntent::SelectMaterial(index) if index < self.project.materials.len() => {
                self.selected_material = index;
                self.tool = EditorTool::Visual;
                self.status = format!("当前组合素材 {}", self.project.materials[index].id);
            }
            EditorIntent::SelectTool(tool) => {
                self.tool = tool;
                self.status = tool_name(tool).into();
            }
            EditorIntent::CreateMaterial(material) => self.create_material(material)?,
            EditorIntent::AddLayer => self.add_layer()?,
            EditorIntent::RemoveLayer => self.remove_layer()?,
            EditorIntent::DeleteMaterial => self.delete_material()?,
            EditorIntent::Undo => {
                let (project, history, changed) =
                    self.history.clone().undo(self.project.clone())?;
                self.project = project;
                self.history = history;
                self.status = if changed {
                    self.dirty = true;
                    "已撤销"
                } else {
                    "没有可撤销的操作"
                }
                .into();
            }
            EditorIntent::Redo => {
                let (project, history, changed) =
                    self.history.clone().redo(self.project.clone())?;
                self.project = project;
                self.history = history;
                self.status = if changed {
                    self.dirty = true;
                    "已重做"
                } else {
                    "没有可重做的操作"
                }
                .into();
            }
            EditorIntent::Save => return Ok(EditorEffect::SaveRequested),
            EditorIntent::ToggleHelp => {
                self.show_help = !self.show_help;
                self.status = if self.show_help {
                    "已打开使用说明"
                } else {
                    "已关闭使用说明"
                }
                .into();
            }
            EditorIntent::SelectAtomic(_) | EditorIntent::SelectMaterial(_) => {}
        }
        Ok(EditorEffect::None)
    }

    pub fn saved(&self) -> Self {
        let mut next = self.clone();
        next.dirty = false;
        next.status = "保存成功".into();
        next
    }

    pub fn with_error(&self, error: impl std::fmt::Display) -> Self {
        let mut next = self.clone();
        next.status = format!("错误：{error}");
        next
    }

    fn paint(&mut self, position: TilePosition, erase: bool) -> Result<(), MapError> {
        let Some(before) = self.project.cell(position) else {
            return Ok(());
        };
        let after = match self.tool {
            EditorTool::Visual => MapCell::new(
                if erase {
                    None
                } else {
                    self.project
                        .materials
                        .get(self.selected_material)
                        .map(|material| material.id.clone())
                },
                before.collision,
                before.event,
            ),
            EditorTool::Collision(collision) => MapCell::new(
                before.visual.material.clone(),
                if erase {
                    Collision::Walkable
                } else {
                    collision
                },
                before.event,
            ),
            EditorTool::Event(event) => MapCell::new(
                before.visual.material.clone(),
                before.collision,
                if erase { None } else { event },
            ),
        };
        if before == after {
            return Ok(());
        }
        self.execute(MapEditCommand::ReplaceCells(vec![CellChange {
            position,
            before,
            after,
        }]))?;
        self.dirty = true;
        self.status = format!("已编辑格子 {}, {}", position.x(), position.y());
        Ok(())
    }

    fn add_layer(&mut self) -> Result<(), MapError> {
        let Some(atomic) = self.atomic_ids.get(self.selected_atomic).cloned() else {
            return Ok(());
        };
        let mut layers = self
            .project
            .materials
            .get(self.selected_material)
            .map(|material| material.layers.clone())
            .unwrap_or_default();
        layers.push(atomic);
        self.create_composition(layers)
    }

    fn create_material(&mut self, material: CompositeTile) -> Result<(), MapError> {
        for tile in &material.layers {
            if !self.atomic_ids.contains(tile) {
                return Err(MapError::UnknownAtomicTile(tile.clone()));
            }
        }
        self.execute(MapEditCommand::CreateMaterial(material))?;
        self.selected_material = self.project.materials.len().saturating_sub(1);
        self.tool = EditorTool::Visual;
        self.dirty = true;
        self.status = "已创建新的组合素材".into();
        Ok(())
    }

    fn remove_layer(&mut self) -> Result<(), MapError> {
        let Some(mut layers) = self
            .project
            .materials
            .get(self.selected_material)
            .map(|material| material.layers.clone())
        else {
            return Ok(());
        };
        layers.pop();
        if layers.is_empty() {
            self.status = "组合素材至少需要一层".into();
            return Ok(());
        }
        self.create_composition(layers)
    }

    fn create_composition(&mut self, layers: Vec<AtomicTileId>) -> Result<(), MapError> {
        let id = loop {
            self.next_material = self.next_material.saturating_add(1);
            let id = CompositeTileId::new(format!("material-{:04}", self.next_material))?;
            if self.project.material(&id).is_none() {
                break id;
            }
        };
        self.create_material(CompositeTile::new(id, layers))
    }

    fn delete_material(&mut self) -> Result<(), MapError> {
        let Some(material) = self.project.materials.get(self.selected_material).cloned() else {
            self.status = "没有可删除的组合素材".into();
            return Ok(());
        };
        if self
            .project
            .visual_cells
            .iter()
            .any(|cell| cell.material.as_ref() == Some(&material.id))
        {
            self.status = "该组合素材仍被地图使用，请先改画或擦除对应格子".into();
            return Ok(());
        }
        self.execute(MapEditCommand::RemoveMaterial(material))?;
        self.selected_material = self
            .selected_material
            .min(self.project.materials.len().saturating_sub(1));
        self.tool = EditorTool::Visual;
        self.dirty = true;
        self.status = "已删除组合素材，可使用撤销恢复".into();
        Ok(())
    }

    fn execute(&mut self, command: MapEditCommand) -> Result<(), MapError> {
        let (project, history) = self
            .history
            .clone()
            .execute(self.project.clone(), command)?;
        self.project = project;
        self.history = history;
        Ok(())
    }
}

pub const fn tool_name(tool: EditorTool) -> &'static str {
    match tool {
        EditorTool::Visual => "贴图画笔",
        EditorTool::Collision(Collision::Walkable) => "可通行画笔",
        EditorTool::Collision(Collision::Blocked) => "阻挡画笔",
        EditorTool::Event(Some(MapEventKind::Encounter)) => "遭遇事件画笔",
        EditorTool::Event(None) => "清除事件画笔",
    }
}

#[cfg(test)]
mod tests {
    use map_project::{AtomicTileId, CompositeTile, CompositeTileId, MapProject, MapProjectId};

    use super::*;

    fn model() -> EditorModel {
        let tile = AtomicTileId::new("tile").unwrap();
        let project = MapProject::blank(
            MapProjectId::new("map").unwrap(),
            2,
            1,
            Some(CompositeTile::new(
                CompositeTileId::new("base").unwrap(),
                vec![tile.clone()],
            )),
        );
        EditorModel::new(project, vec![tile])
    }

    fn reduce(model: EditorModel, intent: EditorIntent) -> EditorModel {
        model.reduce(intent).unwrap().0
    }

    #[test]
    fn adding_a_layer_creates_a_new_composition_without_mutating_the_old_one() {
        let mut model = model();
        let original = model.project.materials[0].clone();
        model = reduce(model, EditorIntent::AddLayer);
        assert_eq!(model.project.materials[0], original);
        assert_eq!(model.project.materials[1].layers.len(), 2);
    }

    #[test]
    fn collision_paint_does_not_change_visual_material() {
        let mut model = model();
        let before = model.project.visual_cells[0].clone();
        model = reduce(
            model,
            EditorIntent::SelectTool(EditorTool::Collision(Collision::Blocked)),
        );
        model = reduce(
            model,
            EditorIntent::Paint {
                position: TilePosition::new(0, 0),
                erase: false,
            },
        );
        assert_eq!(model.project.visual_cells[0], before);
        assert_eq!(model.project.collision_cells[0], Collision::Blocked);
    }

    #[test]
    fn help_is_a_modelled_toggle_without_changing_the_map() {
        let mut model = model();
        let project = model.project.clone();
        model = reduce(model, EditorIntent::ToggleHelp);
        assert!(model.show_help);
        assert_eq!(model.project, project);
        model = reduce(model, EditorIntent::ToggleHelp);
        assert!(!model.show_help);
    }

    #[test]
    fn deletes_an_unused_material_and_undo_restores_it() {
        let mut model = model();
        model = reduce(model, EditorIntent::AddLayer);
        let deleted = model.project.materials[1].clone();

        model = reduce(model, EditorIntent::DeleteMaterial);
        assert_eq!(model.project.materials.len(), 1);
        assert_eq!(model.status, "已删除组合素材，可使用撤销恢复");

        model = reduce(model, EditorIntent::Undo);
        assert_eq!(model.project.materials.last(), Some(&deleted));
    }

    #[test]
    fn refuses_to_delete_a_material_used_by_the_map() {
        let mut model = model();

        model = reduce(model, EditorIntent::DeleteMaterial);

        assert_eq!(model.project.materials.len(), 1);
        assert_eq!(
            model.status,
            "该组合素材仍被地图使用，请先改画或擦除对应格子"
        );
    }

    #[test]
    fn every_public_intent_is_an_immutable_reduction() {
        let source = model();
        let (selected, effect) = source.reduce(EditorIntent::SelectAtomic(0)).unwrap();
        assert_eq!(effect, EditorEffect::None);
        assert_eq!(source.status, "就绪");
        assert!(selected.status.starts_with("已选择原子素材"));
        let (unchanged, _) = selected.reduce(EditorIntent::SelectAtomic(99)).unwrap();
        assert_eq!(unchanged.status, selected.status);

        let (selected, _) = unchanged.reduce(EditorIntent::SelectMaterial(0)).unwrap();
        assert!(selected.status.starts_with("当前组合素材"));
        let (unchanged, _) = selected.reduce(EditorIntent::SelectMaterial(99)).unwrap();
        assert_eq!(unchanged.selected_material, selected.selected_material);

        for (tool, name) in [
            (EditorTool::Visual, "贴图画笔"),
            (EditorTool::Collision(Collision::Walkable), "可通行画笔"),
            (EditorTool::Collision(Collision::Blocked), "阻挡画笔"),
            (
                EditorTool::Event(Some(MapEventKind::Encounter)),
                "遭遇事件画笔",
            ),
            (EditorTool::Event(None), "清除事件画笔"),
        ] {
            let (next, _) = unchanged.reduce(EditorIntent::SelectTool(tool)).unwrap();
            assert_eq!(next.status, name);
        }

        let (_, effect) = unchanged.reduce(EditorIntent::Save).unwrap();
        assert_eq!(effect, EditorEffect::SaveRequested);
        let failed = unchanged.with_error("broken");
        assert_eq!(failed.status, "错误：broken");
        let mut dirty = failed.clone();
        dirty.dirty = true;
        let saved = dirty.saved();
        assert!(!saved.dirty);
        assert_eq!(saved.status, "保存成功");
    }

    #[test]
    fn paint_reducer_covers_visual_collision_event_and_bounds() {
        let position = TilePosition::new(0, 0);
        let mut state = model();
        (state, _) = state
            .reduce(EditorIntent::Paint {
                position,
                erase: true,
            })
            .unwrap();
        assert_eq!(state.project.visual_cells[0].material, None);
        (state, _) = state
            .reduce(EditorIntent::Paint {
                position,
                erase: false,
            })
            .unwrap();
        assert!(state.project.visual_cells[0].material.is_some());

        (state, _) = state
            .reduce(EditorIntent::SelectTool(EditorTool::Collision(
                Collision::Blocked,
            )))
            .unwrap();
        (state, _) = state
            .reduce(EditorIntent::Paint {
                position,
                erase: false,
            })
            .unwrap();
        assert_eq!(state.project.collision_cells[0], Collision::Blocked);
        (state, _) = state
            .reduce(EditorIntent::Paint {
                position,
                erase: true,
            })
            .unwrap();
        assert_eq!(state.project.collision_cells[0], Collision::Walkable);

        (state, _) = state
            .reduce(EditorIntent::SelectTool(EditorTool::Event(Some(
                MapEventKind::Encounter,
            ))))
            .unwrap();
        (state, _) = state
            .reduce(EditorIntent::Paint {
                position,
                erase: false,
            })
            .unwrap();
        assert_eq!(state.project.event_cells[0], Some(MapEventKind::Encounter));
        (state, _) = state
            .reduce(EditorIntent::Paint {
                position,
                erase: true,
            })
            .unwrap();
        assert_eq!(state.project.event_cells[0], None);
        let (same, _) = state
            .reduce(EditorIntent::Paint {
                position: TilePosition::new(99, 99),
                erase: false,
            })
            .unwrap();
        assert_eq!(same.project, state.project);
    }

    #[test]
    fn composition_and_history_edge_states_are_explicit() {
        let state = model();
        let (state, _) = state.reduce(EditorIntent::Undo).unwrap();
        assert_eq!(state.status, "没有可撤销的操作");
        let (state, _) = state.reduce(EditorIntent::Redo).unwrap();
        assert_eq!(state.status, "没有可重做的操作");

        let (state, _) = state.reduce(EditorIntent::AddLayer).unwrap();
        let (state, _) = state.reduce(EditorIntent::Undo).unwrap();
        let (state, _) = state.reduce(EditorIntent::Redo).unwrap();
        assert_eq!(state.status, "已重做");
        let (state, _) = state.reduce(EditorIntent::RemoveLayer).unwrap();
        assert_eq!(state.project.materials.last().unwrap().layers.len(), 1);

        let mut no_atomic = model();
        no_atomic.atomic_ids.clear();
        let (no_atomic, _) = no_atomic.reduce(EditorIntent::AddLayer).unwrap();
        assert_eq!(no_atomic.status, "就绪");

        let one_layer = model();
        let (one_layer, _) = one_layer.reduce(EditorIntent::RemoveLayer).unwrap();
        assert_eq!(one_layer.status, "组合素材至少需要一层");

        let mut empty = model();
        empty.project.materials.clear();
        empty.project.visual_cells[0].material = None;
        let (empty, _) = empty.reduce(EditorIntent::RemoveLayer).unwrap();
        assert_eq!(empty.status, "就绪");
        let (empty, _) = empty.reduce(EditorIntent::DeleteMaterial).unwrap();
        assert_eq!(empty.status, "没有可删除的组合素材");
        let (empty, _) = empty
            .reduce(EditorIntent::Paint {
                position: TilePosition::new(0, 0),
                erase: false,
            })
            .unwrap();
        assert_eq!(empty.project.visual_cells[0].material, None);
    }
}
