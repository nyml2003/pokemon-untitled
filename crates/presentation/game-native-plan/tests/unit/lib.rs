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
    let base =
        GameView::new([ViewLayer::new(LayerKind::Map).with_surface(surface(GridSize::new(2, 2)))]);
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
        Dimension, UiBorderRadius, UiColor, UiContent, UiContentId, UiNode, UiSize, UiStyle, UiTree,
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
