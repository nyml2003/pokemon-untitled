use punctum_grid::{GridPos, GridRect, GridSize};
use punctum_ui::{
    Align, Border, Column, ConstraintError, Constraints, HorizontalAlign, Insets, LayoutError,
    LayoutKind, Node, Padding, PaintError, PaintTarget, Row, Spacer, SurfaceView, Text, TextLayout,
    TextLayouter, Ui, VerticalAlign,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct TestLayout {
    id: u32,
    size: GridSize,
}

impl TextLayout for TestLayout {
    fn size(&self) -> GridSize {
        self.size
    }
}

enum LayoutMode {
    Normal,
    Failing,
    Fixed(GridSize),
}

struct TestLayouter {
    calls: Vec<(String, Constraints)>,
    mode: LayoutMode,
}

impl Default for TestLayouter {
    fn default() -> Self {
        Self {
            calls: Vec::new(),
            mode: LayoutMode::Normal,
        }
    }
}

impl TestLayouter {
    fn failing() -> Self {
        Self {
            calls: Vec::new(),
            mode: LayoutMode::Failing,
        }
    }

    fn fixed(size: GridSize) -> Self {
        Self {
            calls: Vec::new(),
            mode: LayoutMode::Fixed(size),
        }
    }
}

impl TextLayouter for TestLayouter {
    type Error = &'static str;
    type Layout = TestLayout;

    fn layout_text(
        &mut self,
        content: &str,
        constraints: Constraints,
    ) -> Result<Self::Layout, Self::Error> {
        self.calls.push((content.to_owned(), constraints));
        let id = self.calls.len() as u32;
        if matches!(self.mode, LayoutMode::Failing) {
            return Err("font unavailable");
        }
        if let LayoutMode::Fixed(size) = self.mode {
            return Ok(TestLayout { id, size });
        }
        Ok(TestLayout {
            id,
            size: GridSize::new(
                (content.chars().count() as u32).min(constraints.max().cols),
                u32::from(!content.is_empty() && constraints.max().rows > 0),
            ),
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
enum PaintCall {
    Text {
        content: String,
        layout_id: u32,
        bounds: GridRect,
        clip: GridRect,
    },
    Border {
        bounds: GridRect,
        clip: GridRect,
    },
    Surface {
        name: &'static str,
        bounds: GridRect,
        clip: GridRect,
    },
}

#[derive(Clone, Copy)]
enum Failure {
    Text,
    Border,
    Surface,
}

#[derive(Default)]
struct Recorder {
    calls: Vec<PaintCall>,
    failure: Option<Failure>,
}

impl Recorder {
    fn failing(failure: Failure) -> Self {
        Self {
            calls: Vec::new(),
            failure: Some(failure),
        }
    }
}

impl PaintTarget<&'static str, TestLayout> for Recorder {
    type Error = &'static str;

    fn paint_text(
        &mut self,
        content: &str,
        layout: &TestLayout,
        bounds: GridRect,
        clip: GridRect,
    ) -> Result<(), Self::Error> {
        if matches!(self.failure, Some(Failure::Text)) {
            return Err("text paint");
        }
        self.calls.push(PaintCall::Text {
            content: content.to_owned(),
            layout_id: layout.id,
            bounds,
            clip,
        });
        Ok(())
    }

    fn paint_border(&mut self, bounds: GridRect, clip: GridRect) -> Result<(), Self::Error> {
        if matches!(self.failure, Some(Failure::Border)) {
            return Err("border paint");
        }
        self.calls.push(PaintCall::Border { bounds, clip });
        Ok(())
    }

    fn paint_surface(
        &mut self,
        surface: &&'static str,
        bounds: GridRect,
        clip: GridRect,
    ) -> Result<(), Self::Error> {
        if matches!(self.failure, Some(Failure::Surface)) {
            return Err("surface paint");
        }
        self.calls.push(PaintCall::Surface {
            name: surface,
            bounds,
            clip,
        });
        Ok(())
    }
}

fn loose(cols: u32, rows: u32) -> Constraints {
    Constraints::loose(GridSize::new(cols, rows)).unwrap()
}

fn tight(cols: u32, rows: u32) -> Constraints {
    Constraints::tight(GridSize::new(cols, rows)).unwrap()
}

#[test]
fn primitives_produce_exact_integer_layout() {
    let page: Node<&'static str> = Column::new(vec![
        Text::new("title").into(),
        Row::new(vec![
            Border::new(Padding::new(
                Insets::symmetric(1, 0),
                SurfaceView::new("board", GridSize::new(3, 2)),
            ))
            .into(),
            Spacer::new(GridSize::new(2, 1)).into(),
        ])
        .with_gap(1)
        .into(),
    ])
    .with_gap(1)
    .into();
    let ui = Ui::new(page);
    let mut layouter = TestLayouter::default();

    let frame = ui.layout(loose(20, 10), &mut layouter).unwrap();

    assert_eq!(frame.size(), GridSize::new(10, 6));
    assert_eq!(
        frame.entries(),
        vec![
            (
                LayoutKind::Column,
                GridRect::new(GridPos::new(0, 0), GridSize::new(10, 6))
            ),
            (
                LayoutKind::Text,
                GridRect::new(GridPos::new(0, 0), GridSize::new(5, 1))
            ),
            (
                LayoutKind::Row,
                GridRect::new(GridPos::new(0, 2), GridSize::new(10, 4))
            ),
            (
                LayoutKind::Border,
                GridRect::new(GridPos::new(0, 2), GridSize::new(7, 4))
            ),
            (
                LayoutKind::Padding,
                GridRect::new(GridPos::new(1, 3), GridSize::new(5, 2))
            ),
            (
                LayoutKind::SurfaceView,
                GridRect::new(GridPos::new(2, 3), GridSize::new(3, 2))
            ),
            (
                LayoutKind::Spacer,
                GridRect::new(GridPos::new(8, 2), GridSize::new(2, 1))
            ),
        ]
    );

    let mut recorder = Recorder::default();
    frame.paint(&mut recorder).unwrap();
    assert_eq!(recorder.calls.len(), 3);
}

#[test]
fn measure_and_layout_are_explicit_separate_stages() {
    let ui: Ui<&'static str> = Ui::new(SurfaceView::new("panel", GridSize::new(3, 2)));
    let mut layouter = TestLayouter::default();

    let measured = ui.measure(tight(5, 4), &mut layouter).unwrap();
    assert_eq!(measured.size(), GridSize::new(5, 4));

    let frame = measured.layout();
    assert_eq!(frame.size(), GridSize::new(5, 4));
}

#[test]
fn text_paint_reuses_the_layout_created_during_measure() {
    let ui: Ui<&'static str> = Ui::new(Text::new("readonly"));
    let mut layouter = TestLayouter::default();
    let frame = ui.layout(loose(20, 2), &mut layouter).unwrap();
    let mut recorder = Recorder::default();

    frame.paint(&mut recorder).unwrap();

    assert_eq!(layouter.calls.len(), 1);
    assert_eq!(
        recorder.calls,
        vec![PaintCall::Text {
            content: "readonly".to_owned(),
            layout_id: 1,
            bounds: GridRect::new(GridPos::new(0, 0), GridSize::new(8, 1)),
            clip: GridRect::new(GridPos::new(0, 0), GridSize::new(8, 1)),
        }]
    );
}

#[test]
fn zero_sized_layout_paints_nothing() {
    let ui: Ui<&'static str> = Ui::new(Border::new(Text::new("hidden")));
    let mut layouter = TestLayouter::default();
    let frame = ui.layout(tight(0, 0), &mut layouter).unwrap();
    let mut recorder = Recorder::default();

    frame.paint(&mut recorder).unwrap();

    assert_eq!(frame.size(), GridSize::new(0, 0));
    assert!(recorder.calls.is_empty());
}

#[test]
fn oversized_decoration_stays_inside_zero_sized_bounds() {
    let ui: Ui<&'static str> = Ui::new(Padding::new(
        Insets::new(u32::MAX, u32::MAX, u32::MAX, u32::MAX),
        Border::new(SurfaceView::new("hidden", GridSize::new(1, 1))),
    ));
    let frame = ui
        .layout(tight(0, 0), &mut TestLayouter::default())
        .unwrap();

    for (_, bounds) in frame.entries() {
        assert_eq!(
            bounds,
            GridRect::new(GridPos::new(0, 0), GridSize::new(0, 0))
        );
    }
}

#[test]
fn row_shrinks_later_children_when_space_is_insufficient() {
    let ui = Ui::new(
        Row::new(vec![
            SurfaceView::new("first", GridSize::new(4, 1)).into(),
            SurfaceView::new("second", GridSize::new(3, 1)).into(),
        ])
        .with_gap(1),
    );
    let mut layouter = TestLayouter::default();

    let frame = ui.layout(loose(5, 1), &mut layouter).unwrap();

    assert_eq!(frame.size(), GridSize::new(5, 1));
    assert_eq!(
        frame.entries(),
        vec![
            (
                LayoutKind::Row,
                GridRect::new(GridPos::new(0, 0), GridSize::new(5, 1))
            ),
            (
                LayoutKind::SurfaceView,
                GridRect::new(GridPos::new(0, 0), GridSize::new(4, 1))
            ),
            (
                LayoutKind::SurfaceView,
                GridRect::new(GridPos::new(5, 0), GridSize::new(0, 1))
            ),
        ]
    );
}

#[test]
fn nested_containers_intersect_their_clip_during_paint() {
    let ui = Ui::new(Padding::new(
        Insets::all(1),
        Border::new(SurfaceView::new("map", GridSize::new(4, 2))),
    ));
    let mut layouter = TestLayouter::default();
    let frame = ui.layout(loose(10, 10), &mut layouter).unwrap();
    let mut recorder = Recorder::default();
    let external_clip = GridRect::new(GridPos::new(2, 2), GridSize::new(3, 2));

    frame.paint_clipped(external_clip, &mut recorder).unwrap();

    assert_eq!(
        recorder.calls,
        vec![
            PaintCall::Border {
                bounds: GridRect::new(GridPos::new(1, 1), GridSize::new(6, 4)),
                clip: external_clip,
            },
            PaintCall::Surface {
                name: "map",
                bounds: GridRect::new(GridPos::new(2, 2), GridSize::new(4, 2)),
                clip: external_clip,
            },
        ]
    );
}

#[test]
fn resize_remeasures_text_and_realigns_the_child() {
    let ui: Ui<&'static str> = Ui::new(Align::new(
        HorizontalAlign::Center,
        VerticalAlign::End,
        Text::new("abcdef"),
    ));
    let mut layouter = TestLayouter::default();

    let wide = ui.layout(tight(8, 3), &mut layouter).unwrap();
    let narrow = ui.layout(tight(3, 2), &mut layouter).unwrap();

    assert_eq!(
        wide.entries()[1],
        (
            LayoutKind::Text,
            GridRect::new(GridPos::new(1, 2), GridSize::new(6, 1))
        )
    );
    assert_eq!(
        narrow.entries()[1],
        (
            LayoutKind::Text,
            GridRect::new(GridPos::new(0, 1), GridSize::new(3, 1))
        )
    );
    assert_eq!(layouter.calls.len(), 2);
    assert_eq!(layouter.calls[0].1.max(), GridSize::new(8, 3));
    assert_eq!(layouter.calls[1].1.max(), GridSize::new(3, 2));
}

#[test]
fn constraints_and_backend_failures_are_structured() {
    assert_eq!(
        Constraints::new(GridSize::new(3, 1), GridSize::new(2, 1)),
        Err(ConstraintError::MinimumExceedsMaximum {
            min: GridSize::new(3, 1),
            max: GridSize::new(2, 1),
        })
    );
    assert_eq!(loose(2, 3).min(), GridSize::new(0, 0));
    assert!(
        ConstraintError::MinimumExceedsMaximum {
            min: GridSize::new(3, 1),
            max: GridSize::new(2, 1),
        }
        .to_string()
        .contains("minimum")
    );
    assert!(
        ConstraintError::CoordinateRangeExceeded {
            size: GridSize::new(i32::MAX as u32 + 1, 1),
        }
        .to_string()
        .contains("coordinate")
    );
    assert_eq!(
        Constraints::loose(GridSize::new(i32::MAX as u32 + 1, 1)),
        Err(ConstraintError::CoordinateRangeExceeded {
            size: GridSize::new(i32::MAX as u32 + 1, 1),
        })
    );

    let ui: Ui<&'static str> = Ui::new(Text::new("text"));
    assert!(matches!(
        ui.layout(loose(4, 1), &mut TestLayouter::failing()),
        Err(LayoutError::TextLayout {
            source: "font unavailable"
        })
    ));
    assert!(
        LayoutError::TextLayout {
            source: "font unavailable"
        }
        .to_string()
        .contains("font unavailable")
    );

    let width_error = ui.layout(loose(4, 1), &mut TestLayouter::fixed(GridSize::new(5, 1)));
    assert!(matches!(
        width_error,
        Err(LayoutError::TextLayoutOutOfBounds {
            size: GridSize { cols: 5, rows: 1 },
            max: GridSize { cols: 4, rows: 1 },
        })
    ));
    let height_error = LayoutError::<&str>::TextLayoutOutOfBounds {
        size: GridSize::new(1, 2),
        max: GridSize::new(1, 1),
    };
    assert!(height_error.to_string().contains("exceeds"));
}

#[test]
fn text_layout_failures_propagate_through_every_container_path() {
    fn fails(root: Node<&'static str>) {
        let ui = Ui::new(root);
        assert!(matches!(
            ui.layout(loose(10, 10), &mut TestLayouter::failing()),
            Err(LayoutError::TextLayout {
                source: "font unavailable"
            })
        ));
    }

    fails(Border::new(Text::new("x")).into());
    fails(Padding::new(Insets::all(1), Text::new("x")).into());
    fails(
        Align::new(
            HorizontalAlign::Center,
            VerticalAlign::Center,
            Text::new("x"),
        )
        .into(),
    );
    fails(Row::new(vec![Text::new("x").into()]).into());
}

#[test]
fn all_alignment_modes_place_children_deterministically() {
    fn child_bounds(horizontal: HorizontalAlign, vertical: VerticalAlign) -> GridRect {
        let ui: Ui<&'static str> = Ui::new(Align::new(
            horizontal,
            vertical,
            SurfaceView::new("child", GridSize::new(2, 1)),
        ));
        let frame = ui
            .layout(tight(6, 5), &mut TestLayouter::default())
            .unwrap();
        frame.entries()[1].1
    }

    assert_eq!(
        child_bounds(HorizontalAlign::Start, VerticalAlign::Start).origin,
        GridPos::new(0, 0)
    );
    assert_eq!(
        child_bounds(HorizontalAlign::End, VerticalAlign::Center).origin,
        GridPos::new(4, 2)
    );
}

#[test]
fn paint_skips_invisible_subtrees_and_outside_clips() {
    let ui = Ui::new(Row::new(vec![
        Spacer::new(GridSize::new(2, 1)).into(),
        SurfaceView::new("visible", GridSize::new(2, 1)).into(),
        SurfaceView::new("hidden", GridSize::new(2, 1)).into(),
    ]));
    let frame = ui
        .layout(loose(6, 1), &mut TestLayouter::default())
        .unwrap();
    let mut recorder = Recorder::default();

    frame
        .paint_clipped(
            GridRect::new(GridPos::new(2, 0), GridSize::new(2, 1)),
            &mut recorder,
        )
        .unwrap();
    assert_eq!(recorder.calls.len(), 1);
    assert!(matches!(
        recorder.calls[0],
        PaintCall::Surface {
            name: "visible",
            ..
        }
    ));

    frame
        .paint_clipped(
            GridRect::new(GridPos::new(20, 20), GridSize::new(1, 1)),
            &mut recorder,
        )
        .unwrap();
    assert_eq!(recorder.calls.len(), 1);
}

#[test]
fn paint_target_failures_remain_structured() {
    fn paint_error(root: Node<&'static str>, failure: Failure) -> PaintError<&'static str> {
        let ui = Ui::new(root);
        let frame = ui
            .layout(loose(10, 10), &mut TestLayouter::default())
            .unwrap();
        frame.paint(&mut Recorder::failing(failure)).unwrap_err()
    }

    assert_eq!(
        paint_error(Text::new("x").into(), Failure::Text),
        PaintError::Target {
            source: "text paint"
        }
    );
    assert_eq!(
        paint_error(Row::new(vec![Text::new("x").into()]).into(), Failure::Text,),
        PaintError::Target {
            source: "text paint"
        }
    );
    assert_eq!(
        paint_error(
            Column::new(vec![Text::new("x").into()]).into(),
            Failure::Text,
        ),
        PaintError::Target {
            source: "text paint"
        }
    );
    assert_eq!(
        paint_error(
            Padding::new(Insets::all(1), Text::new("x")).into(),
            Failure::Text,
        ),
        PaintError::Target {
            source: "text paint"
        }
    );
    assert_eq!(
        paint_error(
            Align::new(HorizontalAlign::Start, VerticalAlign::Start, Text::new("x"),).into(),
            Failure::Text,
        ),
        PaintError::Target {
            source: "text paint"
        }
    );
    assert_eq!(
        paint_error(Border::new(Text::new("x")).into(), Failure::Text),
        PaintError::Target {
            source: "text paint"
        }
    );
    assert_eq!(
        paint_error(
            Border::new(SurfaceView::new("x", GridSize::new(1, 1))).into(),
            Failure::Border,
        ),
        PaintError::Target {
            source: "border paint"
        }
    );
    let surface_error = paint_error(
        SurfaceView::new("x", GridSize::new(1, 1)).into(),
        Failure::Surface,
    );
    assert_eq!(
        surface_error,
        PaintError::Target {
            source: "surface paint"
        }
    );
    assert!(surface_error.to_string().contains("surface paint"));
}
