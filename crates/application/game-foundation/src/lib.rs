//! Pure state transitions and save-format rules for the playable foundation slice.

#![forbid(unsafe_code)]

mod content;
mod economy;
mod id;
mod save;
mod state;
mod trainer;

pub use content::{
    ActorDefinition, BattleDefinition, CONTENT_VERSION, ContentError, CreatureTemplate,
    MapDefinition, NpcCapability, NpcDefinition, ShopDefinition, ThinSliceContent, WarpDefinition,
};
pub use economy::{EconomyError, Inventory, ItemCategory, ItemDefinition, Money, ShopListing};
pub use id::{
    BattleId, CreatureId, CreatureTemplateId, EventFlagId, GameIdError, ItemId, MapId, MoveId,
    NpcId, ShopId, TrainerId, WarpId,
};
pub use save::{SaveEnvelope, SaveError};
pub use state::{
    ActiveBattle, BattleOutcome, CreatureState, Direction, GameCommand, GameError, GameEvent,
    GameState, Position,
};
pub use trainer::{
    TRAINER_CATALOG_FORMAT, TrainerCatalog, TrainerDefinition, TrainerEditCommand, TrainerError,
    TrainerPokemon,
};

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
