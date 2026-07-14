use std::{error::Error, fmt};

use punctum_grid::Patch;
use unicode_width::UnicodeWidthStr;

use crate::TerminalCell;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalRun {
    col: u16,
    row: u16,
    cells: Vec<TerminalCell>,
}

impl TerminalRun {
    pub const fn col(&self) -> u16 {
        self.col
    }

    pub const fn row(&self) -> u16 {
        self.row
    }

    pub fn cells(&self) -> &[TerminalCell] {
        &self.cells
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalPlan {
    runs: Vec<TerminalRun>,
    final_cursor: (u16, u16),
}

impl TerminalPlan {
    pub fn runs(&self) -> &[TerminalRun] {
        &self.runs
    }

    /// The presenter parks its hidden cursor at the origin after every frame.
    pub const fn final_cursor(&self) -> (u16, u16) {
        self.final_cursor
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalPlanError {
    ZeroCellWidth,
    CoordinateOverflow { col: u32, row: u32, cell_width: u16 },
}

impl fmt::Display for TerminalPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroCellWidth => {
                formatter.write_str("terminal cell width must be greater than zero")
            }
            Self::CoordinateOverflow {
                col,
                row,
                cell_width,
            } => write!(
                formatter,
                "logical cell ({col}, {row}) at width {cell_width} exceeds terminal coordinates"
            ),
        }
    }
}

impl Error for TerminalPlanError {}

pub fn plan_patch(
    patch: &Patch<TerminalCell>,
    cell_width: u16,
) -> Result<TerminalPlan, TerminalPlanError> {
    validate_cell_width(cell_width)?;

    let runs = patch
        .spans()
        .iter()
        .map(|span| {
            let logical_end = u64::from(span.start_col()) + span.cells().len() as u64;
            let terminal_end = logical_end * u64::from(cell_width);
            if terminal_end > u64::from(u16::MAX) + 1 {
                return Err(TerminalPlanError::CoordinateOverflow {
                    col: (logical_end - 1) as u32,
                    row: span.row(),
                    cell_width,
                });
            }

            let row =
                u16::try_from(span.row()).map_err(|_| TerminalPlanError::CoordinateOverflow {
                    col: span.start_col(),
                    row: span.row(),
                    cell_width,
                })?;

            Ok(TerminalRun {
                col: (u64::from(span.start_col()) * u64::from(cell_width)) as u16,
                row,
                cells: sanitize_cells(span.cells()),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(TerminalPlan {
        runs,
        final_cursor: (0, 0),
    })
}

fn sanitize_cells(cells: &[TerminalCell]) -> Vec<TerminalCell> {
    let mut sanitized = Vec::with_capacity(cells.len());
    let mut index = 0;
    while index < cells.len() {
        let cell = &cells[index];
        if let Some(grapheme) = cell.grapheme() {
            let width = UnicodeWidthStr::width(grapheme);
            if width == 1 {
                sanitized.push(cell.clone());
                index += 1;
                continue;
            }
            if width == 2
                && cells
                    .get(index + 1)
                    .is_some_and(TerminalCell::is_continuation)
            {
                sanitized.push(cell.clone());
                sanitized.push(cells[index + 1].clone());
                index += 2;
                continue;
            }
        }

        sanitized.push(TerminalCell::new(
            '\u{fffd}',
            cell.foreground(),
            cell.background(),
        ));
        index += 1;
    }
    sanitized
}

pub fn validate_cell_width(cell_width: u16) -> Result<(), TerminalPlanError> {
    if cell_width == 0 {
        Err(TerminalPlanError::ZeroCellWidth)
    } else {
        Ok(())
    }
}
