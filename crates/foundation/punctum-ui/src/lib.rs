//! 构建并解析与渲染器无关的像素 UI 树。
//!
//! 该 crate 定义 UI 数据、校验树结构，并将树转换为绘制命令和命中区域。
//! 它不持有应用状态、不加载资源，也不调用渲染器。

#![forbid(unsafe_code)]

mod error;
mod layout;
mod model;
mod tree;

pub use error::{UiBuildError, UiLayoutError};
pub use layout::{UiActionHit, UiDrawCommand, UiFrame, UiHitRegion};
pub use model::{
    CrossAlign, Dimension, FlexDirection, Insets, MainAlign, Position, UiBorder, UiBorderRadius,
    UiColor, UiContent, UiContentId, UiId, UiKey, UiPixelOffset, UiRect, UiSize, UiStyle,
    UiTextSize,
};
pub use tree::{UiNode, UiTree};

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
