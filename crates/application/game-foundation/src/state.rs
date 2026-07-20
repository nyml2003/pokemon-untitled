use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use world_domain::{
    Direction as WorldDirection, World, WorldActor, WorldActorId, WorldCommand, WorldError,
    WorldEvent,
};

use crate::{
    BattleId, CreatureId, CreatureTemplateId, EconomyError, EventFlagId, Inventory, ItemId, MapId,
    Money, NpcCapability, NpcId, ShopId, ThinSliceContent, TrainerId, WarpId,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CreatureState {
    id: CreatureId,
    template: CreatureTemplateId,
    hp: u16,
    pp: u8,
    experience: u32,
}

impl CreatureState {
    fn from_template(
        id: CreatureId,
        template: &crate::CreatureTemplate,
    ) -> Result<Self, GameError> {
        Ok(Self {
            id,
            template: template.id().clone(),
            hp: template.max_hp(),
            pp: template.max_pp(),
            experience: 0,
        })
    }

    pub fn id(&self) -> &CreatureId {
        &self.id
    }

    pub fn template(&self) -> &CreatureTemplateId {
        &self.template
    }

    pub const fn hp(&self) -> u16 {
        self.hp
    }

    pub const fn pp(&self) -> u8 {
        self.pp
    }

    pub const fn experience(&self) -> u32 {
        self.experience
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BattleOutcome {
    Victory,
    Defeat,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActiveBattle {
    battle: BattleId,
    participant: CreatureId,
    encounter_roll: Option<u8>,
    #[serde(default)]
    trainer: Option<TrainerId>,
}

impl ActiveBattle {
    pub fn battle(&self) -> &BattleId {
        &self.battle
    }

    pub fn participant(&self) -> &CreatureId {
        &self.participant
    }

    pub const fn encounter_roll(&self) -> Option<u8> {
        self.encounter_roll
    }

    pub fn trainer(&self) -> Option<&TrainerId> {
        self.trainer.as_ref()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GameState {
    map: MapId,
    position: Position,
    facing: Direction,
    party: Vec<CreatureState>,
    inventory: Inventory,
    money: Money,
    flags: BTreeSet<EventFlagId>,
    defeated_trainers: BTreeSet<NpcId>,
    active_battle: Option<ActiveBattle>,
    pending_encounter: Option<Position>,
    encounter_history: Vec<u8>,
    #[serde(default)]
    last_message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameCommand {
    NewGame,
    Interact {
        npc: NpcId,
    },
    Move {
        direction: Direction,
    },
    Warp {
        warp: WarpId,
    },
    Encounter {
        roll: u8,
    },
    ResolveBattle {
        outcome: BattleOutcome,
        hp: u16,
        pp: u8,
    },
    Buy {
        npc: NpcId,
        item: ItemId,
        quantity: u16,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameEvent {
    NewGameStarted,
    GiftReceived {
        creature: CreatureId,
        item: ItemId,
        quantity: u16,
    },
    Moved {
        position: Position,
    },
    MovementBlocked {
        position: Position,
    },
    Warped {
        map: MapId,
        position: Position,
    },
    EncounterStarted {
        battle: BattleId,
        roll: u8,
    },
    EncounterAvailable {
        position: Position,
    },
    TrainerBattleStarted {
        battle: BattleId,
        trainer: TrainerId,
    },
    MerchantOpened {
        shop: ShopId,
    },
    BattleResolved {
        battle: BattleId,
        outcome: BattleOutcome,
        experience: u32,
        money: Money,
    },
    ItemPurchased {
        item: ItemId,
        quantity: u16,
        spent: Money,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameError {
    Id(crate::GameIdError),
    Economy(EconomyError),
    StartingMapMissing(MapId),
    MapMissing(MapId),
    WrongMap {
        expected: MapId,
        actual: MapId,
    },
    UnknownNpc(NpcId),
    UnknownWarp(WarpId),
    UnknownItem(ItemId),
    UnknownShop(ShopId),
    UnknownBattle(BattleId),
    UnknownTrainer(TrainerId),
    UnknownCreatureTemplate(CreatureTemplateId),
    EncounterUnavailable {
        map: MapId,
    },
    UnknownEventFlag(EventFlagId),
    InvalidDefeatedTrainer(NpcId),
    DuplicateCreature(CreatureId),
    InvalidCreatureState {
        creature: CreatureId,
        hp: u16,
        pp: u8,
    },
    InvalidInventoryCapacity {
        expected: u16,
        actual: u16,
    },
    InvalidInventoryQuantity {
        item: ItemId,
        quantity: u16,
    },
    InvalidActiveBattleParticipant(CreatureId),
    InvalidPosition {
        map: MapId,
        position: Position,
    },
    InvalidPendingEncounter {
        map: MapId,
        position: Position,
    },
    World(WorldError),
    GiftAlreadyClaimed(EventFlagId),
    TrainerAlreadyDefeated(NpcId),
    InteractionUnavailable(NpcId),
    PartyRequired,
    BattleAlreadyActive,
    EncounterPending,
    BattleMissing,
    InvalidEncounterRoll(u8),
    InvalidBattleState {
        hp: u16,
        pp: u8,
    },
    ExperienceOverflow,
}

impl From<crate::GameIdError> for GameError {
    fn from(value: crate::GameIdError) -> Self {
        Self::Id(value)
    }
}

impl From<EconomyError> for GameError {
    fn from(value: EconomyError) -> Self {
        Self::Economy(value)
    }
}

impl From<WorldError> for GameError {
    fn from(value: WorldError) -> Self {
        Self::World(value)
    }
}

impl GameState {
    pub fn new(content: &ThinSliceContent) -> Result<Self, GameError> {
        let map = content.starting_map().clone();
        let definition = content
            .map(&map)
            .ok_or_else(|| GameError::StartingMapMissing(map.clone()))?;
        let state = Self {
            map,
            position: definition.spawn(),
            facing: Direction::Down,
            party: Vec::new(),
            inventory: Inventory::new(content.inventory_capacity())?,
            money: content.starting_money(),
            flags: BTreeSet::new(),
            defeated_trainers: BTreeSet::new(),
            active_battle: None,
            pending_encounter: None,
            encounter_history: Vec::new(),
            last_message: None,
        };
        state.validate(content)?;
        Ok(state)
    }

    pub fn map(&self) -> &MapId {
        &self.map
    }

    pub const fn position(&self) -> Position {
        self.position
    }

    pub const fn facing(&self) -> Direction {
        self.facing
    }

    pub fn party(&self) -> &[CreatureState] {
        &self.party
    }

    pub fn inventory(&self) -> &Inventory {
        &self.inventory
    }

    pub const fn money(&self) -> Money {
        self.money
    }

    pub fn flags(&self) -> &BTreeSet<EventFlagId> {
        &self.flags
    }

    pub fn defeated_trainers(&self) -> &BTreeSet<NpcId> {
        &self.defeated_trainers
    }

    pub fn active_battle(&self) -> Option<&ActiveBattle> {
        self.active_battle.as_ref()
    }

    pub const fn pending_encounter(&self) -> Option<Position> {
        self.pending_encounter
    }

    pub fn encounter_history(&self) -> &[u8] {
        &self.encounter_history
    }

    pub fn last_message(&self) -> Option<&str> {
        self.last_message.as_deref()
    }

    pub fn validate(&self, content: &ThinSliceContent) -> Result<(), GameError> {
        let map = content
            .map(&self.map)
            .ok_or_else(|| GameError::MapMissing(self.map.clone()))?;
        if map
            .tile(self.position)
            .is_none_or(|tile| !tile.is_walkable())
        {
            return Err(GameError::InvalidPosition {
                map: self.map.clone(),
                position: self.position,
            });
        }
        if self.inventory.capacity() != content.inventory_capacity() {
            return Err(GameError::InvalidInventoryCapacity {
                expected: content.inventory_capacity(),
                actual: self.inventory.capacity(),
            });
        }
        if self.inventory.entries().len() > usize::from(self.inventory.capacity()) {
            return Err(EconomyError::CapacityExceeded {
                capacity: self.inventory.capacity(),
            }
            .into());
        }
        for (item, quantity) in self.inventory.entries() {
            let definition = content
                .item(item)
                .ok_or_else(|| GameError::UnknownItem(item.clone()))?;
            if *quantity == 0 {
                return Err(GameError::InvalidInventoryQuantity {
                    item: item.clone(),
                    quantity: *quantity,
                });
            }
            if *quantity > definition.stack_limit() {
                return Err(EconomyError::StackLimitExceeded {
                    item: item.clone(),
                    limit: definition.stack_limit(),
                    attempted: *quantity,
                }
                .into());
            }
        }
        let mut creature_ids = BTreeSet::new();
        for creature in &self.party {
            if !creature_ids.insert(creature.id.clone()) {
                return Err(GameError::DuplicateCreature(creature.id.clone()));
            }
            let template = content
                .creature(&creature.template)
                .ok_or_else(|| GameError::UnknownCreatureTemplate(creature.template.clone()))?;
            if creature.hp > template.max_hp() || creature.pp > template.max_pp() {
                return Err(GameError::InvalidCreatureState {
                    creature: creature.id.clone(),
                    hp: creature.hp,
                    pp: creature.pp,
                });
            }
        }
        for flag in &self.flags {
            if !content.has_event_flag(flag) {
                return Err(GameError::UnknownEventFlag(flag.clone()));
            }
        }
        for trainer in &self.defeated_trainers {
            let definition = content
                .npc(trainer)
                .ok_or_else(|| GameError::InvalidDefeatedTrainer(trainer.clone()))?;
            if !definition
                .capabilities()
                .iter()
                .any(|capability| matches!(capability, NpcCapability::Trainer { .. }))
            {
                return Err(GameError::InvalidDefeatedTrainer(trainer.clone()));
            }
        }
        if let Some(roll) = self
            .encounter_history
            .iter()
            .copied()
            .find(|roll| *roll > 99)
        {
            return Err(GameError::InvalidEncounterRoll(roll));
        }
        if let Some(active) = &self.active_battle {
            if content.battle(&active.battle).is_none() {
                return Err(GameError::UnknownBattle(active.battle.clone()));
            }
            if !creature_ids.contains(&active.participant) {
                return Err(GameError::InvalidActiveBattleParticipant(
                    active.participant.clone(),
                ));
            }
            if let Some(trainer) = active.trainer()
                && content.trainer(trainer).is_none()
            {
                return Err(GameError::UnknownTrainer(trainer.clone()));
            }
        }
        if let Some(position) = self.pending_encounter
            && map.tile(position) != Some(world_domain::Tile::Grass)
        {
            return Err(GameError::InvalidPendingEncounter {
                map: self.map.clone(),
                position,
            });
        }
        self.world(content)?;
        Ok(())
    }

    pub fn transition(
        self,
        content: &ThinSliceContent,
        command: GameCommand,
    ) -> (Self, Result<GameEvent, GameError>) {
        let mut candidate = self.clone();
        let result = candidate.apply(content, command);
        match result {
            Ok(event) => (candidate, Ok(event)),
            Err(error) => (self, Err(error)),
        }
    }

    fn apply(
        &mut self,
        content: &ThinSliceContent,
        command: GameCommand,
    ) -> Result<GameEvent, GameError> {
        match command {
            GameCommand::NewGame => {
                *self = Self::new(content)?;
                Ok(GameEvent::NewGameStarted)
            }
            GameCommand::Interact { npc } => self.interact(content, npc),
            GameCommand::Move { direction } => self.move_player(content, direction),
            GameCommand::Warp { warp } => self.warp(content, warp),
            GameCommand::Encounter { roll } => self.encounter(content, roll),
            GameCommand::ResolveBattle { outcome, hp, pp } => {
                self.resolve_battle(content, outcome, hp, pp)
            }
            GameCommand::Buy {
                npc,
                item,
                quantity,
            } => self.buy(content, npc, item, quantity),
        }
    }

    fn interact(&mut self, content: &ThinSliceContent, npc: NpcId) -> Result<GameEvent, GameError> {
        let definition = content
            .npc(&npc)
            .ok_or_else(|| GameError::UnknownNpc(npc.clone()))?;
        self.require_map(definition.actor().map())?;
        self.require_interaction_position(definition.actor().position(), npc.clone())?;
        for capability in definition.capabilities() {
            match capability {
                NpcCapability::Gift {
                    claimed_flag,
                    creature,
                    item,
                    quantity,
                } if !self.flags.contains(claimed_flag) => {
                    let template = content
                        .creature(creature)
                        .ok_or_else(|| GameError::UnknownCreatureTemplate(creature.clone()))?;
                    let item_definition = content
                        .item(item)
                        .ok_or_else(|| GameError::UnknownItem(item.clone()))?;
                    let creature_id = CreatureId::new(format!("{}-1", creature.as_str()))?;
                    self.party
                        .push(CreatureState::from_template(creature_id.clone(), template)?);
                    self.inventory.add(item_definition, *quantity)?;
                    self.flags.insert(claimed_flag.clone());
                    return Ok(GameEvent::GiftReceived {
                        creature: creature_id,
                        item: item.clone(),
                        quantity: *quantity,
                    });
                }
                NpcCapability::Gift { claimed_flag, .. } => {
                    return Err(GameError::GiftAlreadyClaimed(claimed_flag.clone()));
                }
                NpcCapability::Trainer { trainer, battle } => {
                    if self.defeated_trainers.contains(&npc) {
                        return Err(GameError::TrainerAlreadyDefeated(npc));
                    }
                    let trainer_definition = content
                        .trainer(trainer)
                        .ok_or_else(|| GameError::UnknownTrainer(trainer.clone()))?;
                    self.require_party()?;
                    self.start_battle(content, battle.clone(), None, Some(trainer.clone()))?;
                    self.last_message = Some(trainer_definition.script().to_owned());
                    return Ok(GameEvent::TrainerBattleStarted {
                        battle: battle.clone(),
                        trainer: trainer.clone(),
                    });
                }
                NpcCapability::Merchant { shop } => {
                    return Ok(GameEvent::MerchantOpened { shop: shop.clone() });
                }
                NpcCapability::Guide => {}
            }
        }
        Err(GameError::InteractionUnavailable(npc))
    }

    fn move_player(
        &mut self,
        content: &ThinSliceContent,
        direction: Direction,
    ) -> Result<GameEvent, GameError> {
        if self.active_battle.is_some() {
            return Err(GameError::BattleAlreadyActive);
        }
        if self.pending_encounter.is_some() {
            return Err(GameError::EncounterPending);
        }
        let world = self.world(content)?;
        let (next, outcome) = world.transition(WorldCommand::Move(world_direction(direction)));
        self.position = from_world_position(next.player());
        self.facing = from_world_direction(next.facing());
        match outcome.event() {
            WorldEvent::EncounterTriggered { .. } => {
                self.pending_encounter = Some(self.position);
                Ok(GameEvent::EncounterAvailable {
                    position: self.position,
                })
            }
            WorldEvent::Moved { .. } | WorldEvent::Turned { .. } => Ok(GameEvent::Moved {
                position: self.position,
            }),
            WorldEvent::Blocked { .. } | WorldEvent::BlockedByActor { .. } => {
                Ok(GameEvent::MovementBlocked {
                    position: self.position,
                })
            }
            WorldEvent::TransitionRejected { error } => Err(GameError::World(error)),
        }
    }

    fn warp(&mut self, content: &ThinSliceContent, warp: WarpId) -> Result<GameEvent, GameError> {
        if self.active_battle.is_some() {
            return Err(GameError::BattleAlreadyActive);
        }
        self.require_party()?;
        let definition = content
            .warp(&warp)
            .ok_or_else(|| GameError::UnknownWarp(warp.clone()))?;
        self.require_map(definition.from_map())?;
        if content.map(definition.to_map()).is_none() {
            return Err(GameError::MapMissing(definition.to_map().clone()));
        }
        self.map = definition.to_map().clone();
        self.position = definition.destination();
        self.pending_encounter = None;
        self.world(content)?;
        Ok(GameEvent::Warped {
            map: self.map.clone(),
            position: self.position,
        })
    }

    fn encounter(&mut self, content: &ThinSliceContent, roll: u8) -> Result<GameEvent, GameError> {
        self.require_party()?;
        if roll > 99 {
            return Err(GameError::InvalidEncounterRoll(roll));
        }
        self.pending_encounter
            .ok_or(GameError::EncounterUnavailable {
                map: self.map.clone(),
            })?;
        let battle = content
            .encounter_battle(&self.map)
            .ok_or_else(|| GameError::EncounterUnavailable {
                map: self.map.clone(),
            })?
            .clone();
        self.start_battle(content, battle.clone(), Some(roll), None)?;
        self.pending_encounter = None;
        self.encounter_history.push(roll);
        Ok(GameEvent::EncounterStarted { battle, roll })
    }

    fn start_battle(
        &mut self,
        content: &ThinSliceContent,
        battle: BattleId,
        encounter_roll: Option<u8>,
        trainer: Option<TrainerId>,
    ) -> Result<(), GameError> {
        if self.active_battle.is_some() {
            return Err(GameError::BattleAlreadyActive);
        }
        if content.battle(&battle).is_none() {
            return Err(GameError::UnknownBattle(battle));
        }
        if let Some(trainer) = &trainer
            && content.trainer(trainer).is_none()
        {
            return Err(GameError::UnknownTrainer(trainer.clone()));
        }
        let participant = self
            .party
            .first()
            .map(CreatureState::id)
            .cloned()
            .ok_or(GameError::PartyRequired)?;
        self.active_battle = Some(ActiveBattle {
            battle,
            participant,
            encounter_roll,
            trainer,
        });
        Ok(())
    }

    fn resolve_battle(
        &mut self,
        content: &ThinSliceContent,
        outcome: BattleOutcome,
        hp: u16,
        pp: u8,
    ) -> Result<GameEvent, GameError> {
        let active = self.active_battle.take().ok_or(GameError::BattleMissing)?;
        let definition = content
            .battle(&active.battle)
            .ok_or_else(|| GameError::UnknownBattle(active.battle.clone()))?;
        let participant = self
            .party
            .iter_mut()
            .find(|creature| creature.id() == &active.participant)
            .ok_or(GameError::PartyRequired)?;
        let template = content
            .creature(participant.template())
            .ok_or_else(|| GameError::UnknownCreatureTemplate(participant.template().clone()))?;
        if hp > template.max_hp() || pp > template.max_pp() {
            return Err(GameError::InvalidBattleState { hp, pp });
        }
        let (experience, money) = match outcome {
            BattleOutcome::Victory => (definition.experience_reward(), definition.money_reward()),
            BattleOutcome::Defeat => (0, Money::new(0)),
        };
        participant.hp = hp;
        participant.pp = pp;
        participant.experience = participant
            .experience
            .checked_add(experience)
            .ok_or(GameError::ExperienceOverflow)?;
        self.money = self.money.credit(money)?;
        if outcome == BattleOutcome::Victory
            && let Some(trainer) = definition.trainer()
        {
            self.defeated_trainers.insert(trainer.clone());
        }
        Ok(GameEvent::BattleResolved {
            battle: active.battle,
            outcome,
            experience,
            money,
        })
    }

    fn buy(
        &mut self,
        content: &ThinSliceContent,
        npc: NpcId,
        item: ItemId,
        quantity: u16,
    ) -> Result<GameEvent, GameError> {
        let merchant = content
            .npc(&npc)
            .ok_or_else(|| GameError::UnknownNpc(npc.clone()))?;
        self.require_map(merchant.actor().map())?;
        self.require_interaction_position(merchant.actor().position(), npc.clone())?;
        let shop = merchant
            .capabilities()
            .iter()
            .find_map(|capability| match capability {
                NpcCapability::Merchant { shop } => Some(shop),
                _ => None,
            })
            .ok_or(GameError::InteractionUnavailable(npc))?;
        let shop = content
            .shop(shop)
            .ok_or_else(|| GameError::UnknownShop(shop.clone()))?;
        let listing = shop
            .listing(&item)
            .ok_or_else(|| GameError::UnknownItem(item.clone()))?;
        let item_definition = content
            .item(&item)
            .ok_or_else(|| GameError::UnknownItem(item.clone()))?;
        let spent = listing.total_price(quantity)?;
        self.money = self.money.spend(spent)?;
        self.inventory.add(item_definition, quantity)?;
        Ok(GameEvent::ItemPurchased {
            item,
            quantity,
            spent,
        })
    }

    fn require_map(&self, expected: &MapId) -> Result<(), GameError> {
        if self.map == *expected {
            Ok(())
        } else {
            Err(GameError::WrongMap {
                expected: expected.clone(),
                actual: self.map.clone(),
            })
        }
    }

    fn require_party(&self) -> Result<(), GameError> {
        if self.party.is_empty() {
            Err(GameError::PartyRequired)
        } else {
            Ok(())
        }
    }

    fn require_interaction_position(
        &self,
        position: Position,
        npc: NpcId,
    ) -> Result<(), GameError> {
        let horizontal = self.position.x().abs_diff(position.x());
        let vertical = self.position.y().abs_diff(position.y());
        let facing_npc = match self.facing {
            Direction::Up => vertical == 1 && horizontal == 0 && position.y() < self.position.y(),
            Direction::Down => vertical == 1 && horizontal == 0 && position.y() > self.position.y(),
            Direction::Left => horizontal == 1 && vertical == 0 && position.x() < self.position.x(),
            Direction::Right => {
                horizontal == 1 && vertical == 0 && position.x() > self.position.x()
            }
        };
        if facing_npc {
            Ok(())
        } else {
            Err(GameError::InteractionUnavailable(npc))
        }
    }

    fn world(&self, content: &ThinSliceContent) -> Result<World, GameError> {
        let map = content
            .map(&self.map)
            .ok_or_else(|| GameError::MapMissing(self.map.clone()))?;
        let actors = content
            .npcs_on_map(&self.map)
            .map(|npc| {
                let actor = npc.actor();
                WorldActorId::new(actor.id().as_str())
                    .map_err(GameError::from)
                    .map(|id| {
                        WorldActor::new(
                            id,
                            world_position(actor.position()),
                            WorldDirection::Down,
                            actor.blocks_movement(),
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        World::with_actors(
            map.layout().clone(),
            world_position(self.position),
            world_direction(self.facing),
            actors,
        )
        .map_err(GameError::from)
    }
}

fn world_position(position: Position) -> world_domain::Position {
    world_domain::Position::new(position.x(), position.y())
}

fn from_world_position(position: world_domain::Position) -> Position {
    Position::new(position.x(), position.y())
}

const fn world_direction(direction: Direction) -> WorldDirection {
    match direction {
        Direction::Up => WorldDirection::Up,
        Direction::Down => WorldDirection::Down,
        Direction::Left => WorldDirection::Left,
        Direction::Right => WorldDirection::Right,
    }
}

const fn from_world_direction(direction: WorldDirection) -> Direction {
    match direction {
        WorldDirection::Up => Direction::Up,
        WorldDirection::Down => Direction::Down,
        WorldDirection::Left => Direction::Left,
        WorldDirection::Right => Direction::Right,
    }
}
