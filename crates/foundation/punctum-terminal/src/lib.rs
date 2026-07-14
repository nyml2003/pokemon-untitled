//! Pure terminal cell, text, resize, and patch planning.

#![forbid(unsafe_code)]

mod cell;
mod plan;
mod text;

pub use cell::{TerminalCell, TerminalCellError, TerminalColor};
pub use plan::{TerminalPlan, TerminalPlanError, TerminalRun, plan_patch, validate_cell_width};
pub use text::{TerminalTextError, resize_text_surface, write_text};
