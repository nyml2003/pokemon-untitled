//! Shared map projection used by both the game and the editor.

#![forbid(unsafe_code)]

use std::{collections::BTreeMap, error::Error, fmt};

use game_assets::AssetKey;
use game_view::{LayerKind, ViewCell, ViewImage, ViewLayer};
use map_project::{AtomicTileId, MapProject};
use punctum_gpu::{PixelOffset, Rgba8, Viewport};
use punctum_grid::{GridPos, GridRect, GridSize, Surface, SurfaceError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtomicTileAsset {
    pub id: AtomicTileId,
    pub asset: AssetKey,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AtomicTileCatalog {
    tiles: BTreeMap<AtomicTileId, AssetKey>,
}

impl AtomicTileCatalog {
    pub fn new(tiles: impl IntoIterator<Item = AtomicTileAsset>) -> Result<Self, MapRenderError> {
        let mut resources = BTreeMap::new();
        for tile in tiles {
            if resources.insert(tile.id.clone(), tile.asset).is_some() {
                return Err(MapRenderError::DuplicateAtomicTile(tile.id));
            }
        }
        Ok(Self { tiles: resources })
    }

    pub fn asset(&self, id: &AtomicTileId) -> Option<&AssetKey> {
        self.tiles.get(id)
    }

    pub fn ids(&self) -> impl Iterator<Item = &AtomicTileId> {
        self.tiles.keys()
    }

    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MapCamera {
    pub col: i32,
    pub row: i32,
}

impl MapCamera {
    pub const fn new(col: i32, row: i32) -> Self {
        Self { col, row }
    }
}

pub struct MapRenderInput<'a> {
    pub project: &'a MapProject,
    pub catalog: &'a AtomicTileCatalog,
    pub camera: MapCamera,
    pub pixel_offset: PixelOffset,
    pub viewport: Viewport,
    pub layout: MapGridLayout,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapGridLayout {
    pub surface_size: GridSize,
    pub tile_span: GridSize,
}

impl MapGridLayout {
    pub const fn native(project: &MapProject) -> Self {
        Self {
            surface_size: GridSize::new(project.width as u32, project.height as u32),
            tile_span: GridSize::new(1, 1),
        }
    }

    pub const fn new(surface_size: GridSize, tile_span: GridSize) -> Self {
        Self {
            surface_size,
            tile_span,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapScenePlan {
    layer: ViewLayer,
    viewport: Viewport,
}

impl MapScenePlan {
    pub fn layer(&self) -> &ViewLayer {
        &self.layer
    }

    pub fn tile_images(&self) -> &[ViewImage] {
        &self.layer.images
    }

    pub fn into_layer(self) -> ViewLayer {
        self.layer
    }

    pub const fn viewport(&self) -> Viewport {
        self.viewport
    }
}

pub fn project_map(input: MapRenderInput<'_>) -> Result<MapScenePlan, MapRenderError> {
    if input.layout.tile_span.is_empty() {
        return Err(MapRenderError::EmptyTileSpan);
    }
    if input.layout.surface_size.cols > i32::MAX as u32
        || input.layout.surface_size.rows > i32::MAX as u32
    {
        return Err(MapRenderError::LayoutOverflow);
    }
    let base = Surface::filled(input.layout.surface_size, ViewCell::Empty)
        .map_err(MapRenderError::Surface)?;
    let mut tile_images = Vec::new();
    if input.layout.tile_span.cols > input.layout.surface_size.cols
        || input.layout.tile_span.rows > input.layout.surface_size.rows
    {
        return Ok(MapScenePlan {
            layer: ViewLayer::new(LayerKind::Map)
                .with_surface(base)
                .with_images(tile_images),
            viewport: input.viewport,
        });
    }

    let (visible_cols, visible_rows) = visible_tile_ranges(&input);
    for row in visible_rows {
        for col in visible_cols.clone() {
            let target_col = (i64::from(col) - i64::from(input.camera.col))
                * i64::from(input.layout.tile_span.cols);
            let target_row = (i64::from(row) - i64::from(input.camera.row))
                * i64::from(input.layout.tile_span.rows);
            if !tile_intersects_viewport(target_col, target_row, &input) {
                continue;
            }
            let (image_origin, image_offset) =
                anchored_tile_placement(target_col, target_row, &input)?;
            let index = usize::from(row) * usize::from(input.project.width) + usize::from(col);
            let Some(material_id) = &input.project.visual_cells[index].material else {
                continue;
            };
            let material = input
                .project
                .material(material_id)
                .ok_or_else(|| MapRenderError::UnknownMaterial(material_id.clone()))?;
            for atomic_id in &material.layers {
                let asset = input
                    .catalog
                    .asset(atomic_id)
                    .ok_or_else(|| MapRenderError::UnknownAtomicTile(atomic_id.clone()))?;
                tile_images.push(
                    ViewImage::new(
                        GridRect::new(image_origin, input.layout.tile_span),
                        asset.clone(),
                        Rgba8::new(255, 255, 255, 255),
                        0,
                    )
                    .with_pixel_offset(image_offset),
                );
            }
        }
    }

    Ok(MapScenePlan {
        layer: ViewLayer::new(LayerKind::Map)
            .with_surface(base)
            .with_images(tile_images),
        viewport: input.viewport,
    })
}

fn visible_tile_ranges(input: &MapRenderInput<'_>) -> (std::ops::Range<u16>, std::ops::Range<u16>) {
    let tile_pixel_width =
        u64::from(input.layout.tile_span.cols) * u64::from(input.viewport.cell_size.width);
    let tile_pixel_height =
        u64::from(input.layout.tile_span.rows) * u64::from(input.viewport.cell_size.height);
    let overscan_cols = pixel_overscan(input.pixel_offset.x, tile_pixel_width);
    let overscan_rows = pixel_overscan(input.pixel_offset.y, tile_pixel_height);
    let visible_cols = input.layout.surface_size.cols / input.layout.tile_span.cols;
    let visible_rows = input.layout.surface_size.rows / input.layout.tile_span.rows;
    (
        bounded_tile_range(
            input.camera.col,
            visible_cols,
            overscan_cols,
            input.project.width,
        ),
        bounded_tile_range(
            input.camera.row,
            visible_rows,
            overscan_rows,
            input.project.height,
        ),
    )
}

fn pixel_overscan(offset: i32, tile_pixel_extent: u64) -> u32 {
    if offset == 0 || tile_pixel_extent == 0 {
        return 0;
    }
    let magnitude = u64::from(offset.unsigned_abs());
    magnitude
        .div_ceil(tile_pixel_extent)
        .min(u64::from(u32::MAX)) as u32
}

fn bounded_tile_range(
    camera: i32,
    visible: u32,
    overscan: u32,
    extent: u16,
) -> std::ops::Range<u16> {
    let start = i64::from(camera) - i64::from(overscan);
    let end = i64::from(camera) + i64::from(visible) + i64::from(overscan);
    let extent = i64::from(extent);
    start.clamp(0, extent) as u16..end.clamp(0, extent) as u16
}

fn tile_intersects_viewport(target_col: i64, target_row: i64, input: &MapRenderInput<'_>) -> bool {
    let cell_width = i64::from(input.viewport.cell_size.width);
    let cell_height = i64::from(input.viewport.cell_size.height);
    let left = target_col * cell_width + i64::from(input.pixel_offset.x);
    let top = target_row * cell_height + i64::from(input.pixel_offset.y);
    let right = left + i64::from(input.layout.tile_span.cols) * cell_width;
    let bottom = top + i64::from(input.layout.tile_span.rows) * cell_height;
    let viewport_width = i64::from(input.layout.surface_size.cols) * cell_width;
    let viewport_height = i64::from(input.layout.surface_size.rows) * cell_height;
    right > 0 && bottom > 0 && left < viewport_width && top < viewport_height
}

fn anchored_tile_placement(
    target_col: i64,
    target_row: i64,
    input: &MapRenderInput<'_>,
) -> Result<(GridPos, PixelOffset), MapRenderError> {
    let maximum_col = i64::from(input.layout.surface_size.cols - input.layout.tile_span.cols);
    let maximum_row = i64::from(input.layout.surface_size.rows - input.layout.tile_span.rows);
    let anchored_col = target_col.clamp(0, maximum_col);
    let anchored_row = target_row.clamp(0, maximum_row);
    let offset_x = i64::from(input.pixel_offset.x)
        + (target_col - anchored_col) * i64::from(input.viewport.cell_size.width);
    let offset_y = i64::from(input.pixel_offset.y)
        + (target_row - anchored_row) * i64::from(input.viewport.cell_size.height);
    Ok((
        GridPos::new(
            i32::try_from(anchored_col).map_err(|_| MapRenderError::LayoutOverflow)?,
            i32::try_from(anchored_row).map_err(|_| MapRenderError::LayoutOverflow)?,
        ),
        PixelOffset::new(
            i32::try_from(offset_x).map_err(|_| MapRenderError::LayoutOverflow)?,
            i32::try_from(offset_y).map_err(|_| MapRenderError::LayoutOverflow)?,
        ),
    ))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapRenderError {
    DuplicateAtomicTile(AtomicTileId),
    UnknownAtomicTile(AtomicTileId),
    UnknownMaterial(map_project::CompositeTileId),
    EmptyTileSpan,
    LayoutOverflow,
    Surface(SurfaceError),
}

impl fmt::Display for MapRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateAtomicTile(id) => write!(formatter, "atomic tile {id} is defined twice"),
            Self::UnknownAtomicTile(id) => {
                write!(formatter, "atomic tile {id} has no atlas resource")
            }
            Self::UnknownMaterial(id) => write!(formatter, "material {id} is not defined"),
            Self::EmptyTileSpan => formatter.write_str("map tile span must be non-empty"),
            Self::LayoutOverflow => formatter.write_str("scaled map dimensions overflow"),
            Self::Surface(error) => write!(formatter, "cannot allocate map surface: {error}"),
        }
    }
}

impl Error for MapRenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Surface(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
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
}
