use serde::Deserialize;
use world_domain::{Tile, TileMap};

use crate::{
    ActorDefinition, BattleDefinition, BattleId, ContentDefinitions, ContentError, ContentPackage,
    ContentPackageError, ContentPackageManifest, CreatureTemplate, CreatureTemplateId, EventFlagId,
    ItemDefinition, ItemId, MapDefinition, MapId, Money, NpcCapability, NpcDefinition, NpcId,
    Position, ShopDefinition, ShopId, ShopListing, ThinSliceContent, TrainerDefinition, TrainerId,
    WarpDefinition, WarpId, WildOpponentDefinition,
};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ContentPackageDocument {
    manifest: ContentPackageManifest,
    content: ContentDocument,
}

impl ContentPackageDocument {
    pub fn from_json(json: &str) -> Result<Self, ContentPackageError> {
        serde_json::from_str(json).map_err(|error| ContentPackageError::Json(error.to_string()))
    }

    pub fn into_package(self) -> Result<ContentPackage, ContentPackageError> {
        let content = self.content.into_content()?;
        ContentPackage::new(self.manifest, content)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct ContentDocument {
    maps: Vec<MapDocument>,
    warps: Vec<WarpDocument>,
    npcs: Vec<NpcDocument>,
    items: Vec<ItemDefinition>,
    shops: Vec<ShopDocument>,
    battles: Vec<BattleDocument>,
    creatures: Vec<CreatureTemplateDocument>,
    trainers: Vec<TrainerDefinition>,
    encounters: Vec<EncounterDocument>,
    starting_map: MapId,
    starting_money: Money,
    inventory_capacity: u16,
}

impl ContentDocument {
    fn into_content(self) -> Result<ThinSliceContent, ContentPackageError> {
        let maps = self
            .maps
            .into_iter()
            .map(MapDocument::into_definition)
            .collect::<Result<Vec<_>, _>>()?;
        let warps = self
            .warps
            .into_iter()
            .map(WarpDocument::into_definition)
            .collect();
        let npcs = self
            .npcs
            .into_iter()
            .map(NpcDocument::into_definition)
            .collect();
        let shops = self
            .shops
            .into_iter()
            .map(ShopDocument::into_definition)
            .collect::<Result<Vec<_>, _>>()?;
        let battles = self
            .battles
            .into_iter()
            .map(BattleDocument::into_definition)
            .collect::<Result<Vec<_>, _>>()?;
        let creatures = self
            .creatures
            .into_iter()
            .map(CreatureTemplateDocument::into_definition)
            .collect::<Result<Vec<_>, _>>()?;
        let encounters = self
            .encounters
            .into_iter()
            .map(|encounter| (encounter.map, encounter.battle))
            .collect();
        ThinSliceContent::from_definitions(ContentDefinitions {
            maps,
            warps,
            npcs,
            items: self.items,
            shops,
            battles,
            creatures,
            trainers: self.trainers,
            encounters,
            starting_map: self.starting_map,
            starting_money: self.starting_money,
            inventory_capacity: self.inventory_capacity,
        })
        .map_err(Into::into)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct MapDocument {
    id: MapId,
    spawn: Position,
    width: u16,
    height: u16,
    tiles: Vec<TileDocument>,
}

impl MapDocument {
    fn into_definition(self) -> Result<MapDefinition, ContentPackageError> {
        let layout = TileMap::new(
            self.width,
            self.height,
            self.tiles
                .into_iter()
                .map(TileDocument::into_tile)
                .collect(),
        )
        .map_err(ContentError::World)?;
        MapDefinition::new(self.id, self.spawn, layout).map_err(Into::into)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum TileDocument {
    Ground,
    Wall,
    Grass,
}

impl TileDocument {
    const fn into_tile(self) -> Tile {
        match self {
            Self::Ground => Tile::Ground,
            Self::Wall => Tile::Wall,
            Self::Grass => Tile::Grass,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct WarpDocument {
    id: WarpId,
    from_map: MapId,
    to_map: MapId,
    destination: Position,
}

impl WarpDocument {
    fn into_definition(self) -> WarpDefinition {
        WarpDefinition::new(self.id, self.from_map, self.to_map, self.destination)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct NpcDocument {
    id: NpcId,
    map: MapId,
    position: Position,
    blocks_movement: bool,
    capabilities: Vec<NpcCapabilityDocument>,
}

impl NpcDocument {
    fn into_definition(self) -> NpcDefinition {
        let actor = ActorDefinition::new(self.id, self.map, self.position, self.blocks_movement);
        NpcDefinition::new(
            actor,
            self.capabilities
                .into_iter()
                .map(NpcCapabilityDocument::into_capability)
                .collect(),
        )
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum NpcCapabilityDocument {
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

impl NpcCapabilityDocument {
    fn into_capability(self) -> NpcCapability {
        match self {
            Self::Gift {
                claimed_flag,
                creature,
                item,
                quantity,
            } => NpcCapability::Gift {
                claimed_flag,
                creature,
                item,
                quantity,
            },
            Self::Trainer { trainer, battle } => NpcCapability::Trainer { trainer, battle },
            Self::Merchant { shop } => NpcCapability::Merchant { shop },
            Self::Guide => NpcCapability::Guide,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct ShopDocument {
    id: ShopId,
    listings: Vec<ShopListing>,
}

impl ShopDocument {
    fn into_definition(self) -> Result<ShopDefinition, ContentPackageError> {
        ShopDefinition::new(self.id, self.listings).map_err(Into::into)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct BattleDocument {
    id: BattleId,
    experience_reward: u32,
    money_reward: Money,
    #[serde(default)]
    trainer: Option<NpcId>,
    #[serde(default)]
    wild_opponent: Option<WildOpponentDocument>,
}

impl BattleDocument {
    fn into_definition(self) -> Result<BattleDefinition, ContentPackageError> {
        match (self.trainer, self.wild_opponent) {
            (Some(trainer), None) => Ok(BattleDefinition::new(
                self.id,
                self.experience_reward,
                self.money_reward,
                Some(trainer),
            )),
            (None, Some(opponent)) => Ok(BattleDefinition::wild(
                self.id,
                self.experience_reward,
                self.money_reward,
                opponent.into_definition()?,
            )),
            _ => Err(ContentPackageError::InvalidBattleKind(self.id)),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct WildOpponentDocument {
    species: String,
    level: u8,
}

impl WildOpponentDocument {
    fn into_definition(self) -> Result<WildOpponentDefinition, ContentPackageError> {
        WildOpponentDefinition::new(self.species, self.level).map_err(Into::into)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct CreatureTemplateDocument {
    id: CreatureTemplateId,
    species: String,
    max_hp: u16,
    max_pp: u8,
}

impl CreatureTemplateDocument {
    fn into_definition(self) -> Result<CreatureTemplate, ContentPackageError> {
        CreatureTemplate::new(self.id, self.species, self.max_hp, self.max_pp).map_err(Into::into)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct EncounterDocument {
    map: MapId,
    battle: BattleId,
}
