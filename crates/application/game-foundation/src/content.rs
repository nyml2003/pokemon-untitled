use std::collections::BTreeMap;

use crate::{
    BattleId, CreatureTemplateId, EconomyError, EventFlagId, ItemCategory, ItemDefinition, ItemId,
    MapId, Money, NpcId, Position, ShopId, ShopListing, TrainerCatalog, TrainerDefinition,
    TrainerError, TrainerId, WarpId,
};
use world_domain::{Tile, TileMap, WorldError};

pub const CONTENT_VERSION: &str = "thin-slice-v3";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapDefinition {
    id: MapId,
    spawn: Position,
    layout: TileMap,
}

impl MapDefinition {
    pub fn new(id: MapId, spawn: Position, layout: TileMap) -> Result<Self, ContentError> {
        let spawn_tile =
            layout
                .tile(world_position(spawn))
                .ok_or_else(|| ContentError::InvalidMapSpawn {
                    map: id.clone(),
                    spawn,
                })?;
        if !spawn_tile.is_walkable() {
            return Err(ContentError::InvalidMapSpawn { map: id, spawn });
        }
        Ok(Self { id, spawn, layout })
    }

    pub fn id(&self) -> &MapId {
        &self.id
    }

    pub const fn spawn(&self) -> Position {
        self.spawn
    }

    pub fn layout(&self) -> &TileMap {
        &self.layout
    }

    pub fn tile(&self, position: Position) -> Option<Tile> {
        self.layout.tile(world_position(position))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WarpDefinition {
    id: WarpId,
    from_map: MapId,
    to_map: MapId,
    destination: Position,
}

impl WarpDefinition {
    pub fn new(id: WarpId, from_map: MapId, to_map: MapId, destination: Position) -> Self {
        Self {
            id,
            from_map,
            to_map,
            destination,
        }
    }

    pub fn id(&self) -> &WarpId {
        &self.id
    }

    pub fn from_map(&self) -> &MapId {
        &self.from_map
    }

    pub fn to_map(&self) -> &MapId {
        &self.to_map
    }

    pub const fn destination(&self) -> Position {
        self.destination
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorDefinition {
    id: NpcId,
    map: MapId,
    position: Position,
    blocks_movement: bool,
}

impl ActorDefinition {
    pub fn new(id: NpcId, map: MapId, position: Position, blocks_movement: bool) -> Self {
        Self {
            id,
            map,
            position,
            blocks_movement,
        }
    }

    pub fn id(&self) -> &NpcId {
        &self.id
    }

    pub fn map(&self) -> &MapId {
        &self.map
    }

    pub const fn position(&self) -> Position {
        self.position
    }

    pub const fn blocks_movement(&self) -> bool {
        self.blocks_movement
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NpcCapability {
    Gift {
        claimed_flag: EventFlagId,
        creature: CreatureTemplateId,
        item: ItemId,
        quantity: u16,
    },
    Trainer {
        trainer: TrainerId,
        battle: BattleId,
    },
    Merchant {
        shop: ShopId,
    },
    Guide,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NpcDefinition {
    actor: ActorDefinition,
    capabilities: Vec<NpcCapability>,
}

impl NpcDefinition {
    pub fn new(actor: ActorDefinition, capabilities: Vec<NpcCapability>) -> Self {
        Self {
            actor,
            capabilities,
        }
    }

    pub fn actor(&self) -> &ActorDefinition {
        &self.actor
    }

    pub fn capabilities(&self) -> &[NpcCapability] {
        &self.capabilities
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShopDefinition {
    id: ShopId,
    listings: BTreeMap<ItemId, ShopListing>,
}

impl ShopDefinition {
    pub fn new(id: ShopId, listings: Vec<ShopListing>) -> Result<Self, ContentError> {
        let mut entries = BTreeMap::new();
        for listing in listings {
            if entries.insert(listing.item().clone(), listing).is_some() {
                return Err(ContentError::DuplicateShopListing { shop: id });
            }
        }
        Ok(Self {
            id,
            listings: entries,
        })
    }

    pub fn id(&self) -> &ShopId {
        &self.id
    }

    pub fn listing(&self, item: &ItemId) -> Option<&ShopListing> {
        self.listings.get(item)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BattleDefinition {
    id: BattleId,
    experience_reward: u32,
    money_reward: Money,
    trainer: Option<NpcId>,
    wild_opponent: Option<WildOpponentDefinition>,
}

impl BattleDefinition {
    pub fn new(
        id: BattleId,
        experience_reward: u32,
        money_reward: Money,
        trainer: Option<NpcId>,
    ) -> Self {
        Self {
            id,
            experience_reward,
            money_reward,
            trainer,
            wild_opponent: None,
        }
    }

    pub fn wild(
        id: BattleId,
        experience_reward: u32,
        money_reward: Money,
        opponent: WildOpponentDefinition,
    ) -> Self {
        Self {
            id,
            experience_reward,
            money_reward,
            trainer: None,
            wild_opponent: Some(opponent),
        }
    }

    pub fn id(&self) -> &BattleId {
        &self.id
    }

    pub const fn experience_reward(&self) -> u32 {
        self.experience_reward
    }

    pub const fn money_reward(&self) -> Money {
        self.money_reward
    }

    pub fn trainer(&self) -> Option<&NpcId> {
        self.trainer.as_ref()
    }

    pub fn wild_opponent(&self) -> Option<&WildOpponentDefinition> {
        self.wild_opponent.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WildOpponentDefinition {
    species: String,
    level: u8,
}

impl WildOpponentDefinition {
    pub fn new(species: impl Into<String>, level: u8) -> Result<Self, ContentError> {
        let species = species.into();
        if species.trim().is_empty() || !(1..=100).contains(&level) {
            return Err(ContentError::InvalidWildOpponent { species, level });
        }
        Ok(Self { species, level })
    }

    pub fn species(&self) -> &str {
        &self.species
    }

    pub const fn level(&self) -> u8 {
        self.level
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreatureTemplate {
    id: CreatureTemplateId,
    species: String,
    max_hp: u16,
    max_pp: u8,
}

impl CreatureTemplate {
    pub fn new(
        id: CreatureTemplateId,
        species: impl Into<String>,
        max_hp: u16,
        max_pp: u8,
    ) -> Result<Self, ContentError> {
        let species = species.into();
        if species.trim().is_empty() || max_hp == 0 || max_pp == 0 {
            return Err(ContentError::InvalidCreatureTemplate { id });
        }
        Ok(Self {
            id,
            species,
            max_hp,
            max_pp,
        })
    }

    pub fn id(&self) -> &CreatureTemplateId {
        &self.id
    }

    pub fn species(&self) -> &str {
        &self.species
    }

    pub const fn max_hp(&self) -> u16 {
        self.max_hp
    }

    pub const fn max_pp(&self) -> u8 {
        self.max_pp
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThinSliceContent {
    maps: BTreeMap<MapId, MapDefinition>,
    warps: BTreeMap<WarpId, WarpDefinition>,
    npcs: BTreeMap<NpcId, NpcDefinition>,
    items: BTreeMap<ItemId, ItemDefinition>,
    shops: BTreeMap<ShopId, ShopDefinition>,
    battles: BTreeMap<BattleId, BattleDefinition>,
    creatures: BTreeMap<CreatureTemplateId, CreatureTemplate>,
    trainers: BTreeMap<TrainerId, TrainerDefinition>,
    encounters: BTreeMap<MapId, BattleId>,
    starting_map: MapId,
    starting_money: Money,
    inventory_capacity: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentDefinitions {
    pub maps: Vec<MapDefinition>,
    pub warps: Vec<WarpDefinition>,
    pub npcs: Vec<NpcDefinition>,
    pub items: Vec<ItemDefinition>,
    pub shops: Vec<ShopDefinition>,
    pub battles: Vec<BattleDefinition>,
    pub creatures: Vec<CreatureTemplate>,
    pub trainers: Vec<TrainerDefinition>,
    pub encounters: Vec<(MapId, BattleId)>,
    pub starting_map: MapId,
    pub starting_money: Money,
    pub inventory_capacity: u16,
}

impl ThinSliceContent {
    pub fn from_definitions(definitions: ContentDefinitions) -> Result<Self, ContentError> {
        let ContentDefinitions {
            maps,
            warps,
            npcs,
            items,
            shops,
            battles,
            creatures,
            trainers,
            encounters,
            starting_map,
            starting_money,
            inventory_capacity,
        } = definitions;
        let content = Self {
            maps: map_by_id(maps)?,
            warps: map_by_id(warps)?,
            npcs: map_by_id(npcs)?,
            items: map_by_id(items)?,
            shops: map_by_id(shops)?,
            battles: map_by_id(battles)?,
            creatures: map_by_id(creatures)?,
            trainers: map_by_id(trainers)?,
            encounters: encounters.into_iter().collect(),
            starting_map,
            starting_money,
            inventory_capacity,
        };
        content.validate_references()?;
        Ok(content)
    }

    pub fn standard() -> Result<Self, ContentError> {
        let town = MapId::new("starting-town")?;
        let route = MapId::new("verdant-route")?;
        let professor = NpcId::new("professor")?;
        let trainer = NpcId::new("route-trainer")?;
        let trainer_profile = TrainerId::new("route-rival")?;
        let merchant = NpcId::new("merchant")?;
        let potion = ItemId::new("potion")?;
        let starter = CreatureTemplateId::new("starter-treecko")?;
        let gift_flag = EventFlagId::new("professor-gift-claimed")?;
        let wild_battle = BattleId::new("route-wild")?;
        let trainer_battle = BattleId::new("route-trainer-battle")?;
        let shop_id = ShopId::new("town-mart")?;

        let town_layout = TileMap::new(6, 4, vec![Tile::Ground; 24])?;
        let route_layout = TileMap::new(
            6,
            4,
            (0..24)
                .map(|index| {
                    if index == 8 {
                        Tile::Grass
                    } else {
                        Tile::Ground
                    }
                })
                .collect(),
        )?;
        let maps = map_by_id(vec![
            MapDefinition::new(town.clone(), Position::new(2, 2), town_layout)?,
            MapDefinition::new(route.clone(), Position::new(1, 1), route_layout)?,
        ])?;
        let warps = map_by_id(vec![
            WarpDefinition::new(
                WarpId::new("town-to-route")?,
                town.clone(),
                route.clone(),
                Position::new(1, 1),
            ),
            WarpDefinition::new(
                WarpId::new("route-to-town")?,
                route.clone(),
                town.clone(),
                Position::new(3, 2),
            ),
        ])?;
        let items = map_by_id(vec![ItemDefinition::new(
            potion.clone(),
            ItemCategory::Medicine,
            99,
        )?])?;
        let creatures = map_by_id(vec![CreatureTemplate::new(
            starter.clone(),
            "Treecko",
            35,
            35,
        )?])?;
        let trainers = map_by_id(TrainerCatalog::standard()?.trainers().to_vec())?;
        let battles = map_by_id(vec![
            BattleDefinition::wild(
                wild_battle.clone(),
                20,
                Money::new(0),
                WildOpponentDefinition::new("Torchic", 3)?,
            ),
            BattleDefinition::new(
                trainer_battle.clone(),
                45,
                Money::new(120),
                Some(trainer.clone()),
            ),
        ])?;
        let shops = map_by_id(vec![ShopDefinition::new(
            shop_id.clone(),
            vec![ShopListing::new(potion.clone(), Money::new(30))],
        )?])?;
        let npcs = map_by_id(vec![
            NpcDefinition::new(
                ActorDefinition::new(professor.clone(), town.clone(), Position::new(2, 1), true),
                vec![NpcCapability::Gift {
                    claimed_flag: gift_flag,
                    creature: starter,
                    item: potion,
                    quantity: 1,
                }],
            ),
            NpcDefinition::new(
                ActorDefinition::new(trainer.clone(), route.clone(), Position::new(4, 2), true),
                vec![NpcCapability::Trainer {
                    trainer: trainer_profile,
                    battle: trainer_battle,
                }],
            ),
            NpcDefinition::new(
                ActorDefinition::new(merchant, town.clone(), Position::new(3, 1), true),
                vec![NpcCapability::Merchant { shop: shop_id }],
            ),
        ])?;

        Ok(Self {
            maps,
            warps,
            npcs,
            items,
            shops,
            battles,
            creatures,
            trainers,
            encounters: BTreeMap::from([(route.clone(), wild_battle)]),
            starting_map: town,
            starting_money: Money::new(200),
            inventory_capacity: 20,
        })
    }

    pub const fn content_version(&self) -> &'static str {
        CONTENT_VERSION
    }

    pub fn starting_map(&self) -> &MapId {
        &self.starting_map
    }
    pub const fn starting_money(&self) -> Money {
        self.starting_money
    }
    pub const fn inventory_capacity(&self) -> u16 {
        self.inventory_capacity
    }
    pub fn map(&self, id: &MapId) -> Option<&MapDefinition> {
        self.maps.get(id)
    }
    pub fn warp(&self, id: &WarpId) -> Option<&WarpDefinition> {
        self.warps.get(id)
    }
    pub fn npc(&self, id: &NpcId) -> Option<&NpcDefinition> {
        self.npcs.get(id)
    }
    pub fn npcs_on_map(&self, map: &MapId) -> impl Iterator<Item = &NpcDefinition> {
        self.npcs
            .values()
            .filter(move |npc| npc.actor().map() == map)
    }
    pub fn item(&self, id: &ItemId) -> Option<&ItemDefinition> {
        self.items.get(id)
    }
    pub fn shop(&self, id: &ShopId) -> Option<&ShopDefinition> {
        self.shops.get(id)
    }
    pub fn battle(&self, id: &BattleId) -> Option<&BattleDefinition> {
        self.battles.get(id)
    }
    pub fn creature(&self, id: &CreatureTemplateId) -> Option<&CreatureTemplate> {
        self.creatures.get(id)
    }
    pub fn creatures(&self) -> impl Iterator<Item = &CreatureTemplate> {
        self.creatures.values()
    }
    pub fn trainer(&self, id: &TrainerId) -> Option<&TrainerDefinition> {
        self.trainers.get(id)
    }
    pub fn trainers(&self) -> impl Iterator<Item = &TrainerDefinition> {
        self.trainers.values()
    }
    pub fn battles(&self) -> impl Iterator<Item = &BattleDefinition> {
        self.battles.values()
    }
    pub fn encounter_battle(&self, map: &MapId) -> Option<&BattleId> {
        self.encounters.get(map)
    }

    pub fn has_event_flag(&self, flag: &EventFlagId) -> bool {
        self.npcs.values().any(|npc| {
            npc.capabilities().iter().any(|capability| {
                matches!(
                    capability,
                    NpcCapability::Gift { claimed_flag, .. } if claimed_flag == flag
                )
            })
        })
    }

    pub fn with_trainer_catalog(mut self, catalog: TrainerCatalog) -> Result<Self, ContentError> {
        let trainers = map_by_id(catalog.trainers().to_vec())?;
        for npc in self.npcs.values() {
            for capability in npc.capabilities() {
                if let NpcCapability::Trainer { trainer, .. } = capability
                    && !trainers.contains_key(trainer)
                {
                    return Err(ContentError::MissingTrainerDefinition {
                        npc: npc.actor().id().clone(),
                        trainer: trainer.clone(),
                    });
                }
            }
        }
        self.trainers = trainers;
        self.validate_references()?;
        Ok(self)
    }

    fn validate_references(&self) -> Result<(), ContentError> {
        if self.maps.is_empty() {
            return Err(ContentError::EmptyMaps);
        }
        if !self.maps.contains_key(&self.starting_map) {
            return Err(ContentError::MissingStartingMap(self.starting_map.clone()));
        }
        if self.inventory_capacity == 0 {
            return Err(ContentError::ZeroInventoryCapacity);
        }
        for warp in self.warps.values() {
            let Some(destination_map) = self.maps.get(warp.to_map()) else {
                return Err(ContentError::MissingMapReference {
                    owner: warp.id().as_str().to_owned(),
                    map: warp.to_map().clone(),
                });
            };
            if !self.maps.contains_key(warp.from_map()) {
                return Err(ContentError::MissingMapReference {
                    owner: warp.id().as_str().to_owned(),
                    map: warp.from_map().clone(),
                });
            }
            if !destination_map
                .tile(warp.destination())
                .is_some_and(Tile::is_walkable)
            {
                return Err(ContentError::InvalidWarpDestination {
                    warp: warp.id().clone(),
                    map: warp.to_map().clone(),
                    destination: warp.destination(),
                });
            }
        }
        for npc in self.npcs.values() {
            let actor = npc.actor();
            let Some(map) = self.maps.get(actor.map()) else {
                return Err(ContentError::MissingMapReference {
                    owner: actor.id().as_str().to_owned(),
                    map: actor.map().clone(),
                });
            };
            if !map.tile(actor.position()).is_some_and(Tile::is_walkable) {
                return Err(ContentError::InvalidNpcPosition {
                    npc: actor.id().clone(),
                    map: actor.map().clone(),
                    position: actor.position(),
                });
            }
            for capability in npc.capabilities() {
                match capability {
                    NpcCapability::Gift { creature, item, .. } => {
                        if !self.creatures.contains_key(creature) {
                            return Err(ContentError::MissingContentReference {
                                owner: actor.id().as_str().to_owned(),
                                reference: creature.as_str().to_owned(),
                            });
                        }
                        if !self.items.contains_key(item) {
                            return Err(ContentError::MissingContentReference {
                                owner: actor.id().as_str().to_owned(),
                                reference: item.as_str().to_owned(),
                            });
                        }
                    }
                    NpcCapability::Trainer { trainer, battle } => {
                        if !self.trainers.contains_key(trainer) {
                            return Err(ContentError::MissingTrainerDefinition {
                                npc: actor.id().clone(),
                                trainer: trainer.clone(),
                            });
                        }
                        if !self.battles.contains_key(battle) {
                            return Err(ContentError::MissingContentReference {
                                owner: actor.id().as_str().to_owned(),
                                reference: battle.as_str().to_owned(),
                            });
                        }
                    }
                    NpcCapability::Merchant { shop } => {
                        if !self.shops.contains_key(shop) {
                            return Err(ContentError::MissingContentReference {
                                owner: actor.id().as_str().to_owned(),
                                reference: shop.as_str().to_owned(),
                            });
                        }
                    }
                    NpcCapability::Guide => {}
                }
            }
        }
        for shop in self.shops.values() {
            for listing in shop.listings.values() {
                if !self.items.contains_key(listing.item()) {
                    return Err(ContentError::MissingContentReference {
                        owner: shop.id().as_str().to_owned(),
                        reference: listing.item().as_str().to_owned(),
                    });
                }
            }
        }
        for battle in self.battles.values() {
            if let Some(trainer) = battle.trainer()
                && !self.npcs.contains_key(trainer)
            {
                return Err(ContentError::MissingContentReference {
                    owner: battle.id().as_str().to_owned(),
                    reference: trainer.as_str().to_owned(),
                });
            }
        }
        for (map, battle) in &self.encounters {
            if !self.maps.contains_key(map) {
                return Err(ContentError::MissingMapReference {
                    owner: battle.as_str().to_owned(),
                    map: map.clone(),
                });
            }
            if !self.battles.contains_key(battle) {
                return Err(ContentError::MissingContentReference {
                    owner: map.as_str().to_owned(),
                    reference: battle.as_str().to_owned(),
                });
            }
        }
        Ok(())
    }
}

fn world_position(position: Position) -> world_domain::Position {
    world_domain::Position::new(position.x(), position.y())
}

trait ContentIdentity {
    type Id: Ord + Clone;
    fn content_id(&self) -> &Self::Id;
}

impl ContentIdentity for MapDefinition {
    type Id = MapId;
    fn content_id(&self) -> &Self::Id {
        self.id()
    }
}
impl ContentIdentity for WarpDefinition {
    type Id = WarpId;
    fn content_id(&self) -> &Self::Id {
        self.id()
    }
}
impl ContentIdentity for NpcDefinition {
    type Id = NpcId;
    fn content_id(&self) -> &Self::Id {
        self.actor().id()
    }
}
impl ContentIdentity for ItemDefinition {
    type Id = ItemId;
    fn content_id(&self) -> &Self::Id {
        self.id()
    }
}
impl ContentIdentity for ShopDefinition {
    type Id = ShopId;
    fn content_id(&self) -> &Self::Id {
        self.id()
    }
}
impl ContentIdentity for BattleDefinition {
    type Id = BattleId;
    fn content_id(&self) -> &Self::Id {
        self.id()
    }
}
impl ContentIdentity for CreatureTemplate {
    type Id = CreatureTemplateId;
    fn content_id(&self) -> &Self::Id {
        self.id()
    }
}
impl ContentIdentity for TrainerDefinition {
    type Id = TrainerId;
    fn content_id(&self) -> &Self::Id {
        self.id()
    }
}

fn map_by_id<T: ContentIdentity>(values: Vec<T>) -> Result<BTreeMap<T::Id, T>, ContentError> {
    let mut mapped = BTreeMap::new();
    for value in values {
        let id = value.content_id().clone();
        if mapped.insert(id, value).is_some() {
            return Err(ContentError::DuplicateContentId);
        }
    }
    Ok(mapped)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContentError {
    Id(crate::GameIdError),
    Economy(EconomyError),
    Trainer(TrainerError),
    World(WorldError),
    DuplicateContentId,
    EmptyMaps,
    MissingStartingMap(MapId),
    ZeroInventoryCapacity,
    MissingMapReference {
        owner: String,
        map: MapId,
    },
    InvalidWarpDestination {
        warp: WarpId,
        map: MapId,
        destination: Position,
    },
    InvalidNpcPosition {
        npc: NpcId,
        map: MapId,
        position: Position,
    },
    MissingContentReference {
        owner: String,
        reference: String,
    },
    DuplicateShopListing {
        shop: ShopId,
    },
    InvalidMapSpawn {
        map: MapId,
        spawn: Position,
    },
    InvalidCreatureTemplate {
        id: CreatureTemplateId,
    },
    InvalidWildOpponent {
        species: String,
        level: u8,
    },
    MissingTrainerDefinition {
        npc: NpcId,
        trainer: TrainerId,
    },
}

impl From<crate::GameIdError> for ContentError {
    fn from(value: crate::GameIdError) -> Self {
        Self::Id(value)
    }
}
impl From<EconomyError> for ContentError {
    fn from(value: EconomyError) -> Self {
        Self::Economy(value)
    }
}
impl From<TrainerError> for ContentError {
    fn from(value: TrainerError) -> Self {
        Self::Trainer(value)
    }
}
impl From<WorldError> for ContentError {
    fn from(value: WorldError) -> Self {
        Self::World(value)
    }
}
