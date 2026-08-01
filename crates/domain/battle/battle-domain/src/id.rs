use crate::error::ValidationError;
pub const TEAM_SIZE: usize = 6;
/// 一只宝可梦可携带的招式数量上限。
pub const MAX_MOVES: usize = 4;

/// 对战中两支队伍的稳定标识。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TeamSlot(u8);

impl TeamSlot {
    /// 从零基索引创建队伍位置。
    ///
    /// 索引必须小于 [`TEAM_SIZE`]。
    pub fn new(index: usize) -> Result<Self, ValidationError> {
        if index < TEAM_SIZE {
            Ok(Self(index as u8))
        } else {
            Err(ValidationError::InvalidTeamSlot { index })
        }
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }

    pub const fn from_valid_index(index: usize) -> Self {
        Self(index as u8)
    }
}

/// 招式列表内零基且已验证的位置。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MoveSlot(u8);

impl MoveSlot {
    /// 从零基索引创建招式位置。
    ///
    /// 索引必须小于 [`MAX_MOVES`]。
    pub fn new(index: usize) -> Result<Self, ValidationError> {
        if index < MAX_MOVES {
            Ok(Self(index as u8))
        } else {
            Err(ValidationError::InvalidMoveSlot { index })
        }
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }

    pub const fn from_valid_index(index: usize) -> Self {
        Self(index as u8)
    }
}

/// 宝可梦的非空稳定业务标识。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PokemonId(String);

impl PokemonId {
    /// 创建非空标识。
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        Self::from_string(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn from_string(value: String) -> Result<Self, ValidationError> {
        if value.trim().is_empty() {
            Err(ValidationError::EmptyPokemonId)
        } else {
            Ok(Self(value))
        }
    }
}

/// 招式的非空稳定业务标识。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MoveId(String);

impl MoveId {
    /// 创建非空标识。
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        Self::from_string(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn from_string(value: String) -> Result<Self, ValidationError> {
        if value.trim().is_empty() {
            Err(ValidationError::EmptyMoveId)
        } else {
            Ok(Self(value))
        }
    }
}

/// 第三世代属性相性使用的十八种属性。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BattleUnitId(String);

impl BattleUnitId {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ValidationError::EmptyPokemonId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 全国图鉴编号。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NationalDexId(u16);

impl NationalDexId {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
}

/// 形态/变体编号。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormId(u32);

impl FormId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
}
