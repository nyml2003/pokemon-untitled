//! Pure state transitions and save-format rules for the playable foundation slice.

#![forbid(unsafe_code)]

mod content;
mod economy;
mod id;
mod package;
mod package_document;
mod save;
mod state;
mod trainer;

pub use content::{
    ActorDefinition, BattleDefinition, CONTENT_VERSION, ContentDefinitions, ContentError,
    CreatureTemplate, MapDefinition, NpcCapability, NpcDefinition, ShopDefinition,
    ThinSliceContent, WarpDefinition, WildOpponentDefinition,
};
pub use economy::{EconomyError, Inventory, ItemCategory, ItemDefinition, Money, ShopListing};
pub use id::{
    BattleId, ContentPackageId, CreatureId, CreatureTemplateId, EventFlagId, GameIdError, ItemId,
    MapId, MoveId, NpcId, ShopId, TrainerId, WarpId,
};
pub use package::{
    ContentPackage, ContentPackageError, ContentPackageManifest, EMBEDDED_GEN3_DATA_REFERENCE,
    LEGACY_GEN3_RULESET_REFERENCE, STARTER_REGION_PACKAGE_ID, STARTER_REGION_PACKAGE_REVISION,
};
pub use package_document::ContentPackageDocument;
pub use save::{SaveEnvelope, SaveError};
pub use state::{
    ActiveBattle, BattleOutcome, BattleResolution, CreatureState, Direction, GameCommand,
    GameError, GameEvent, GameState, Position,
};
pub use trainer::{
    TRAINER_CATALOG_FORMAT, TrainerCatalog, TrainerDefinition, TrainerEditCommand, TrainerError,
    TrainerPokemon,
};

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
