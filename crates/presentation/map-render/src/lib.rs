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
#[path = "../tests/unit/lib.rs"]
mod tests;
