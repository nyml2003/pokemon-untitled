//! Pure owner of map-editor state, intent routing, and layout.

#![forbid(unsafe_code)]

mod controller;
mod input;
pub mod layout;
mod model;
mod virtual_command;

pub use controller::{EditorController, PointerButton};
pub use input::{key_intent, wheel_intent};
pub use model::{EditorEffect, EditorIntent, EditorModel, EditorTool, tool_name};
pub use virtual_command::{
    EditorVirtualCommand, EditorVirtualCommandError, EditorVirtualCommandResult, EditorVirtualState,
};
