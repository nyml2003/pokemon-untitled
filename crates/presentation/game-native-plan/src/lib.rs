//! Pure native asset and frame planning.

#![forbid(unsafe_code)]

use std::{collections::BTreeMap, error::Error, fmt};

use game_assets::{AssetKey, DecodedImage};
use game_view::{GameView, LayerKind, ViewCell};
use punctum_gpu::{
    GpuAtlas, GpuCell, GpuClip, GpuImage, GpuPixelImage, GpuPlanError, PixelOffset, PixelRect,
    PixelSize, ResourceId, Rgba8, SubmissionPlan, Viewport as GridViewport, plan_composite,
    plan_pixels,
};
use punctum_grid::{GridSize, Surface};
use punctum_ui::{UiDrawCommand, UiFrame};

pub struct NativeAssets {
    atlas: GpuAtlas,
    resources: BTreeMap<AssetKey, ResourceId>,
}

impl NativeAssets {
    pub fn new(images: Vec<(AssetKey, DecodedImage)>) -> Result<Self, NativeAssetError> {
        let mut resources = BTreeMap::new();
        let mut numbered = Vec::with_capacity(images.len());
        for (index, (key, image)) in images.iter().enumerate() {
            let id = resource_id(index)?;
            if resources.insert(key.clone(), id).is_some() {
                return Err(NativeAssetError::DuplicateKey(key.clone()));
            }
            numbered.push((id, image));
        }
        let atlas = game_assets::build_atlas(&numbered)
            .map_err(|error| NativeAssetError::Atlas(error.to_string()))?;
        Ok(Self { atlas, resources })
    }

    pub fn resource(&self, key: &AssetKey) -> Option<ResourceId> {
        self.resources.get(key).copied()
    }

    pub const fn atlas_size(&self) -> PixelSize {
        self.atlas.size()
    }

    pub const fn atlas(&self) -> &GpuAtlas {
        &self.atlas
    }
}

fn resource_id(index: usize) -> Result<ResourceId, NativeAssetError> {
    Ok(ResourceId(
        u32::try_from(index)
            .map_err(|_| NativeAssetError::TooManyAssets)?
            .checked_add(1)
            .ok_or(NativeAssetError::TooManyAssets)?,
    ))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeAssetError {
    DuplicateKey(AssetKey),
    TooManyAssets,
    Atlas(String),
}

impl fmt::Display for NativeAssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateKey(key) => write!(formatter, "duplicate asset key {}", key.as_str()),
            Self::TooManyAssets => formatter.write_str("native asset count exceeds u32"),
            Self::Atlas(message) => write!(formatter, "cannot build native atlas: {message}"),
        }
    }
}

impl Error for NativeAssetError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeTextLabel {
    pub col: u32,
    pub row: u32,
    pub width: u32,
    pub height: u32,
    pub content: String,
    pub color: Rgba8,
    /// Pixel UI supplies this value; Grid labels derive it from `TextScale`.
    pub font_size: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeTextBounds {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl NativeTextBounds {
    pub fn width(self) -> i32 {
        self.right.saturating_sub(self.left)
    }

    pub fn height(self) -> i32 {
        self.bottom.saturating_sub(self.top)
    }
}

pub fn text_bounds(
    label: &NativeTextLabel,
    viewport: GridViewport,
) -> Result<NativeTextBounds, std::num::TryFromIntError> {
    let left =
        i64::from(viewport.origin.x) + i64::from(label.col) * i64::from(viewport.cell_size.width);
    let top =
        i64::from(viewport.origin.y) + i64::from(label.row) * i64::from(viewport.cell_size.height);
    let right = left + i64::from(label.width) * i64::from(viewport.cell_size.width);
    let bottom = top + i64::from(label.height) * i64::from(viewport.cell_size.height);
    Ok(NativeTextBounds {
        left: i32::try_from(left)?,
        top: i32::try_from(top)?,
        right: i32::try_from(right)?,
        bottom: i32::try_from(bottom)?,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextScale {
    numerator: u32,
    denominator: u32,
    minimum: u32,
    maximum: u32,
}

impl TextScale {
    pub const fn new(numerator: u32, denominator: u32, minimum: u32, maximum: u32) -> Self {
        assert!(denominator > 0);
        assert!(minimum <= maximum);
        Self {
            numerator,
            denominator,
            minimum,
            maximum,
        }
    }

    pub fn font_size(self, cell_height: u32) -> f32 {
        (cell_height * self.numerator / self.denominator).clamp(self.minimum, self.maximum) as f32
    }
}

pub struct FramePass {
    gpu: SubmissionPlan,
    labels: Vec<NativeTextLabel>,
    text_scale: TextScale,
}

impl FramePass {
    pub fn viewport(&self) -> GridViewport {
        self.gpu.viewport
    }

    pub fn gpu(&self) -> &SubmissionPlan {
        &self.gpu
    }

    pub fn labels(&self) -> &[NativeTextLabel] {
        &self.labels
    }

    pub fn text_scale(&self) -> TextScale {
        self.text_scale
    }
}

pub struct FramePlan {
    passes: Vec<FramePass>,
}

impl FramePlan {
    /// Converts an already-resolved pixel UI frame at the adapter boundary.
    /// The UI crate stays independent from atlas IDs and GPU plans.
    pub fn from_ui_frame<Action>(
        frame: &UiFrame<Action>,
        assets: &NativeAssets,
        text_scale: TextScale,
    ) -> Result<Self, FramePlanError> {
        let white_key = AssetKey::new("solid/white").expect("the white asset key is valid");
        let white = assets
            .resource(&white_key)
            .ok_or(FramePlanError::UnknownAsset(white_key))?;
        let mut images = Vec::new();
        let mut labels = Vec::new();
        for (z_index, command) in frame.commands().iter().enumerate() {
            match command {
                UiDrawCommand::Fill {
                    bounds,
                    color,
                    border_radius,
                    clip,
                } => {
                    if let Some(bounds) = ui_visible_bounds(*bounds, *clip) {
                        images.push(
                            GpuPixelImage::new(
                                bounds,
                                white,
                                Rgba8::new(color.red, color.green, color.blue, color.alpha),
                                z_index as i32,
                            )
                            .with_corner_radii(ui_corner_radii(*border_radius, bounds)),
                        );
                    }
                }
                UiDrawCommand::Image {
                    bounds,
                    content,
                    tint,
                    pixel_offset,
                    border_radius,
                    clip,
                    ..
                } => {
                    if let Some(bounds) = ui_visible_bounds(*bounds, *clip) {
                        let key = AssetKey::new(content.as_str()).map_err(|_| {
                            FramePlanError::InvalidUiContent(content.as_str().to_owned())
                        })?;
                        let resource = assets
                            .resource(&key)
                            .ok_or(FramePlanError::UnknownAsset(key))?;
                        images.push(
                            GpuPixelImage::new(
                                bounds,
                                resource,
                                Rgba8::new(tint.red, tint.green, tint.blue, tint.alpha),
                                z_index as i32,
                            )
                            .with_pixel_offset(PixelOffset::new(pixel_offset.x, pixel_offset.y))
                            .with_corner_radii(ui_corner_radii(*border_radius, bounds)),
                        );
                    }
                }
                UiDrawCommand::Text {
                    bounds,
                    content,
                    color,
                    font_size,
                    clip,
                    ..
                } => {
                    if let Some(bounds) = ui_visible_bounds(*bounds, *clip) {
                        labels.push(NativeTextLabel {
                            col: bounds.x,
                            row: bounds.y,
                            width: bounds.width,
                            height: bounds.height,
                            content: content.clone(),
                            color: Rgba8::new(color.red, color.green, color.blue, color.alpha),
                            font_size: Some(*font_size),
                        });
                    }
                }
            }
        }
        Ok(Self::single(
            plan_pixels(
                &images,
                &assets.atlas,
                u32::MAX,
                PixelSize::new(frame.viewport().width, frame.viewport().height),
            )
            .map_err(FramePlanError::Gpu)?,
            labels,
            text_scale,
        ))
    }

    pub fn from_game_view(
        view: &GameView,
        assets: &NativeAssets,
        viewport: GridViewport,
        text_scale: TextScale,
    ) -> Result<Self, FramePlanError> {
        let size = view
            .layers()
            .iter()
            .find_map(|layer| layer.surface.as_ref().map(Surface::size))
            .ok_or(FramePlanError::MissingSurface)?;
        let mut cells = vec![GpuCell::Empty; (size.cols * size.rows) as usize];
        let white_key = AssetKey::new("solid/white").expect("the white asset key is valid");
        let white = assets
            .resource(&white_key)
            .ok_or(FramePlanError::UnknownAsset(white_key))?;
        let mut images = Vec::new();
        let mut labels = Vec::new();
        for layer in view.layers() {
            if let Some(surface) = &layer.surface {
                if surface.size() != size {
                    return Err(FramePlanError::SurfaceSizeMismatch {
                        expected: size,
                        actual: surface.size(),
                    });
                }
                for (target, source) in cells.iter_mut().zip(surface.cells()) {
                    if let ViewCell::Fill(tint) = source {
                        *target = GpuCell::Sprite {
                            resource: white,
                            tint: *tint,
                        };
                    }
                }
            }
            let layer_z = match layer.kind {
                LayerKind::Map => 0,
                LayerKind::Character => 100,
                LayerKind::Hud => 200,
                LayerKind::Console => 300,
            };
            for image in &layer.images {
                let resource = assets
                    .resource(&image.asset)
                    .ok_or_else(|| FramePlanError::UnknownAsset(image.asset.clone()))?;
                images.push(
                    GpuImage::new(
                        image.bounds,
                        resource,
                        image.tint,
                        layer_z + i32::from(image.z_index),
                    )
                    .with_pixel_offset(image.pixel_offset),
                );
            }
            labels.extend(layer.labels.iter().map(|label| NativeTextLabel {
                col: label.col,
                row: label.row,
                width: label.width,
                height: label.height,
                content: label.content.clone(),
                color: label.color,
                font_size: None,
            }));
        }
        let surface = Surface::from_cells(size, cells)
            .expect("the product surface has exactly one cell per grid position");
        Self::new(
            &surface,
            &images,
            &assets.atlas,
            u32::MAX,
            viewport,
            GpuClip::Surface,
            labels,
            text_scale,
        )
        .map_err(FramePlanError::Gpu)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        surface: &Surface<GpuCell>,
        images: &[GpuImage],
        atlas: &GpuAtlas,
        max_instances: u32,
        viewport: GridViewport,
        clip: GpuClip,
        labels: impl IntoIterator<Item = NativeTextLabel>,
        text_scale: TextScale,
    ) -> Result<Self, GpuPlanError> {
        Ok(Self::single(
            plan_composite(surface, images, atlas, max_instances, viewport, clip)?,
            labels.into_iter().collect(),
            text_scale,
        ))
    }

    fn single(gpu: SubmissionPlan, labels: Vec<NativeTextLabel>, text_scale: TextScale) -> Self {
        Self {
            passes: vec![FramePass {
                gpu,
                labels,
                text_scale,
            }],
        }
    }

    /// Keeps independent viewport mappings separate while rendering them in order.
    pub fn compose(mut base: Self, overlay: Self) -> Self {
        base.passes.extend(overlay.passes);
        base
    }

    pub fn passes(&self) -> &[FramePass] {
        &self.passes
    }

    pub fn viewport(&self) -> GridViewport {
        self.passes[0].viewport()
    }

    pub fn gpu(&self) -> &SubmissionPlan {
        self.passes[0].gpu()
    }

    pub fn labels(&self) -> &[NativeTextLabel] {
        self.passes[0].labels()
    }

    pub fn text_scale(&self) -> TextScale {
        self.passes[0].text_scale()
    }
}

fn ui_corner_radii(radius: punctum_ui::UiBorderRadius, bounds: PixelRect) -> [u32; 4] {
    let maximum = bounds.width.min(bounds.height) / 2;
    [
        radius.top_left.min(maximum),
        radius.top_right.min(maximum),
        radius.bottom_right.min(maximum),
        radius.bottom_left.min(maximum),
    ]
}

fn ui_visible_bounds(bounds: punctum_ui::UiRect, clip: punctum_ui::UiRect) -> Option<PixelRect> {
    bounds
        .intersect(clip)
        .map(|rect| PixelRect::new(rect.x, rect.y, rect.width, rect.height))
}

#[derive(Debug)]
pub enum FramePlanError {
    MissingSurface,
    SurfaceSizeMismatch {
        expected: GridSize,
        actual: GridSize,
    },
    UnknownAsset(AssetKey),
    InvalidUiContent(String),
    Gpu(GpuPlanError),
}

impl fmt::Display for FramePlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSurface => formatter.write_str("product frame has no grid surface"),
            Self::SurfaceSizeMismatch { expected, actual } => write!(
                formatter,
                "product layer surface {actual:?} does not match {expected:?}"
            ),
            Self::UnknownAsset(key) => write!(formatter, "unknown asset key {}", key.as_str()),
            Self::InvalidUiContent(content) => {
                write!(formatter, "invalid UI content key {content}")
            }
            Self::Gpu(error) => write!(formatter, "cannot plan product frame: {error}"),
        }
    }
}

impl Error for FramePlanError {}

#[cfg(test)]
mod tests {
    use game_assets::DecodedImage;
    use game_view::{GameView, LayerKind, TextLabel, TextRole, ViewCell, ViewImage, ViewLayer};
    use punctum_gpu::{PixelOffset, PixelSize, Rgba8, Viewport};
    use punctum_grid::{GridPos, GridRect, GridSize, Surface};

    use super::*;

    #[test]
    fn text_bounds_and_product_scales_preserve_existing_layout_rules() {
        let viewport = Viewport::new(
            PixelSize::new(960, 720),
            PixelOffset::new(10, 20),
            PixelSize::new(20, 30),
        )
        .unwrap();
        let label = NativeTextLabel {
            col: 2,
            row: 3,
            width: 4,
            height: 2,
            content: "label".into(),
            color: Rgba8::new(255, 255, 255, 255),
            font_size: None,
        };

        let bounds = text_bounds(&label, viewport).unwrap();
        assert_eq!(
            bounds,
            NativeTextBounds {
                left: 50,
                top: 110,
                right: 130,
                bottom: 170,
            }
        );
        assert_eq!((bounds.width(), bounds.height()), (80, 60));
        assert_eq!(TextScale::new(3, 5, 10, 28).font_size(30), 18.0);
        assert_eq!(TextScale::new(11, 20, 11, 22).font_size(30), 16.0);

        let overflowing = NativeTextLabel {
            col: u32::MAX,
            ..label
        };
        assert!(text_bounds(&overflowing, viewport).is_err());
    }

    fn key(value: &str) -> AssetKey {
        AssetKey::new(value).unwrap()
    }

    fn assets() -> NativeAssets {
        NativeAssets::new(vec![
            (
                key("solid/white"),
                DecodedImage::solid(Rgba8::new(255, 255, 255, 255)),
            ),
            (
                key("sprite/player"),
                DecodedImage::solid(Rgba8::new(255, 0, 0, 255)),
            ),
        ])
        .unwrap()
    }

    fn viewport() -> GridViewport {
        Viewport::new(
            PixelSize::new(20, 20),
            PixelOffset::new(0, 0),
            PixelSize::new(10, 10),
        )
        .unwrap()
    }

    fn surface(size: GridSize) -> Surface<ViewCell> {
        let mut cells = vec![ViewCell::Empty; (size.cols * size.rows) as usize];
        cells[0] = ViewCell::Fill(Rgba8::new(1, 2, 3, 255));
        Surface::from_cells(size, cells).unwrap()
    }

    #[test]
    fn layered_game_view_produces_one_complete_native_frame() {
        let assets = assets();
        let size = GridSize::new(2, 2);
        let view = GameView::new([
            ViewLayer::new(LayerKind::Map).with_surface(surface(size)),
            ViewLayer::new(LayerKind::Character).with_images(vec![
                ViewImage::new(
                    GridRect::new(GridPos::new(0, 0), GridSize::new(1, 1)),
                    key("sprite/player"),
                    Rgba8::new(255, 255, 255, 255),
                    7,
                )
                .with_pixel_offset(PixelOffset::new(2, 3)),
            ]),
            ViewLayer::new(LayerKind::Hud)
                .with_surface(surface(size))
                .with_labels(vec![TextLabel {
                    role: TextRole::Message,
                    col: 0,
                    row: 1,
                    width: 2,
                    height: 1,
                    content: "ready".into(),
                    color: Rgba8::new(9, 8, 7, 255),
                }]),
            ViewLayer::new(LayerKind::Console).with_surface(surface(size)),
        ]);
        let scale = TextScale::new(1, 2, 4, 10);
        let frame = FramePlan::from_game_view(&view, &assets, viewport(), scale).unwrap();

        assert_eq!(frame.viewport(), viewport());
        assert_eq!(frame.text_scale(), scale);
        assert_eq!(frame.labels().len(), 1);
        assert_eq!(frame.labels()[0].content, "ready");
        assert!(frame.gpu().instance_count > 0);
        assert_eq!(assets.atlas_size(), PixelSize::new(2, 1));
        assert_eq!(assets.atlas().size(), PixelSize::new(2, 1));
        assert!(assets.resource(&key("sprite/player")).is_some());
    }

    #[test]
    fn native_asset_and_frame_failures_are_explicit() {
        let duplicate = NativeAssets::new(vec![
            (key("same"), DecodedImage::solid(Rgba8::new(1, 1, 1, 255))),
            (key("same"), DecodedImage::solid(Rgba8::new(2, 2, 2, 255))),
        ])
        .err()
        .unwrap();
        let empty = NativeAssets::new(Vec::new()).err().unwrap();
        assert!(resource_id(u32::MAX as usize).is_err());
        assert!(resource_id(u32::MAX as usize + 1).is_err());
        for error in [duplicate, empty, NativeAssetError::TooManyAssets] {
            assert!(!error.to_string().is_empty());
        }

        let assets = assets();
        let missing_surface = GameView::new([ViewLayer::new(LayerKind::Hud)]);
        let error = FramePlan::from_game_view(
            &missing_surface,
            &assets,
            viewport(),
            TextScale::new(1, 1, 1, 1),
        )
        .err()
        .unwrap();
        assert!(matches!(error, FramePlanError::MissingSurface));
        assert!(!error.to_string().is_empty());

        let no_white = NativeAssets::new(vec![(
            key("sprite/player"),
            DecodedImage::solid(Rgba8::new(1, 1, 1, 255)),
        )])
        .unwrap();
        let base = GameView::new([
            ViewLayer::new(LayerKind::Map).with_surface(surface(GridSize::new(2, 2)))
        ]);
        assert!(matches!(
            FramePlan::from_game_view(&base, &no_white, viewport(), TextScale::new(1, 1, 1, 1)),
            Err(FramePlanError::UnknownAsset(_))
        ));

        let mismatch = GameView::new([
            ViewLayer::new(LayerKind::Map).with_surface(surface(GridSize::new(2, 2))),
            ViewLayer::new(LayerKind::Hud).with_surface(surface(GridSize::new(1, 1))),
        ]);
        let mismatch =
            FramePlan::from_game_view(&mismatch, &assets, viewport(), TextScale::new(1, 1, 1, 1))
                .err()
                .unwrap();
        assert!(matches!(
            mismatch,
            FramePlanError::SurfaceSizeMismatch { .. }
        ));
        assert!(!mismatch.to_string().is_empty());

        let image_view = |asset: AssetKey, col| {
            GameView::new([
                ViewLayer::new(LayerKind::Map).with_surface(surface(GridSize::new(2, 2))),
                ViewLayer::new(LayerKind::Character).with_images(vec![ViewImage::new(
                    GridRect::new(GridPos::new(col, 0), GridSize::new(1, 1)),
                    asset,
                    Rgba8::new(255, 255, 255, 255),
                    0,
                )]),
            ])
        };
        let unknown = FramePlan::from_game_view(
            &image_view(key("missing"), 0),
            &assets,
            viewport(),
            TextScale::new(1, 1, 1, 1),
        )
        .err()
        .unwrap();
        let gpu = FramePlan::from_game_view(
            &image_view(key("sprite/player"), 3),
            &assets,
            viewport(),
            TextScale::new(1, 1, 1, 1),
        )
        .err()
        .unwrap();
        for error in [unknown, gpu] {
            assert!(!error.to_string().is_empty());
        }
    }

    #[test]
    fn resolved_ui_frame_uses_pixel_instances_without_a_grid_surface() {
        use punctum_ui::{Dimension, UiColor, UiContent, UiNode, UiSize, UiStyle, UiTree};

        let tree = UiTree::<()>::new(
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    ..UiStyle::default()
                })
                .with_content(UiContent::Fill(UiColor::new(4, 5, 6, 255))),
        )
        .unwrap();
        let ui = tree.resolve(UiSize::new(80, 60)).unwrap();
        let frame = FramePlan::from_ui_frame(&ui, &assets(), TextScale::new(1, 1, 12, 12)).unwrap();

        assert_eq!(frame.viewport().target_size, PixelSize::new(80, 60));
        assert_eq!(frame.viewport().cell_size, PixelSize::new(1, 1));
        assert_eq!(frame.gpu().grid_size, GridSize::new(80, 60));
        assert_eq!(frame.gpu().instance_count, 1);
    }

    #[test]
    fn pixel_ui_keeps_per_corner_radius_for_fills_and_images() {
        use punctum_ui::{
            Dimension, UiBorderRadius, UiColor, UiContent, UiContentId, UiNode, UiSize, UiStyle,
            UiTree,
        };

        let tree = UiTree::new(
            UiNode::<()>::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    ..UiStyle::default()
                })
                .with_children([
                    UiNode::auto()
                        .with_style(UiStyle {
                            width: Dimension::Px(40),
                            height: Dimension::Px(20),
                            border_radius: UiBorderRadius {
                                top_left: 30,
                                top_right: 3,
                                bottom_right: 8,
                                bottom_left: 1,
                            },
                            ..UiStyle::default()
                        })
                        .with_content(UiContent::Fill(UiColor::new(1, 2, 3, 255))),
                    UiNode::auto()
                        .with_style(UiStyle {
                            width: Dimension::Px(30),
                            height: Dimension::Px(12),
                            border_radius: UiBorderRadius::all(9),
                            ..UiStyle::default()
                        })
                        .with_content(UiContent::Image(UiContentId::new("sprite/player").unwrap())),
                ]),
        )
        .unwrap();
        let ui = tree.resolve(UiSize::new(80, 60)).unwrap();
        let frame = FramePlan::from_ui_frame(&ui, &assets(), TextScale::new(1, 1, 12, 12)).unwrap();
        let instances = &frame.gpu().uploads[0].instances;
        assert_eq!(instances[0].corner_radii, [10, 3, 8, 1]);
        assert_eq!(instances[1].corner_radii, [6, 6, 6, 6]);
    }
}
