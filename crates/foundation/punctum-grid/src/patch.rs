use std::{error::Error, fmt};

use crate::{GridSize, Surface};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchKind {
    Delta,
    Replace,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatchSpan<T> {
    row: u32,
    start_col: u32,
    cells: Vec<T>,
}

impl<T> PatchSpan<T> {
    pub const fn row(&self) -> u32 {
        self.row
    }

    pub const fn start_col(&self) -> u32 {
        self.start_col
    }

    pub fn cells(&self) -> &[T] {
        &self.cells
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Patch<T> {
    size: GridSize,
    kind: PatchKind,
    spans: Vec<PatchSpan<T>>,
}

impl<T> Patch<T> {
    pub const fn size(&self) -> GridSize {
        self.size
    }

    pub const fn kind(&self) -> PatchKind {
        self.kind
    }

    pub fn spans(&self) -> &[PatchSpan<T>] {
        &self.spans
    }

    pub fn changed_cell_count(&self) -> usize {
        self.spans.iter().map(|span| span.cells.len()).sum()
    }
}

pub fn diff<T>(previous: &Surface<T>, next: &Surface<T>) -> Patch<T>
where
    T: Clone + Eq,
{
    if previous.size != next.size {
        return replacement_patch(next);
    }

    if next.size.is_empty() {
        return Patch {
            size: next.size,
            kind: PatchKind::Delta,
            spans: Vec::new(),
        };
    }

    let cols = next.size.cols as usize;
    let mut spans = Vec::new();

    for row in 0..next.size.rows {
        let row_start = row as usize * cols;
        let mut col = 0;

        while col < cols {
            while col < cols && previous.cells[row_start + col] == next.cells[row_start + col] {
                col += 1;
            }

            let span_start = col;
            while col < cols && previous.cells[row_start + col] != next.cells[row_start + col] {
                col += 1;
            }

            if span_start < col {
                spans.push(PatchSpan {
                    row,
                    start_col: span_start as u32,
                    cells: next.cells[row_start + span_start..row_start + col].to_vec(),
                });
            }
        }
    }

    Patch {
        size: next.size,
        kind: PatchKind::Delta,
        spans,
    }
}

fn replacement_patch<T>(surface: &Surface<T>) -> Patch<T>
where
    T: Clone,
{
    let cols = surface.size.cols as usize;
    let spans = if cols == 0 {
        Vec::new()
    } else {
        (0..surface.size.rows)
            .map(|row| {
                let start = row as usize * cols;
                PatchSpan {
                    row,
                    start_col: 0,
                    cells: surface.cells[start..start + cols].to_vec(),
                }
            })
            .collect()
    };

    Patch {
        size: surface.size,
        kind: PatchKind::Replace,
        spans,
    }
}

pub fn apply_patch<T>(surface: &mut Surface<T>, patch: &Patch<T>) -> Result<(), PatchApplyError>
where
    T: Clone,
{
    match patch.kind {
        PatchKind::Delta => {
            if surface.size != patch.size {
                return Err(PatchApplyError::SizeMismatch {
                    surface_size: surface.size,
                    patch_size: patch.size,
                });
            }

            let cols = surface.size.cols as usize;
            for span in &patch.spans {
                let start = span.row as usize * cols + span.start_col as usize;
                let end = start + span.cells.len();
                surface.cells[start..end].clone_from_slice(&span.cells);
            }
        }
        PatchKind::Replace => {
            let mut cells = Vec::with_capacity(patch.changed_cell_count());
            for span in &patch.spans {
                cells.extend_from_slice(&span.cells);
            }
            *surface = Surface {
                size: patch.size,
                cells,
            };
        }
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchApplyError {
    SizeMismatch {
        surface_size: GridSize,
        patch_size: GridSize,
    },
}

impl fmt::Display for PatchApplyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SizeMismatch {
                surface_size,
                patch_size,
            } => write!(
                formatter,
                "surface size {surface_size:?} does not match delta patch size {patch_size:?}"
            ),
        }
    }
}

impl Error for PatchApplyError {}
