use crossterm::event::{
    Event as RawEvent, KeyCode as RawKeyCode, KeyEvent as RawKeyEvent, KeyEventKind, KeyModifiers,
    ModifierKeyCode,
};
use punctum_input::{
    KeyEvent, KeyPhase, LogicalKey, Modifiers, NamedKey, TextEvent, TextEventError,
};

pub fn normalize_text_event(event: &RawEvent) -> Result<Option<TextEvent>, TextEventError> {
    match event {
        RawEvent::Paste(text) => TextEvent::new(text).map(Some),
        RawEvent::Key(event)
            if matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat)
                && !event.modifiers.intersects(
                    KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER,
                ) =>
        {
            match event.code {
                RawKeyCode::Char(character) => TextEvent::new(character.to_string()).map(Some),
                _ => Ok(None),
            }
        }
        _ => Ok(None),
    }
}

pub fn normalize_key_event(event: RawKeyEvent) -> KeyEvent {
    KeyEvent {
        physical: None,
        logical: normalize_logical_key(event.code),
        modifiers: Modifiers {
            shift: event.modifiers.contains(KeyModifiers::SHIFT),
            control: event.modifiers.contains(KeyModifiers::CONTROL),
            alt: event.modifiers.contains(KeyModifiers::ALT),
            super_key: event.modifiers.contains(KeyModifiers::SUPER),
        },
        phase: match event.kind {
            KeyEventKind::Press => KeyPhase::Press,
            KeyEventKind::Repeat => KeyPhase::Repeat,
            KeyEventKind::Release => KeyPhase::Release,
        },
    }
}

fn normalize_logical_key(code: RawKeyCode) -> LogicalKey {
    match code {
        RawKeyCode::Backspace => LogicalKey::Named(NamedKey::Backspace),
        RawKeyCode::Enter => LogicalKey::Named(NamedKey::Enter),
        RawKeyCode::Left => LogicalKey::Named(NamedKey::ArrowLeft),
        RawKeyCode::Right => LogicalKey::Named(NamedKey::ArrowRight),
        RawKeyCode::Up => LogicalKey::Named(NamedKey::ArrowUp),
        RawKeyCode::Down => LogicalKey::Named(NamedKey::ArrowDown),
        RawKeyCode::Home => LogicalKey::Named(NamedKey::Home),
        RawKeyCode::End => LogicalKey::Named(NamedKey::End),
        RawKeyCode::PageUp => LogicalKey::Named(NamedKey::PageUp),
        RawKeyCode::PageDown => LogicalKey::Named(NamedKey::PageDown),
        RawKeyCode::Tab | RawKeyCode::BackTab => LogicalKey::Named(NamedKey::Tab),
        RawKeyCode::Delete => LogicalKey::Named(NamedKey::Delete),
        RawKeyCode::Insert => LogicalKey::Named(NamedKey::Insert),
        RawKeyCode::F(number) => LogicalKey::Named(NamedKey::Function(number)),
        RawKeyCode::Char(' ') => LogicalKey::Named(NamedKey::Space),
        RawKeyCode::Char(character) => LogicalKey::Character(character.to_string()),
        RawKeyCode::Esc => LogicalKey::Named(NamedKey::Escape),
        RawKeyCode::Modifier(modifier) => normalize_modifier_key(modifier),
        RawKeyCode::Null
        | RawKeyCode::CapsLock
        | RawKeyCode::ScrollLock
        | RawKeyCode::NumLock
        | RawKeyCode::PrintScreen
        | RawKeyCode::Pause
        | RawKeyCode::Menu
        | RawKeyCode::KeypadBegin
        | RawKeyCode::Media(_) => LogicalKey::Unidentified,
    }
}

fn normalize_modifier_key(modifier: ModifierKeyCode) -> LogicalKey {
    match modifier {
        ModifierKeyCode::LeftShift | ModifierKeyCode::RightShift => {
            LogicalKey::Named(NamedKey::Shift)
        }
        ModifierKeyCode::LeftControl | ModifierKeyCode::RightControl => {
            LogicalKey::Named(NamedKey::Control)
        }
        ModifierKeyCode::LeftAlt | ModifierKeyCode::RightAlt => LogicalKey::Named(NamedKey::Alt),
        ModifierKeyCode::LeftSuper | ModifierKeyCode::RightSuper => {
            LogicalKey::Named(NamedKey::Super)
        }
        ModifierKeyCode::LeftHyper
        | ModifierKeyCode::LeftMeta
        | ModifierKeyCode::RightHyper
        | ModifierKeyCode::RightMeta
        | ModifierKeyCode::IsoLevel3Shift
        | ModifierKeyCode::IsoLevel5Shift => LogicalKey::Unidentified,
    }
}
