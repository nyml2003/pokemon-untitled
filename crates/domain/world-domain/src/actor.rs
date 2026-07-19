use crate::{
    error::WorldError,
    map::{Direction, Position},
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// 世界中角色的稳定业务标识。
///
/// `World` 构造时拒绝重复标识，并保留 `player` 作为玩家的固定标识。
pub struct WorldActorId(String);

impl WorldActorId {
    pub fn new(value: impl Into<String>) -> Result<Self, WorldError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(WorldError::EmptyActorId);
        }
        Ok(Self(value))
    }

    pub fn player() -> Self {
        Self("player".into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// 世界中的玩家或 NPC。
///
/// 角色可以占据地图格子；只有 `blocks_movement` 为 `true` 的角色会阻挡其他角色移动。
pub struct WorldActor {
    pub(crate) id: WorldActorId,
    pub(crate) position: Position,
    pub(crate) facing: Direction,
    pub(crate) blocks_movement: bool,
}

impl WorldActor {
    pub const fn new(
        id: WorldActorId,
        position: Position,
        facing: Direction,
        blocks_movement: bool,
    ) -> Self {
        Self {
            id,
            position,
            facing,
            blocks_movement,
        }
    }

    pub const fn id(&self) -> &WorldActorId {
        &self.id
    }

    pub const fn position(&self) -> Position {
        self.position
    }

    pub const fn facing(&self) -> Direction {
        self.facing
    }

    pub const fn blocks_movement(&self) -> bool {
        self.blocks_movement
    }
}
