use std::cmp::{max, min};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct GridPos {
    pub col: i32,
    pub row: i32,
}

impl GridPos {
    pub const fn new(col: i32, row: i32) -> Self {
        Self { col, row }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct GridSize {
    pub cols: u32,
    pub rows: u32,
}

impl GridSize {
    pub const fn new(cols: u32, rows: u32) -> Self {
        Self { cols, rows }
    }

    pub const fn is_empty(self) -> bool {
        self.cols == 0 || self.rows == 0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct GridRect {
    pub origin: GridPos,
    pub size: GridSize,
}

impl GridRect {
    pub const fn new(origin: GridPos, size: GridSize) -> Self {
        Self { origin, size }
    }

    pub fn contains(self, position: GridPos) -> bool {
        let col = i64::from(position.col);
        let row = i64::from(position.row);
        let left = i64::from(self.origin.col);
        let top = i64::from(self.origin.row);

        col >= left && col < self.right() && row >= top && row < self.bottom()
    }

    pub fn intersection(self, other: Self) -> Option<Self> {
        let left = max(i64::from(self.origin.col), i64::from(other.origin.col));
        let top = max(i64::from(self.origin.row), i64::from(other.origin.row));
        let right = min(self.right(), other.right());
        let bottom = min(self.bottom(), other.bottom());

        if left >= right || top >= bottom {
            return None;
        }

        Some(Self::new(
            GridPos::new(left as i32, top as i32),
            GridSize::new((right - left) as u32, (bottom - top) as u32),
        ))
    }

    pub fn clip_to(self, size: GridSize) -> Option<Self> {
        self.intersection(Self::new(GridPos::new(0, 0), size))
    }

    pub(crate) fn fits_within(self, size: GridSize) -> bool {
        let left = i64::from(self.origin.col);
        let top = i64::from(self.origin.row);

        left >= 0
            && top >= 0
            && self.right() <= i64::from(size.cols)
            && self.bottom() <= i64::from(size.rows)
    }

    fn right(self) -> i64 {
        i64::from(self.origin.col) + i64::from(self.size.cols)
    }

    fn bottom(self) -> i64 {
        i64::from(self.origin.row) + i64::from(self.size.rows)
    }
}
