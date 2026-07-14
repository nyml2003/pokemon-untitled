//! Crossterm input, presentation, session, and terminal IO.

#![forbid(unsafe_code)]

mod input;
mod runtime;

pub use crossterm::event;
pub use input::{normalize_key_event, normalize_text_event};
pub use runtime::{TerminalPresentError, TerminalPresenter, TerminalSession};
