//! 将地图编辑器模型投影为可渲染的工作台视图。
//!
//! 该 crate 只读取 `EditorModel` 和资源目录并生成 `EditorFrame`。
//! 编辑意图仍由调用方交给应用层处理；这里不保存编辑状态，也不访问文件系统。

#![forbid(unsafe_code)]

mod workbench;

pub use workbench::{
    EditorFrame, EditorViewError, centered_map_viewport, editor_viewport, intent_for_ui_hit,
    project,
};
