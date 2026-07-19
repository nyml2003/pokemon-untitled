//! 定义并执行分层地图图块的语义规则。
//!
//! 该 crate 解析并校验完整图块规则目录，再为 `map_project::MapProject` 返回诊断信息。
//! 它是确定且无副作用的：调用方提供序列化数据和地图状态，适配层负责文件访问。

#![forbid(unsafe_code)]

mod catalog;
mod diagnostic;
mod model;

pub use diagnostic::{MapSemanticDiagnostic, SemanticRuleLocation, TileSemanticsError};
pub use model::{
    CellRequirement, Direction8, LayerScope, NeighbourRule, Neighbours8, PatternCoord,
    PatternDefinition, PatternId, PatternPart, StackRule, TileDefinition, TileHardRules,
    TileMatcher, TileSemanticsCatalog, TileStatus, TileTag,
};

/// `TileSemanticsCatalog::validate` 唯一接受的图块语义 JSON 格式。
pub const FORMAT_VERSION: &str = "map-tile-semantics-v1";
