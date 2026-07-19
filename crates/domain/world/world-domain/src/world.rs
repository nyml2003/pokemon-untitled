use std::collections::BTreeSet;

use crate::{
    actor::{WorldActor, WorldActorId},
    error::WorldError,
    map::{Direction, Position, Tile, TileMap},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// 发给玩家角色的世界命令。
///
/// 玩家进入草地会在结果中触发遭遇事件。
pub enum WorldCommand {
    Face(Direction),
    Move(Direction),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// 发给非玩家角色的世界命令。
///
/// NPC 可以移动和转向，但不会触发遭遇事件。
pub enum WorldActorCommand {
    Face(Direction),
    Move(Direction),
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// 单次世界状态转换对调用方报告的业务事件。
pub enum WorldEvent {
    /// 角色只改变朝向。
    Turned { from: Direction, to: Direction },
    /// 角色成功进入相邻格子。
    Moved { from: Position, to: Position },
    /// 角色被地图边界或不可通行地表阻挡。
    Blocked { at: Position },
    /// 角色被占据目标格子的可阻挡角色阻挡。
    BlockedByActor { actor: WorldActorId, at: Position },
    /// 玩家从非草地进入草地，并应开始遭遇流程。
    EncounterTriggered { at: Position },
    /// 命令无法应用到内部世界状态，世界保持不变。
    TransitionRejected { error: WorldError },
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// 单次状态转换产生的唯一业务事件。
pub struct WorldOutcome {
    event: WorldEvent,
}

impl WorldOutcome {
    pub fn event(&self) -> WorldEvent {
        self.event.clone()
    }

    pub const fn starts_battle(&self) -> bool {
        matches!(self.event, WorldEvent::EncounterTriggered { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// 世界状态的聚合根。
///
/// 它持有地图、玩家和 NPC，并在每次命令后返回新的状态而不修改原实例。
pub struct World {
    map: TileMap,
    player: WorldActor,
    actors: Vec<WorldActor>,
}

impl World {
    pub fn new(map: TileMap, player: Position, facing: Direction) -> Result<Self, WorldError> {
        Self::with_actors(map, player, facing, Vec::new())
    }

    pub fn with_actors(
        map: TileMap,
        player: Position,
        facing: Direction,
        mut actors: Vec<WorldActor>,
    ) -> Result<Self, WorldError> {
        validate_player(&map, player)?;
        let player = WorldActor::new(WorldActorId::player(), player, facing, true);
        let mut actor_ids = BTreeSet::from([player.id.clone()]);
        let mut occupied = BTreeSet::from([player.position]);
        for actor in &actors {
            if !actor_ids.insert(actor.id.clone()) {
                return Err(WorldError::DuplicateActor(actor.id.clone()));
            }
            validate_actor(&map, actor)?;
            if actor.blocks_movement && !occupied.insert(actor.position) {
                return Err(WorldError::ActorOverlap {
                    actor: actor.id.clone(),
                    position: actor.position,
                });
            }
        }
        actors.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(Self {
            map,
            player,
            actors,
        })
    }

    pub const fn map(&self) -> &TileMap {
        &self.map
    }

    pub const fn player(&self) -> Position {
        self.player.position
    }

    pub const fn facing(&self) -> Direction {
        self.player.facing
    }

    pub const fn player_id(&self) -> &WorldActorId {
        &self.player.id
    }

    pub fn actors(&self) -> impl Iterator<Item = &WorldActor> {
        std::iter::once(&self.player).chain(self.actors.iter())
    }

    pub fn transition(&self, command: WorldCommand) -> (Self, WorldOutcome) {
        let command = match command {
            WorldCommand::Face(direction) => WorldActorCommand::Face(direction),
            WorldCommand::Move(direction) => WorldActorCommand::Move(direction),
        };
        match self.transition_with_actor(self.player_id(), command, true) {
            Ok(result) => result,
            Err(error) => (
                self.clone(),
                WorldOutcome {
                    event: WorldEvent::TransitionRejected { error },
                },
            ),
        }
    }

    pub fn transition_actor(
        &self,
        actor: &WorldActorId,
        command: WorldActorCommand,
    ) -> Result<(Self, WorldOutcome), WorldError> {
        if actor == self.player_id() {
            return Err(WorldError::PlayerActorCommand);
        }
        self.transition_with_actor(actor, command, false)
    }

    fn transition_with_actor(
        &self,
        actor_id: &WorldActorId,
        command: WorldActorCommand,
        triggers_encounters: bool,
    ) -> Result<(Self, WorldOutcome), WorldError> {
        let actor = self
            .actors()
            .find(|actor| actor.id == *actor_id)
            .ok_or_else(|| WorldError::UnknownActor(actor_id.clone()))?;
        let from = actor.position;
        let mut next = self.clone();
        let direction = match command {
            WorldActorCommand::Face(direction) => {
                next.actor_mut(actor_id)
                    .ok_or_else(|| WorldError::UnknownActor(actor_id.clone()))?
                    .facing = direction;
                return Ok((
                    next,
                    WorldOutcome {
                        event: WorldEvent::Turned {
                            from: actor.facing,
                            to: direction,
                        },
                    },
                ));
            }
            WorldActorCommand::Move(direction) => direction,
        };
        next.actor_mut(actor_id)
            .ok_or_else(|| WorldError::UnknownActor(actor_id.clone()))?
            .facing = direction;
        let Some(target) = from.neighbor(direction) else {
            return Ok(blocked(next, from));
        };
        let Some(target_tile) = next.map.tile(target) else {
            return Ok(blocked(next, target));
        };
        if !target_tile.is_walkable() {
            return Ok(blocked(next, target));
        }
        let blocking_actor = next
            .actors()
            .find(|other| {
                other.id != *actor_id && other.blocks_movement && other.position == target
            })
            .map(|actor| actor.id.clone());
        if let Some(blocking_actor) = blocking_actor {
            return Ok((
                next,
                WorldOutcome {
                    event: WorldEvent::BlockedByActor {
                        actor: blocking_actor,
                        at: target,
                    },
                },
            ));
        }

        next.actor_mut(actor_id)
            .ok_or_else(|| WorldError::UnknownActor(actor_id.clone()))?
            .position = target;
        let entered_grass = triggers_encounters
            && next.map.tile(from) != Some(Tile::Grass)
            && target_tile == Tile::Grass;
        Ok((
            next,
            WorldOutcome {
                event: if entered_grass {
                    WorldEvent::EncounterTriggered { at: target }
                } else {
                    WorldEvent::Moved { from, to: target }
                },
            },
        ))
    }

    fn actor_mut(&mut self, actor_id: &WorldActorId) -> Option<&mut WorldActor> {
        if actor_id == self.player_id() {
            Some(&mut self.player)
        } else {
            self.actors.iter_mut().find(|actor| actor.id == *actor_id)
        }
    }
}

fn validate_player(map: &TileMap, player: Position) -> Result<(), WorldError> {
    match map.tile(player) {
        None => Err(WorldError::PlayerOutOfBounds(player)),
        Some(tile) if !tile.is_walkable() => Err(WorldError::PlayerOnBlockedTile(player)),
        Some(_) => Ok(()),
    }
}

fn validate_actor(map: &TileMap, actor: &WorldActor) -> Result<(), WorldError> {
    if actor.id.as_str().trim().is_empty() {
        return Err(WorldError::EmptyActorId);
    }
    match map.tile(actor.position) {
        None => Err(WorldError::ActorOutOfBounds {
            actor: actor.id.clone(),
            position: actor.position,
        }),
        Some(tile) if !tile.is_walkable() => Err(WorldError::ActorOnBlockedTile {
            actor: actor.id.clone(),
            position: actor.position,
        }),
        Some(_) => Ok(()),
    }
}

fn blocked(world: World, at: Position) -> (World, WorldOutcome) {
    (
        world,
        WorldOutcome {
            event: WorldEvent::Blocked { at },
        },
    )
}

#[cfg(test)]
#[path = "../tests/unit/world.rs"]
mod tests;
