use punctum_input::{
    KeyEvent, KeyPhase, LogicalKey, Modifiers, NamedKey, PhysicalKeyCode, TextEvent, TextEventError,
};

#[test]
fn key_event_preserves_physical_key() {
    let event = KeyEvent {
        physical: Some(PhysicalKeyCode::KeyW),
        logical: LogicalKey::Character("w".into()),
        modifiers: Modifiers::default(),
        phase: KeyPhase::Press,
    };

    assert_eq!(event.physical, Some(PhysicalKeyCode::KeyW));
}

#[test]
fn key_event_allows_physical_key_to_be_missing() {
    let event = KeyEvent {
        physical: None,
        logical: LogicalKey::Unidentified,
        modifiers: Modifiers::default(),
        phase: KeyPhase::Press,
    };

    assert_eq!(event.physical, None);
}

#[test]
fn physical_key_distinguishes_unavailable_from_unidentified() {
    let unavailable = None;
    let identified_but_unmapped = Some(PhysicalKeyCode::Unidentified);

    assert_ne!(unavailable, identified_but_unmapped);
}

#[test]
fn key_event_preserves_logical_key() {
    let event = KeyEvent {
        physical: Some(PhysicalKeyCode::KeyA),
        logical: LogicalKey::Character("A".into()),
        modifiers: Modifiers::default(),
        phase: KeyPhase::Press,
    };

    assert_eq!(event.logical, LogicalKey::Character("A".into()));
}

#[test]
fn key_event_preserves_modifiers() {
    let event = KeyEvent {
        physical: Some(PhysicalKeyCode::KeyA),
        logical: LogicalKey::Character("A".into()),
        modifiers: Modifiers {
            shift: true,
            control: true,
            alt: false,
            super_key: false,
        },
        phase: KeyPhase::Press,
    };

    assert_eq!(
        event.modifiers,
        Modifiers {
            shift: true,
            control: true,
            alt: false,
            super_key: false,
        }
    );
}

#[test]
fn key_event_preserves_phase() {
    let event = KeyEvent {
        physical: Some(PhysicalKeyCode::Space),
        logical: LogicalKey::Character(" ".into()),
        modifiers: Modifiers::default(),
        phase: KeyPhase::Release,
    };

    assert_eq!(event.phase, KeyPhase::Release);
}

#[test]
fn logical_key_represents_a_layout_character_without_committing_text() {
    let key = LogicalKey::Character("界".into());

    assert_eq!(key, LogicalKey::Character("界".into()));
}

#[test]
fn logical_key_represents_common_named_keys() {
    let keys = [
        LogicalKey::Named(NamedKey::Enter),
        LogicalKey::Named(NamedKey::Escape),
        LogicalKey::Named(NamedKey::Backspace),
        LogicalKey::Named(NamedKey::Tab),
        LogicalKey::Named(NamedKey::ArrowUp),
        LogicalKey::Named(NamedKey::ArrowDown),
        LogicalKey::Named(NamedKey::ArrowLeft),
        LogicalKey::Named(NamedKey::ArrowRight),
        LogicalKey::Named(NamedKey::Home),
        LogicalKey::Named(NamedKey::End),
        LogicalKey::Named(NamedKey::PageUp),
        LogicalKey::Named(NamedKey::PageDown),
        LogicalKey::Named(NamedKey::Insert),
        LogicalKey::Named(NamedKey::Delete),
    ];

    assert_eq!(keys.len(), 14);
}

#[test]
fn logical_key_represents_an_unidentified_key() {
    assert_eq!(LogicalKey::Unidentified, LogicalKey::Unidentified);
}

#[test]
fn physical_key_code_represents_common_game_controls() {
    let keys = [
        PhysicalKeyCode::KeyW,
        PhysicalKeyCode::KeyA,
        PhysicalKeyCode::KeyS,
        PhysicalKeyCode::KeyD,
        PhysicalKeyCode::ArrowUp,
        PhysicalKeyCode::ArrowDown,
        PhysicalKeyCode::ArrowLeft,
        PhysicalKeyCode::ArrowRight,
        PhysicalKeyCode::Space,
        PhysicalKeyCode::Enter,
        PhysicalKeyCode::Escape,
    ];

    assert_eq!(keys.len(), 11);
}

#[test]
fn modifiers_default_to_an_empty_set() {
    assert_eq!(
        Modifiers::default(),
        Modifiers {
            shift: false,
            control: false,
            alt: false,
            super_key: false,
        }
    );
}

#[test]
fn modifiers_represent_shift_control_alt_and_super_together() {
    let modifiers = Modifiers {
        shift: true,
        control: true,
        alt: true,
        super_key: true,
    };

    assert!(modifiers.shift);
    assert!(modifiers.control);
    assert!(modifiers.alt);
    assert!(modifiers.super_key);
}

#[test]
fn key_phase_represents_press() {
    assert_eq!(KeyPhase::Press, KeyPhase::Press);
}

#[test]
fn key_phase_represents_repeat() {
    assert_eq!(KeyPhase::Repeat, KeyPhase::Repeat);
}

#[test]
fn key_phase_represents_release() {
    assert_eq!(KeyPhase::Release, KeyPhase::Release);
}

#[test]
fn text_event_accepts_committed_chinese_text() {
    let event = TextEvent::new("你好").unwrap();

    assert_eq!(event.text(), "你好");
}

#[test]
fn text_event_accepts_committed_emoji_text() {
    let event = TextEvent::new("👩‍💻").unwrap();

    assert_eq!(event.text(), "👩‍💻");
}

#[test]
fn text_event_rejects_empty_text_with_a_structured_error() {
    assert_eq!(TextEvent::new(""), Err(TextEventError::EmptyText));
}

#[test]
fn text_event_error_has_an_actionable_message() {
    assert!(TextEventError::EmptyText.to_string().contains("empty"));
}
