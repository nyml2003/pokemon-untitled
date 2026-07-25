use punctum_gpu::{Rgba8, Viewport as GridViewport};

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
