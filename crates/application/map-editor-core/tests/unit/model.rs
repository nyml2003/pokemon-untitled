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
