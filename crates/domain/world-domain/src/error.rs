use crate::{actor::WorldActorId, map::Position};

#[derive(Clone, Debug, PartialEq, Eq)]
/// 构造或转换世界状态时违反的领域规则。
pub enum WorldError {
    EmptyMap,
    TileCount {
        expected: usize,
        actual: usize,
    },
    EmptyActorId,
    DuplicateActor(WorldActorId),
    UnknownActor(WorldActorId),
    PlayerActorCommand,
    PlayerOutOfBounds(Position),
    PlayerOnBlockedTile(Position),
    ActorOutOfBounds {
        actor: WorldActorId,
        position: Position,
    },
    ActorOnBlockedTile {
        actor: WorldActorId,
        position: Position,
    },
    ActorOverlap {
        actor: WorldActorId,
        position: Position,
    },
}
