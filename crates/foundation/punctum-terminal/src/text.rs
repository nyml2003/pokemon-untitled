use std::{error::Error, fmt};

use punctum_grid::{GridPos, GridSize, Surface, SurfaceError};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{TerminalCell, TerminalColor};

pub fn write_text(
    surface: &mut Surface<TerminalCell>,
    position: GridPos,
    text: &str,
    foreground: TerminalColor,
    background: TerminalColor,
) -> Result<GridPos, TerminalTextError> {
    let size = surface.size();
    if position.col < 0
        || position.row < 0
        || position.col as u32 >= size.cols
        || position.row as u32 >= size.rows
    {
        return Err(TerminalTextError::PositionOutOfBounds { position, size });
    }

    let row = position.row;
    let mut col = position.col as u32;
    for grapheme in text.graphemes(true) {
        if col >= size.cols {
            break;
        }

        let width = UnicodeWidthStr::width(grapheme);
        if width == 0 {
            continue;
        }
        if col + width as u32 > size.cols {
            clear_slot(surface, col, row as u32);
            col = size.cols;
            break;
        }

        for slot in col..col + width as u32 {
            clear_slot(surface, slot, row as u32);
        }
        surface
            .set(
                GridPos::new(col as i32, row),
                TerminalCell::from_grapheme(grapheme, foreground, background)
                    .expect("Unicode segmentation yields one grapheme"),
            )
            .expect("validated text position is in bounds");
        if width == 2 {
            surface
                .set(
                    GridPos::new(col as i32 + 1, row),
                    TerminalCell::continuation(foreground, background),
                )
                .expect("validated continuation position is in bounds");
        }
        col += width as u32;
    }

    Ok(GridPos::new(col as i32, row))
}

pub fn resize_text_surface(
    surface: &Surface<TerminalCell>,
    size: GridSize,
) -> Result<Surface<TerminalCell>, SurfaceError> {
    let mut resized = Surface::filled(size, TerminalCell::default())?;
    let copied_cols = surface.size().cols.min(size.cols);
    let copied_rows = surface.size().rows.min(size.rows);

    for row in 0..copied_rows {
        for col in 0..copied_cols {
            resized
                .set(
                    GridPos::new(col as i32, row as i32),
                    surface
                        .get(GridPos::new(col as i32, row as i32))
                        .expect("copy coordinates are inside the source")
                        .clone(),
                )
                .expect("copy coordinates are inside the destination");
        }
        sanitize_row(&mut resized, row);
    }

    Ok(resized)
}

fn clear_slot(surface: &mut Surface<TerminalCell>, col: u32, row: u32) {
    let position = GridPos::new(col as i32, row as i32);
    let cell = surface
        .get(position)
        .expect("text slot is inside the surface")
        .clone();

    if cell.is_continuation() && col > 0 {
        surface
            .set(
                GridPos::new(col as i32 - 1, row as i32),
                TerminalCell::default(),
            )
            .expect("continuation lead is inside the surface");
    } else if cell
        .grapheme()
        .is_some_and(|text| UnicodeWidthStr::width(text) == 2)
        && col + 1 < surface.size().cols
    {
        surface
            .set(
                GridPos::new(col as i32 + 1, row as i32),
                TerminalCell::default(),
            )
            .expect("wide grapheme continuation is inside the surface");
    }

    surface
        .set(position, TerminalCell::default())
        .expect("text slot is inside the surface");
}

fn sanitize_row(surface: &mut Surface<TerminalCell>, row: u32) {
    let cols = surface.size().cols;
    let mut col = 0;
    while col < cols {
        let position = GridPos::new(col as i32, row as i32);
        let cell = surface
            .get(position)
            .expect("row position is inside the surface")
            .clone();
        if let Some(grapheme) = cell.grapheme() {
            if UnicodeWidthStr::width(grapheme) == 2 {
                let has_continuation = col + 1 < cols
                    && surface
                        .get(GridPos::new(col as i32 + 1, row as i32))
                        .expect("checked continuation position is in bounds")
                        .is_continuation();
                if has_continuation {
                    col += 2;
                    continue;
                }
                surface
                    .set(position, TerminalCell::default())
                    .expect("row position is inside the surface");
            }
        } else {
            surface
                .set(position, TerminalCell::default())
                .expect("row position is inside the surface");
        }
        col += 1;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalTextError {
    PositionOutOfBounds { position: GridPos, size: GridSize },
}

impl fmt::Display for TerminalTextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PositionOutOfBounds { position, size } => {
                write!(
                    formatter,
                    "text position {position:?} is outside surface {size:?}"
                )
            }
        }
    }
}

impl Error for TerminalTextError {}
