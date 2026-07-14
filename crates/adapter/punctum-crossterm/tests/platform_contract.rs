use crossterm::event::{
    Event as RawEvent, KeyCode, KeyEvent as RawKeyEvent, KeyEventKind, KeyModifiers, MediaKeyCode,
    ModifierKeyCode,
};
use punctum_crossterm::{TerminalPresenter, normalize_key_event, normalize_text_event};
use punctum_grid::{GridPos, GridSize, Surface};
use punctum_input::{KeyPhase, LogicalKey, Modifiers, NamedKey, TextEventError};
use punctum_terminal::{TerminalCell, TerminalColor, write_text};

fn raw_key(code: KeyCode, modifiers: KeyModifiers, kind: KeyEventKind) -> RawKeyEvent {
    RawKeyEvent::new_with_kind(code, modifiers, kind)
}

#[test]
fn presenter_parks_the_cursor_at_the_origin_after_unicode_output() {
    let mut surface = Surface::filled(GridSize::new(2, 1), TerminalCell::default()).unwrap();
    write_text(
        &mut surface,
        GridPos::new(0, 0),
        "界",
        TerminalColor::White,
        TerminalColor::Black,
    )
    .unwrap();
    let mut presenter = TerminalPresenter::new(Vec::new(), 1).unwrap();

    presenter.present(&surface).unwrap();
    let output = presenter.into_inner();

    assert!(output.ends_with(b"\x1b[0m\x1b[1;1H"));
}

#[test]
fn normalize_text_event_accepts_paste_and_unmodified_character_commits() {
    let text = normalize_text_event(&RawEvent::Paste("你好👩‍💻".into())).unwrap();
    let pressed = normalize_text_event(&RawEvent::Key(raw_key(
        KeyCode::Char('x'),
        KeyModifiers::empty(),
        KeyEventKind::Press,
    )))
    .unwrap();
    let repeated = normalize_text_event(&RawEvent::Key(raw_key(
        KeyCode::Char('X'),
        KeyModifiers::SHIFT,
        KeyEventKind::Repeat,
    )))
    .unwrap();

    assert_eq!(text.unwrap().text(), "你好👩‍💻");
    assert_eq!(pressed.unwrap().text(), "x");
    assert_eq!(repeated.unwrap().text(), "X");
    assert_eq!(
        normalize_text_event(&RawEvent::Paste(String::new())),
        Err(TextEventError::EmptyText)
    );
}

#[test]
fn normalize_text_event_ignores_non_committed_key_events() {
    for event in [
        raw_key(
            KeyCode::Char('x'),
            KeyModifiers::empty(),
            KeyEventKind::Release,
        ),
        raw_key(
            KeyCode::Char('x'),
            KeyModifiers::CONTROL,
            KeyEventKind::Press,
        ),
        raw_key(KeyCode::Char('x'), KeyModifiers::ALT, KeyEventKind::Press),
        raw_key(KeyCode::Char('x'), KeyModifiers::SUPER, KeyEventKind::Press),
        raw_key(KeyCode::Enter, KeyModifiers::empty(), KeyEventKind::Press),
    ] {
        assert_eq!(normalize_text_event(&RawEvent::Key(event)).unwrap(), None);
    }
}

#[test]
fn normalize_key_event_maps_common_named_keys() {
    let cases = [
        (KeyCode::Enter, NamedKey::Enter),
        (KeyCode::Esc, NamedKey::Escape),
        (KeyCode::Backspace, NamedKey::Backspace),
        (KeyCode::Tab, NamedKey::Tab),
        (KeyCode::BackTab, NamedKey::Tab),
        (KeyCode::Left, NamedKey::ArrowLeft),
        (KeyCode::Right, NamedKey::ArrowRight),
        (KeyCode::Up, NamedKey::ArrowUp),
        (KeyCode::Down, NamedKey::ArrowDown),
        (KeyCode::Home, NamedKey::Home),
        (KeyCode::End, NamedKey::End),
        (KeyCode::PageUp, NamedKey::PageUp),
        (KeyCode::PageDown, NamedKey::PageDown),
        (KeyCode::Insert, NamedKey::Insert),
        (KeyCode::Delete, NamedKey::Delete),
        (KeyCode::F(12), NamedKey::Function(12)),
    ];

    for (raw, expected) in cases {
        let normalized =
            normalize_key_event(raw_key(raw, KeyModifiers::empty(), KeyEventKind::Press));
        assert_eq!(normalized.logical, LogicalKey::Named(expected));
        assert_eq!(normalized.physical, None);
    }
}

#[test]
fn normalize_key_event_maps_characters_and_space_without_inventing_physical_keys() {
    let character = normalize_key_event(raw_key(
        KeyCode::Char('界'),
        KeyModifiers::empty(),
        KeyEventKind::Press,
    ));
    let space = normalize_key_event(raw_key(
        KeyCode::Char(' '),
        KeyModifiers::empty(),
        KeyEventKind::Press,
    ));

    assert_eq!(character.logical, LogicalKey::Character("界".into()));
    assert_eq!(character.physical, None);
    assert_eq!(space.logical, LogicalKey::Named(NamedKey::Space));
    assert_eq!(space.physical, None);
}

#[test]
fn normalize_key_event_preserves_press_repeat_and_release() {
    let cases = [
        (KeyEventKind::Press, KeyPhase::Press),
        (KeyEventKind::Repeat, KeyPhase::Repeat),
        (KeyEventKind::Release, KeyPhase::Release),
    ];

    for (raw, expected) in cases {
        let normalized = normalize_key_event(raw_key(KeyCode::Left, KeyModifiers::empty(), raw));
        assert_eq!(normalized.phase, expected);
    }
}

#[test]
fn normalize_key_event_preserves_supported_modifiers() {
    let normalized = normalize_key_event(raw_key(
        KeyCode::Char('x'),
        KeyModifiers::SHIFT | KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER,
        KeyEventKind::Press,
    ));

    assert_eq!(
        normalized.modifiers,
        Modifiers {
            shift: true,
            control: true,
            alt: true,
            super_key: true,
        }
    );
}

#[test]
fn normalize_key_event_maps_modifier_keys_when_the_terminal_identifies_them() {
    let cases = [
        (ModifierKeyCode::LeftShift, NamedKey::Shift),
        (ModifierKeyCode::RightShift, NamedKey::Shift),
        (ModifierKeyCode::LeftControl, NamedKey::Control),
        (ModifierKeyCode::RightControl, NamedKey::Control),
        (ModifierKeyCode::LeftAlt, NamedKey::Alt),
        (ModifierKeyCode::RightAlt, NamedKey::Alt),
        (ModifierKeyCode::LeftSuper, NamedKey::Super),
        (ModifierKeyCode::RightSuper, NamedKey::Super),
    ];

    for (raw, expected) in cases {
        let normalized = normalize_key_event(raw_key(
            KeyCode::Modifier(raw),
            KeyModifiers::empty(),
            KeyEventKind::Press,
        ));
        assert_eq!(normalized.logical, LogicalKey::Named(expected));
    }
}

#[test]
fn normalize_key_event_maps_unsupported_keys_to_unidentified() {
    let keys = [
        KeyCode::Null,
        KeyCode::CapsLock,
        KeyCode::ScrollLock,
        KeyCode::NumLock,
        KeyCode::PrintScreen,
        KeyCode::Pause,
        KeyCode::Menu,
        KeyCode::KeypadBegin,
        KeyCode::Media(MediaKeyCode::Play),
        KeyCode::Modifier(ModifierKeyCode::LeftHyper),
        KeyCode::Modifier(ModifierKeyCode::LeftMeta),
        KeyCode::Modifier(ModifierKeyCode::IsoLevel3Shift),
        KeyCode::Modifier(ModifierKeyCode::IsoLevel5Shift),
    ];

    for raw in keys {
        let normalized =
            normalize_key_event(raw_key(raw, KeyModifiers::empty(), KeyEventKind::Press));
        assert_eq!(normalized.logical, LogicalKey::Unidentified);
    }
}
