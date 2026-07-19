use game_assets::AssetKey;
use map_project::{AtomicTileId, CompositeTile, CompositeTileId, MapProject, MapProjectId};
use punctum_gpu::{PixelOffset, PixelSize, Viewport};

use super::*;

fn asset(name: &str) -> AssetKey {
    AssetKey::new(format!("test/{name}")).unwrap()
}

#[test]
fn expands_every_composite_layer_in_stable_cell_order() {
    let first = AtomicTileId::new("first").unwrap();
    let second = AtomicTileId::new("second").unwrap();
    let material = CompositeTile::new(
        CompositeTileId::new("stack").unwrap(),
        vec![first.clone(), second.clone(), first.clone()],
    );
    let project = MapProject::blank(MapProjectId::new("map").unwrap(), 2, 1, Some(material));
    let catalog = AtomicTileCatalog::new([
        AtomicTileAsset {
            id: first,
            asset: asset("first"),
        },
        AtomicTileAsset {
            id: second,
            asset: asset("second"),
        },
    ])
    .unwrap();
    let viewport = Viewport::new(
        PixelSize::new(320, 160),
        PixelOffset::new(0, 0),
        PixelSize::new(16, 16),
    )
    .unwrap();

    let scene = project_map(MapRenderInput {
        project: &project,
        catalog: &catalog,
        camera: MapCamera::default(),
        pixel_offset: PixelOffset::new(0, 0),
        viewport,
        layout: MapGridLayout::native(&project),
    })
    .unwrap();

    let assets = scene
        .tile_images()
        .iter()
        .map(|image| image.asset.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        assets,
        [
            asset("first"),
            asset("second"),
            asset("first"),
            asset("first"),
            asset("second"),
            asset("first")
        ]
    );
}

#[test]
fn camera_projects_only_visible_map_cells_without_moving_the_ui_viewport() {
    let tile = AtomicTileId::new("tile").unwrap();
    let project = MapProject::blank(
        MapProjectId::new("map").unwrap(),
        3,
        4,
        Some(CompositeTile::new(
            CompositeTileId::new("base").unwrap(),
            vec![tile.clone()],
        )),
    );
    let catalog = AtomicTileCatalog::new([AtomicTileAsset {
        id: tile,
        asset: asset("tile"),
    }])
    .unwrap();
    let viewport = Viewport::new(
        PixelSize::new(100, 100),
        PixelOffset::new(8, 9),
        PixelSize::new(16, 16),
    )
    .unwrap();
    let scene = project_map(MapRenderInput {
        project: &project,
        catalog: &catalog,
        camera: MapCamera::new(2, 3),
        pixel_offset: PixelOffset::new(0, 0),
        viewport,
        layout: MapGridLayout::native(&project),
    })
    .unwrap();
    assert_eq!(scene.viewport(), viewport);
    assert_eq!(scene.tile_images().len(), 1);
    assert_eq!(scene.tile_images()[0].bounds.origin, GridPos::new(0, 0));
}

#[test]
fn moving_camera_keeps_offscreen_tiles_on_valid_grid_anchors() {
    let tile = AtomicTileId::new("tile").unwrap();
    let project = MapProject::blank(
        MapProjectId::new("map").unwrap(),
        3,
        3,
        Some(CompositeTile::new(
            CompositeTileId::new("base").unwrap(),
            vec![tile.clone()],
        )),
    );
    let catalog = AtomicTileCatalog::new([AtomicTileAsset {
        id: tile,
        asset: asset("tile"),
    }])
    .unwrap();
    let viewport = Viewport::new(
        PixelSize::new(32, 32),
        PixelOffset::new(0, 0),
        PixelSize::new(16, 16),
    )
    .unwrap();

    let scene = project_map(MapRenderInput {
        project: &project,
        catalog: &catalog,
        camera: MapCamera::new(1, 1),
        pixel_offset: PixelOffset::new(8, 8),
        viewport,
        layout: MapGridLayout::new(GridSize::new(2, 2), GridSize::new(1, 1)),
    })
    .unwrap();

    assert_eq!(scene.tile_images().len(), 9);
    assert_eq!(scene.tile_images()[0].bounds.origin, GridPos::new(0, 0));
    assert_eq!(
        scene.tile_images()[0].pixel_offset,
        PixelOffset::new(-8, -8)
    );
    assert!(scene.tile_images().iter().all(|image| {
        image.bounds.origin.col >= 0
            && image.bounds.origin.row >= 0
            && image.bounds.origin.col + image.bounds.size.cols as i32 <= 2
            && image.bounds.origin.row + image.bounds.size.rows as i32 <= 2
    }));
}

#[test]
fn collision_and_events_do_not_affect_visual_projection() {
    let tile = AtomicTileId::new("tile").unwrap();
    let mut project = MapProject::blank(
        MapProjectId::new("map").unwrap(),
        1,
        1,
        Some(CompositeTile::new(
            CompositeTileId::new("base").unwrap(),
            vec![tile.clone()],
        )),
    );
    let catalog = AtomicTileCatalog::new([AtomicTileAsset {
        id: tile,
        asset: asset("tile"),
    }])
    .unwrap();
    let viewport = Viewport::new(
        PixelSize::new(16, 16),
        PixelOffset::new(0, 0),
        PixelSize::new(16, 16),
    )
    .unwrap();
    let render = |project: &MapProject| {
        project_map(MapRenderInput {
            project,
            catalog: &catalog,
            camera: MapCamera::default(),
            pixel_offset: PixelOffset::new(0, 0),
            viewport,
            layout: MapGridLayout::native(project),
        })
        .unwrap()
    };
    let before = render(&project);
    project.collision_cells[0] = map_project::Collision::Blocked;
    project.event_cells[0] = Some(map_project::MapEventKind::Encounter);
    let after = render(&project);
    assert_eq!(before, after);
}

#[test]
fn catalog_and_planner_failures_are_explicit() {
    let tile = AtomicTileId::new("tile").unwrap();
    let duplicate = AtomicTileCatalog::new([
        AtomicTileAsset {
            id: tile.clone(),
            asset: asset("one"),
        },
        AtomicTileAsset {
            id: tile.clone(),
            asset: asset("two"),
        },
    ]);
    assert!(matches!(
        duplicate,
        Err(MapRenderError::DuplicateAtomicTile(_))
    ));

    let catalog = AtomicTileCatalog::new([AtomicTileAsset {
        id: tile.clone(),
        asset: asset("tile"),
    }])
    .unwrap();
    assert_eq!(catalog.len(), 1);
    assert!(!catalog.is_empty());
    assert_eq!(catalog.ids().next(), Some(&tile));
    assert_eq!(catalog.asset(&tile), Some(&asset("tile")));

    let project = MapProject::blank(MapProjectId::new("map").unwrap(), 1, 1, None);
    let viewport = Viewport::new(
        PixelSize::new(16, 16),
        PixelOffset::new(0, 0),
        PixelSize::new(16, 16),
    )
    .unwrap();
    let render = |layout| {
        project_map(MapRenderInput {
            project: &project,
            catalog: &catalog,
            camera: MapCamera::default(),
            pixel_offset: PixelOffset::new(0, 0),
            viewport,
            layout,
        })
    };
    assert!(matches!(
        render(MapGridLayout::new(GridSize::new(1, 1), GridSize::new(0, 1))),
        Err(MapRenderError::EmptyTileSpan)
    ));
    assert!(matches!(
        render(MapGridLayout::new(
            GridSize::new(i32::MAX as u32 + 1, 1),
            GridSize::new(1, 1)
        )),
        Err(MapRenderError::LayoutOverflow)
    ));
    let empty = render(MapGridLayout::new(GridSize::new(1, 1), GridSize::new(2, 2))).unwrap();
    assert!(empty.tile_images().is_empty());
    assert_eq!(empty.layer().kind, LayerKind::Map);
    assert_eq!(empty.clone().into_layer(), empty.layer().clone());
    let shifted = project_map(MapRenderInput {
        project: &project,
        catalog: &catalog,
        camera: MapCamera::default(),
        pixel_offset: PixelOffset::new(1_000, 0),
        viewport,
        layout: MapGridLayout::new(GridSize::new(1, 1), GridSize::new(1, 1)),
    })
    .unwrap();
    assert!(shifted.tile_images().is_empty());

    for error in [
        MapRenderError::DuplicateAtomicTile(tile.clone()),
        MapRenderError::UnknownAtomicTile(tile.clone()),
        MapRenderError::UnknownMaterial(CompositeTileId::new("missing").unwrap()),
        MapRenderError::EmptyTileSpan,
        MapRenderError::LayoutOverflow,
        MapRenderError::Surface(SurfaceError::CapacityOverflow {
            size: GridSize::new(u32::MAX, u32::MAX),
        }),
    ] {
        assert!(!error.to_string().is_empty());
        assert_eq!(
            error.source().is_some(),
            matches!(error, MapRenderError::Surface(_))
        );
    }
}

#[test]
fn missing_material_asset_and_offscreen_cells_take_distinct_paths() {
    let tile = AtomicTileId::new("tile").unwrap();
    let missing = AtomicTileId::new("missing").unwrap();
    let material_id = CompositeTileId::new("base").unwrap();
    let mut project = MapProject::blank(
        MapProjectId::new("map").unwrap(),
        2,
        1,
        Some(CompositeTile::new(
            material_id.clone(),
            vec![missing.clone()],
        )),
    );
    let catalog = AtomicTileCatalog::new([AtomicTileAsset {
        id: tile,
        asset: asset("tile"),
    }])
    .unwrap();
    let viewport = Viewport::new(
        PixelSize::new(16, 16),
        PixelOffset::new(0, 0),
        PixelSize::new(16, 16),
    )
    .unwrap();
    let render = |project: &MapProject, camera| {
        project_map(MapRenderInput {
            project,
            catalog: &catalog,
            camera,
            pixel_offset: PixelOffset::new(0, 0),
            viewport,
            layout: MapGridLayout::new(GridSize::new(1, 1), GridSize::new(1, 1)),
        })
    };
    assert!(matches!(
        render(&project, MapCamera::default()),
        Err(MapRenderError::UnknownAtomicTile(id)) if id == missing
    ));

    project.materials.clear();
    assert!(matches!(
        render(&project, MapCamera::default()),
        Err(MapRenderError::UnknownMaterial(id)) if id == material_id
    ));
    for cell in &mut project.visual_cells {
        cell.material = None;
    }
    assert!(
        render(&project, MapCamera::new(1, 0))
            .unwrap()
            .tile_images()
            .is_empty()
    );
}
