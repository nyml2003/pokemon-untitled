//! Platform-neutral keyboard and committed-text input contracts.
//!
//! `KeyEvent` describes key identity and lifecycle. `TextEvent` is the only
//! channel for inserting text; a logical character is not a text commit.

#![forbid(unsafe_code)]

use std::{error::Error, fmt};

/// A platform-neutral keyboard event.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    /// The physical key position, when the source exposes that information.
    ///
    /// `None` means the source has no physical-key channel. An identified key
    /// without a portable mapping is `Some(PhysicalKeyCode::Unidentified)`.
    pub physical: Option<PhysicalKeyCode>,
    /// The key meaning after the platform has applied the active layout.
    pub logical: LogicalKey,
    /// The modifiers active for this event.
    pub modifiers: Modifiers,
    /// Whether the key was pressed, repeated, or released.
    pub phase: KeyPhase,
}

/// A key identified by its physical position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PhysicalKeyCode {
    Unidentified,
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    Enter,
    Escape,
    Space,
    Tab,
    Backspace,
    Insert,
    Delete,
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,
    SuperLeft,
    SuperRight,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}

/// A key identified by its meaning under the active keyboard layout.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum LogicalKey {
    /// The character label associated with the key by the active layout.
    ///
    /// Consumers must not use this value for text insertion. Committed text
    /// arrives separately as `TextEvent`.
    Character(String),
    /// A non-character key with a portable meaning.
    Named(NamedKey),
    /// A key whose logical meaning is unavailable.
    Unidentified,
}

/// A portable logical key that does not represent character text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NamedKey {
    Enter,
    Escape,
    Backspace,
    Tab,
    Space,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    Shift,
    Control,
    Alt,
    Super,
    Function(u8),
}

/// Modifier state reported with a keyboard event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub super_key: bool,
}

/// The lifecycle phase reported by the source for a keyboard event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyPhase {
    Press,
    Repeat,
    Release,
}

/// Non-empty Unicode text committed by the platform input method.
///
/// This is the only Punctum input event that inserts text.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextEvent {
    text: String,
}

impl TextEvent {
    /// Creates an event from text already committed by the platform.
    pub fn new(text: impl Into<String>) -> Result<Self, TextEventError> {
        let text = text.into();
        if text.is_empty() {
            return Err(TextEventError::EmptyText);
        }

        Ok(Self { text })
    }

    /// Returns the committed Unicode text.
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// Validation error returned when constructing committed-text events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextEventError {
    EmptyText,
}

impl fmt::Display for TextEventError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyText => formatter.write_str("committed text must not be empty"),
        }
    }
}

impl Error for TextEventError {}
