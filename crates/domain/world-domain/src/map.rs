use crate::error::WorldError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// 世界地图中一个格子的地表类型。
pub enum Tile {
    /// 普通可通行地面。
    Ground,
    /// 不可通行的障碍物。
    Wall,
    /// 玩家从非草地进入时会触发遭遇的可通行地面。
    Grass,
}

impl Tile {
    pub const fn is_walkable(self) -> bool {
        !matches!(self, Self::Wall)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
/// 地图的零基格子坐标。
///
/// `x` 向右递增，`y` 向下递增。
pub struct Position {
    x: u16,
    y: u16,
}

impl Position {
    pub const fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }

    pub const fn x(self) -> u16 {
        self.x
    }

    pub const fn y(self) -> u16 {
        self.y
    }

    pub(crate) fn neighbor(self, direction: Direction) -> Option<Self> {
        match direction {
            Direction::Up => self.y.checked_sub(1).map(|y| Self::new(self.x, y)),
            Direction::Down => self.y.checked_add(1).map(|y| Self::new(self.x, y)),
            Direction::Left => self.x.checked_sub(1).map(|x| Self::new(x, self.y)),
            Direction::Right => self.x.checked_add(1).map(|x| Self::new(x, self.y)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// 角色朝向和移动命令使用的四个正交方向。
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// 固定尺寸的世界地图。
///
/// 格子按从左到右、从上到下的顺序存储。
pub struct TileMap {
    width: u16,
    height: u16,
    tiles: Vec<Tile>,
}

impl TileMap {
    pub fn new(width: u16, height: u16, tiles: Vec<Tile>) -> Result<Self, WorldError> {
        if width == 0 || height == 0 {
            return Err(WorldError::EmptyMap);
        }
        let expected = usize::from(width) * usize::from(height);
        if tiles.len() != expected {
            return Err(WorldError::TileCount {
                expected,
                actual: tiles.len(),
            });
        }
        Ok(Self {
            width,
            height,
            tiles,
        })
    }

    pub const fn width(&self) -> u16 {
        self.width
    }

    pub const fn height(&self) -> u16 {
        self.height
    }

    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }

    pub fn tile(&self, position: Position) -> Option<Tile> {
        if position.x >= self.width || position.y >= self.height {
            return None;
        }
        Some(
            self.tiles[usize::from(position.y) * usize::from(self.width) + usize::from(position.x)],
        )
    }
}
