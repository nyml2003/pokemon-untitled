use punctum_input::{
    KeyEvent, KeyPhase, LogicalKey, Modifiers, NamedKey, PhysicalKeyCode, TextEvent, TextEventError,
};
use winit::{
    event::ElementState,
    keyboard::{
        Key as WinitKey, KeyCode as WinitKeyCode, ModifiersState, NamedKey as WinitNamedKey,
        PhysicalKey as WinitPhysicalKey,
    },
};

/// The public fields needed from a winit keyboard event plus its modifier snapshot.
///
/// Winit reports modifiers separately from `WindowEvent::KeyboardInput`, so the
/// host must retain the latest `ModifiersChanged` state and include it here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WinitKeyEventSnapshot {
    pub physical_key: WinitPhysicalKey,
    pub logical_key: WinitKey,
    pub modifiers: ModifiersState,
    pub state: ElementState,
    pub repeat: bool,
}

impl WinitKeyEventSnapshot {
    pub fn new(
        physical_key: WinitPhysicalKey,
        logical_key: WinitKey,
        modifiers: ModifiersState,
        state: ElementState,
        repeat: bool,
    ) -> Self {
        Self {
            physical_key,
            logical_key,
            modifiers,
            state,
            repeat,
        }
    }
}

/// The committed text reported alongside a winit keyboard event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WinitCommittedTextSnapshot {
    pub text: Option<String>,
}

impl WinitCommittedTextSnapshot {
    pub fn new(text: Option<String>) -> Self {
        Self { text }
    }
}

/// Converts winit committed text into Punctum's text input contract.
pub fn normalize_committed_text(
    event: WinitCommittedTextSnapshot,
) -> Result<Option<TextEvent>, TextEventError> {
    event.text.map(TextEvent::new).transpose()
}

/// Converts the public data from a winit keyboard event into Punctum's input contract.
pub fn normalize_key_event(event: WinitKeyEventSnapshot) -> KeyEvent {
    KeyEvent {
        physical: Some(normalize_physical_key(event.physical_key)),
        logical: normalize_logical_key(event.logical_key),
        modifiers: normalize_modifiers(event.modifiers),
        phase: normalize_phase(event.state, event.repeat),
    }
}

fn normalize_physical_key(key: WinitPhysicalKey) -> PhysicalKeyCode {
    let WinitPhysicalKey::Code(code) = key else {
        return PhysicalKeyCode::Unidentified;
    };

    match code {
        WinitKeyCode::KeyA => PhysicalKeyCode::KeyA,
        WinitKeyCode::KeyB => PhysicalKeyCode::KeyB,
        WinitKeyCode::KeyC => PhysicalKeyCode::KeyC,
        WinitKeyCode::KeyD => PhysicalKeyCode::KeyD,
        WinitKeyCode::KeyE => PhysicalKeyCode::KeyE,
        WinitKeyCode::KeyF => PhysicalKeyCode::KeyF,
        WinitKeyCode::KeyG => PhysicalKeyCode::KeyG,
        WinitKeyCode::KeyH => PhysicalKeyCode::KeyH,
        WinitKeyCode::KeyI => PhysicalKeyCode::KeyI,
        WinitKeyCode::KeyJ => PhysicalKeyCode::KeyJ,
        WinitKeyCode::KeyK => PhysicalKeyCode::KeyK,
        WinitKeyCode::KeyL => PhysicalKeyCode::KeyL,
        WinitKeyCode::KeyM => PhysicalKeyCode::KeyM,
        WinitKeyCode::KeyN => PhysicalKeyCode::KeyN,
        WinitKeyCode::KeyO => PhysicalKeyCode::KeyO,
        WinitKeyCode::KeyP => PhysicalKeyCode::KeyP,
        WinitKeyCode::KeyQ => PhysicalKeyCode::KeyQ,
        WinitKeyCode::KeyR => PhysicalKeyCode::KeyR,
        WinitKeyCode::KeyS => PhysicalKeyCode::KeyS,
        WinitKeyCode::KeyT => PhysicalKeyCode::KeyT,
        WinitKeyCode::KeyU => PhysicalKeyCode::KeyU,
        WinitKeyCode::KeyV => PhysicalKeyCode::KeyV,
        WinitKeyCode::KeyW => PhysicalKeyCode::KeyW,
        WinitKeyCode::KeyX => PhysicalKeyCode::KeyX,
        WinitKeyCode::KeyY => PhysicalKeyCode::KeyY,
        WinitKeyCode::KeyZ => PhysicalKeyCode::KeyZ,
        WinitKeyCode::Digit0 => PhysicalKeyCode::Digit0,
        WinitKeyCode::Digit1 => PhysicalKeyCode::Digit1,
        WinitKeyCode::Digit2 => PhysicalKeyCode::Digit2,
        WinitKeyCode::Digit3 => PhysicalKeyCode::Digit3,
        WinitKeyCode::Digit4 => PhysicalKeyCode::Digit4,
        WinitKeyCode::Digit5 => PhysicalKeyCode::Digit5,
        WinitKeyCode::Digit6 => PhysicalKeyCode::Digit6,
        WinitKeyCode::Digit7 => PhysicalKeyCode::Digit7,
        WinitKeyCode::Digit8 => PhysicalKeyCode::Digit8,
        WinitKeyCode::Digit9 => PhysicalKeyCode::Digit9,
        WinitKeyCode::ArrowUp => PhysicalKeyCode::ArrowUp,
        WinitKeyCode::ArrowDown => PhysicalKeyCode::ArrowDown,
        WinitKeyCode::ArrowLeft => PhysicalKeyCode::ArrowLeft,
        WinitKeyCode::ArrowRight => PhysicalKeyCode::ArrowRight,
        WinitKeyCode::Home => PhysicalKeyCode::Home,
        WinitKeyCode::End => PhysicalKeyCode::End,
        WinitKeyCode::PageUp => PhysicalKeyCode::PageUp,
        WinitKeyCode::PageDown => PhysicalKeyCode::PageDown,
        WinitKeyCode::Enter => PhysicalKeyCode::Enter,
        WinitKeyCode::Escape => PhysicalKeyCode::Escape,
        WinitKeyCode::Space => PhysicalKeyCode::Space,
        WinitKeyCode::Tab => PhysicalKeyCode::Tab,
        WinitKeyCode::Backspace => PhysicalKeyCode::Backspace,
        WinitKeyCode::Insert => PhysicalKeyCode::Insert,
        WinitKeyCode::Delete => PhysicalKeyCode::Delete,
        WinitKeyCode::ShiftLeft => PhysicalKeyCode::ShiftLeft,
        WinitKeyCode::ShiftRight => PhysicalKeyCode::ShiftRight,
        WinitKeyCode::ControlLeft => PhysicalKeyCode::ControlLeft,
        WinitKeyCode::ControlRight => PhysicalKeyCode::ControlRight,
        WinitKeyCode::AltLeft => PhysicalKeyCode::AltLeft,
        WinitKeyCode::AltRight => PhysicalKeyCode::AltRight,
        WinitKeyCode::SuperLeft => PhysicalKeyCode::SuperLeft,
        WinitKeyCode::SuperRight => PhysicalKeyCode::SuperRight,
        WinitKeyCode::F1 => PhysicalKeyCode::F1,
        WinitKeyCode::F2 => PhysicalKeyCode::F2,
        WinitKeyCode::F3 => PhysicalKeyCode::F3,
        WinitKeyCode::F4 => PhysicalKeyCode::F4,
        WinitKeyCode::F5 => PhysicalKeyCode::F5,
        WinitKeyCode::F6 => PhysicalKeyCode::F6,
        WinitKeyCode::F7 => PhysicalKeyCode::F7,
        WinitKeyCode::F8 => PhysicalKeyCode::F8,
        WinitKeyCode::F9 => PhysicalKeyCode::F9,
        WinitKeyCode::F10 => PhysicalKeyCode::F10,
        WinitKeyCode::F11 => PhysicalKeyCode::F11,
        WinitKeyCode::F12 => PhysicalKeyCode::F12,
        _ => PhysicalKeyCode::Unidentified,
    }
}

fn normalize_logical_key(key: WinitKey) -> LogicalKey {
    match key {
        WinitKey::Character(character) if character.as_str() == " " => {
            LogicalKey::Named(NamedKey::Space)
        }
        WinitKey::Character(character) => LogicalKey::Character(character.to_string()),
        WinitKey::Named(named) => normalize_named_key(named),
        WinitKey::Unidentified(_) | WinitKey::Dead(_) => LogicalKey::Unidentified,
    }
}

fn normalize_named_key(key: WinitNamedKey) -> LogicalKey {
    let named = match key {
        WinitNamedKey::Enter => NamedKey::Enter,
        WinitNamedKey::Escape => NamedKey::Escape,
        WinitNamedKey::Backspace => NamedKey::Backspace,
        WinitNamedKey::Tab => NamedKey::Tab,
        WinitNamedKey::Space => NamedKey::Space,
        WinitNamedKey::ArrowUp => NamedKey::ArrowUp,
        WinitNamedKey::ArrowDown => NamedKey::ArrowDown,
        WinitNamedKey::ArrowLeft => NamedKey::ArrowLeft,
        WinitNamedKey::ArrowRight => NamedKey::ArrowRight,
        WinitNamedKey::Home => NamedKey::Home,
        WinitNamedKey::End => NamedKey::End,
        WinitNamedKey::PageUp => NamedKey::PageUp,
        WinitNamedKey::PageDown => NamedKey::PageDown,
        WinitNamedKey::Insert => NamedKey::Insert,
        WinitNamedKey::Delete => NamedKey::Delete,
        WinitNamedKey::Shift => NamedKey::Shift,
        WinitNamedKey::Control => NamedKey::Control,
        WinitNamedKey::Alt => NamedKey::Alt,
        WinitNamedKey::Super => NamedKey::Super,
        WinitNamedKey::F1 => NamedKey::Function(1),
        WinitNamedKey::F2 => NamedKey::Function(2),
        WinitNamedKey::F3 => NamedKey::Function(3),
        WinitNamedKey::F4 => NamedKey::Function(4),
        WinitNamedKey::F5 => NamedKey::Function(5),
        WinitNamedKey::F6 => NamedKey::Function(6),
        WinitNamedKey::F7 => NamedKey::Function(7),
        WinitNamedKey::F8 => NamedKey::Function(8),
        WinitNamedKey::F9 => NamedKey::Function(9),
        WinitNamedKey::F10 => NamedKey::Function(10),
        WinitNamedKey::F11 => NamedKey::Function(11),
        WinitNamedKey::F12 => NamedKey::Function(12),
        _ => return LogicalKey::Unidentified,
    };
    LogicalKey::Named(named)
}

fn normalize_modifiers(modifiers: ModifiersState) -> Modifiers {
    Modifiers {
        shift: modifiers.shift_key(),
        control: modifiers.control_key(),
        alt: modifiers.alt_key(),
        super_key: modifiers.super_key(),
    }
}

fn normalize_phase(state: ElementState, repeat: bool) -> KeyPhase {
    match (state, repeat) {
        (ElementState::Released, _) => KeyPhase::Release,
        (ElementState::Pressed, true) => KeyPhase::Repeat,
        (ElementState::Pressed, false) => KeyPhase::Press,
    }
}
