use game_assets::AssetKey;
use map_project::{AtomicTileId, CompositeTile, CompositeTileId, MapProject, MapProjectId};
use map_render::AtomicTileAsset;

use super::*;

#[test]
fn projects_a_fixed_workbench_with_readable_labels() {
    let tile = AtomicTileId::new("tile-0001").unwrap();
    let model = EditorModel::new(
        MapProject::blank(
            MapProjectId::new("map").unwrap(),
            16,
            10,
            Some(CompositeTile::new(
                CompositeTileId::new("material-0000").unwrap(),
                vec![tile.clone()],
            )),
        ),
        vec![tile.clone()],
    );
    let catalog = AtomicTileCatalog::new([AtomicTileAsset {
        id: tile,
        asset: AssetKey::new("map/tile/tile-0001").unwrap(),
    }])
    .unwrap();
    let frame = project(
        &model,
        &catalog,
        None,
        PixelSize::new(1280, 720),
        EditorMapViewport::default(),
    )
    .unwrap();
    assert_eq!(frame.map.layers().len(), 3);
    assert!(frame.map.layers()[2].surface.is_none());
    let chrome = frame
        .chrome
        .resolve(punctum_ui::UiSize::new(1280, 720))
        .unwrap();
    assert!(chrome.commands().iter().any(|command| matches!(
        command, punctum_ui::UiDrawCommand::Text { content, .. } if content == "原子素材"
    )));
    assert!(chrome.commands().iter().any(|command| matches!(
        command, punctum_ui::UiDrawCommand::Text { content, .. } if content.starts_with("组合素材")
    )));
    assert!(chrome.action_hits().len() >= 10);
}

#[test]
fn centered_viewport_tracks_the_map_center_at_each_zoom_level() {
    let tile = AtomicTileId::new("tile-0001").unwrap();
    let model = EditorModel::new(
        MapProject::blank(
            MapProjectId::new("map").unwrap(),
            72,
            56,
            Some(CompositeTile::new(
                CompositeTileId::new("material-0000").unwrap(),
                vec![tile.clone()],
            )),
        ),
        vec![tile],
    );
    assert_eq!(
        centered_map_viewport(&model, 1),
        EditorMapViewport::new(1, 12, 12)
    );
    assert_eq!(
        centered_map_viewport(&model, 2),
        EditorMapViewport::new(2, 24, 20)
    );
}

#[test]
fn help_is_an_explicit_layer_and_does_not_steal_hud_labels() {
    let tile = AtomicTileId::new("tile-0001").unwrap();
    let mut model = EditorModel::new(
        MapProject::blank(
            MapProjectId::new("map").unwrap(),
            16,
            10,
            Some(CompositeTile::new(
                CompositeTileId::new("material-0000").unwrap(),
                vec![tile.clone()],
            )),
        ),
        vec![tile.clone()],
    );
    model.show_help = true;
    let catalog = AtomicTileCatalog::new([AtomicTileAsset {
        id: tile,
        asset: AssetKey::new("map/tile/tile-0001").unwrap(),
    }])
    .unwrap();

    let frame = project(
        &model,
        &catalog,
        Some(TilePosition::new(1, 2)),
        PixelSize::new(1280, 720),
        EditorMapViewport::default(),
    )
    .unwrap();

    assert_eq!(frame.map.layers().len(), 3);
    let chrome = frame
        .chrome
        .resolve(punctum_ui::UiSize::new(1280, 720))
        .unwrap();
    assert!(chrome.commands().iter().any(|command| matches!(
        command, punctum_ui::UiDrawCommand::Text { content, .. } if content == "地图编辑器使用说明"
    )));
    assert!(chrome.action_hits().len() >= 11);
}

#[test]
fn projects_every_tool_page_and_error_state() {
    let tile = AtomicTileId::new("tile-0001").unwrap();
    let mut model = EditorModel::new(
        MapProject::blank(
            MapProjectId::new("map").unwrap(),
            16,
            10,
            Some(CompositeTile::new(
                CompositeTileId::new("material-0000").unwrap(),
                vec![tile.clone()],
            )),
        ),
        (0..25)
            .map(|index| AtomicTileId::new(format!("tile-{index:04}")).unwrap())
            .collect(),
    );
    let catalog = AtomicTileCatalog::new([AtomicTileAsset {
        id: tile.clone(),
        asset: AssetKey::new("map/tile/tile-0001").unwrap(),
    }])
    .unwrap();
    model.project.collision_cells[1] = Collision::Blocked;
    model.project.event_cells[0] = Some(MapEventKind::Encounter);
    model.selected_atomic = 24;
    for index in 1..9 {
        model.project.materials.push(CompositeTile::new(
            CompositeTileId::new(format!("material-{index:04}")).unwrap(),
            vec![tile.clone()],
        ));
    }
    model.selected_material = 8;
    model.status = "错误：fixture".into();

    for tool in [
        EditorTool::Collision(Collision::Walkable),
        EditorTool::Collision(Collision::Blocked),
        EditorTool::Event(Some(MapEventKind::Encounter)),
        EditorTool::Event(None),
    ] {
        model.tool = tool;
        let frame = project(
            &model,
            &catalog,
            None,
            PixelSize::new(1280, 720),
            EditorMapViewport::default(),
        )
        .unwrap();
        assert_eq!(frame.map.layers().len(), 3);
        assert!(
            frame.map.layers()[2]
                .images
                .iter()
                .any(|image| image.z_index == 7)
        );
        let chrome = frame
            .chrome
            .resolve(punctum_ui::UiSize::new(1280, 720))
            .unwrap();
        assert!(chrome.commands().iter().any(|command| matches!(
            command, punctum_ui::UiDrawCommand::Text { content, .. } if content == "错误：fixture"
        )));
    }

    let missing = AtomicTileCatalog::new([]).unwrap();
    let error = project(
        &model,
        &missing,
        None,
        PixelSize::new(1280, 720),
        EditorMapViewport::default(),
    )
    .unwrap_err();
    assert!(error.to_string().starts_with("map projection failed:"));
}
