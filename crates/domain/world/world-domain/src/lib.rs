//! 整数网格世界的纯领域规则。
//!
//! 本 crate 定义地图、角色、移动命令及其结果事件。
//! 它不访问文件、窗口、GPU 或真实时间。
//! 调用方通过 [`World::transition`] 和 [`World::transition_actor`] 取得新的 [`World`] 与 [`WorldOutcome`]。
//! 原有状态保持不变。
//!
//! 玩家进入草地时会产生 [`WorldEvent::EncounterTriggered`]。
//! 非玩家角色不会触发遭遇；它们只能通过 [`World::transition_actor`] 移动。

#![forbid(unsafe_code)]

mod actor;
mod error;
mod map;
mod world;

pub use actor::{WorldActor, WorldActorId};
pub use error::WorldError;
pub use map::{Direction, Position, Tile, TileMap};
pub use world::{World, WorldActorCommand, WorldCommand, WorldEvent, WorldOutcome};
