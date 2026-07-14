use std::{error::Error, fmt, mem::size_of};

use crate::{GridPos, GridRect, GridSize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Surface<T> {
    pub(crate) size: GridSize,
    pub(crate) cells: Vec<T>,
}

impl<T> Surface<T> {
    pub fn filled(size: GridSize, value: T) -> Result<Self, SurfaceError>
    where
        T: Clone,
    {
        let length = checked_length::<T>(size)?;
        Ok(Self {
            size,
            cells: vec![value; length],
        })
    }

    pub fn from_cells(size: GridSize, cells: Vec<T>) -> Result<Self, SurfaceError> {
        let expected = checked_length::<T>(size)?;
        let actual = cells.len();
        if actual != expected {
            return Err(SurfaceError::LengthMismatch {
                size,
                expected,
                actual,
            });
        }

        Ok(Self { size, cells })
    }

    pub const fn size(&self) -> GridSize {
        self.size
    }

    pub fn cells(&self) -> &[T] {
        &self.cells
    }

    pub fn get(&self, position: GridPos) -> Result<&T, SurfaceError> {
        let index = self.index_of(position)?;
        Ok(&self.cells[index])
    }

    pub fn set(&mut self, position: GridPos, value: T) -> Result<(), SurfaceError> {
        let index = self.index_of(position)?;
        self.cells[index] = value;
        Ok(())
    }

    pub fn fill(&mut self, value: T)
    where
        T: Clone,
    {
        self.cells.fill(value);
    }

    pub fn fill_rect(&mut self, rect: GridRect, value: T) -> Result<(), SurfaceError>
    where
        T: Clone,
    {
        if !rect.fits_within(self.size) {
            return Err(SurfaceError::RectOutOfBounds {
                rect,
                size: self.size,
            });
        }

        self.fill_rect_unchecked(rect, value);
        Ok(())
    }

    pub fn fill_rect_clipped(&mut self, rect: GridRect, value: T) -> Option<GridRect>
    where
        T: Clone,
    {
        let clipped = rect.clip_to(self.size)?;
        self.fill_rect_unchecked(clipped, value);
        Some(clipped)
    }

    pub fn blit(&mut self, destination: GridPos, source: &Self) -> Result<(), SurfaceError>
    where
        T: Clone,
    {
        let rect = GridRect::new(destination, source.size);
        if !rect.fits_within(self.size) {
            return Err(SurfaceError::RectOutOfBounds {
                rect,
                size: self.size,
            });
        }

        self.blit_region(destination, source, rect);
        Ok(())
    }

    pub fn blit_clipped(&mut self, destination: GridPos, source: &Self) -> Option<GridRect>
    where
        T: Clone,
    {
        let destination_rect = GridRect::new(destination, source.size);
        let clipped = destination_rect.clip_to(self.size)?;
        self.blit_region(destination, source, clipped);
        Some(clipped)
    }

    fn index_of(&self, position: GridPos) -> Result<usize, SurfaceError> {
        if position.col < 0
            || position.row < 0
            || position.col as u32 >= self.size.cols
            || position.row as u32 >= self.size.rows
        {
            return Err(SurfaceError::PositionOutOfBounds {
                position,
                size: self.size,
            });
        }

        Ok(flat_index(
            self.size,
            position.col as u32,
            position.row as u32,
        ))
    }

    fn fill_rect_unchecked(&mut self, rect: GridRect, value: T)
    where
        T: Clone,
    {
        if rect.size.is_empty() {
            return;
        }

        for row_offset in 0..rect.size.rows {
            let row = (i64::from(rect.origin.row) + i64::from(row_offset)) as u32;
            let start = flat_index(self.size, rect.origin.col as u32, row);
            let end = start + rect.size.cols as usize;
            self.cells[start..end].fill(value.clone());
        }
    }

    fn blit_region(&mut self, destination: GridPos, source: &Self, target_rect: GridRect)
    where
        T: Clone,
    {
        if target_rect.size.is_empty() {
            return;
        }

        let source_col = (i64::from(target_rect.origin.col) - i64::from(destination.col)) as u32;
        let source_row = (i64::from(target_rect.origin.row) - i64::from(destination.row)) as u32;

        for row_offset in 0..target_rect.size.rows {
            let target_row = target_rect.origin.row as u32 + row_offset;
            let target_start = flat_index(self.size, target_rect.origin.col as u32, target_row);
            let target_end = target_start + target_rect.size.cols as usize;
            let source_start = flat_index(source.size, source_col, source_row + row_offset);
            let source_end = source_start + target_rect.size.cols as usize;

            self.cells[target_start..target_end]
                .clone_from_slice(&source.cells[source_start..source_end]);
        }
    }
}

fn checked_length<T>(size: GridSize) -> Result<usize, SurfaceError> {
    let area = u128::from(size.cols) * u128::from(size.rows);
    let bytes_per_cell = size_of::<T>().max(1) as u128;
    let maximum = (isize::MAX as u128) / bytes_per_cell;

    if area > maximum {
        return Err(SurfaceError::CapacityOverflow { size });
    }

    Ok(area as usize)
}

fn flat_index(size: GridSize, col: u32, row: u32) -> usize {
    row as usize * size.cols as usize + col as usize
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceError {
    CapacityOverflow {
        size: GridSize,
    },
    LengthMismatch {
        size: GridSize,
        expected: usize,
        actual: usize,
    },
    PositionOutOfBounds {
        position: GridPos,
        size: GridSize,
    },
    RectOutOfBounds {
        rect: GridRect,
        size: GridSize,
    },
}

impl fmt::Display for SurfaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityOverflow { size } => {
                write!(formatter, "surface capacity overflows for size {size:?}")
            }
            Self::LengthMismatch {
                size,
                expected,
                actual,
            } => write!(
                formatter,
                "surface size {size:?} requires {expected} cells, received {actual}"
            ),
            Self::PositionOutOfBounds { position, size } => {
                write!(
                    formatter,
                    "position {position:?} is outside surface {size:?}"
                )
            }
            Self::RectOutOfBounds { rect, size } => {
                write!(formatter, "rectangle {rect:?} is outside surface {size:?}")
            }
        }
    }
}

impl Error for SurfaceError {}
