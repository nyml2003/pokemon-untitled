use punctum_input::{KeyPhase, LogicalKey, Modifiers, NamedKey, PhysicalKeyCode, TextEventError};
use punctum_wgpu::{
    WinitCommittedTextSnapshot, WinitKeyEventSnapshot, normalize_committed_text,
    normalize_key_event,
};
use winit::{
    event::ElementState,
    keyboard::{
        Key, KeyCode, ModifiersState, NamedKey as WinitNamedKey, NativeKey, NativeKeyCode,
        PhysicalKey,
    },
};

fn raw_key(
    physical_key: PhysicalKey,
    logical_key: Key,
    modifiers: ModifiersState,
    state: ElementState,
    repeat: bool,
) -> WinitKeyEventSnapshot {
    WinitKeyEventSnapshot::new(physical_key, logical_key, modifiers, state, repeat)
}

fn pressed(physical_key: KeyCode, logical_key: WinitNamedKey) -> WinitKeyEventSnapshot {
    raw_key(
        PhysicalKey::Code(physical_key),
        Key::Named(logical_key),
        ModifiersState::empty(),
        ElementState::Pressed,
        false,
    )
}

fn committed_text(text: Option<&str>) -> WinitCommittedTextSnapshot {
    WinitCommittedTextSnapshot::new(text.map(str::to_owned))
}

#[test]
fn missing_committed_text_produces_no_text_event() {
    assert_eq!(normalize_committed_text(committed_text(None)), Ok(None));
}

#[test]
fn committed_text_preserves_ascii_and_cjk_text() {
    for text in [" hello ", "你好"] {
        let event = normalize_committed_text(committed_text(Some(text)))
            .unwrap()
            .unwrap();

        assert_eq!(event.text(), text);
    }
}

#[test]
fn committed_text_preserves_multiple_code_points() {
    let text = "e\u{301}👩‍💻";
    let event = normalize_committed_text(committed_text(Some(text)))
        .unwrap()
        .unwrap();

    assert_eq!(event.text(), text);
}

#[test]
fn empty_committed_text_uses_the_text_event_error_contract() {
    assert_eq!(
        normalize_committed_text(committed_text(Some(""))),
        Err(TextEventError::EmptyText)
    );
}

#[test]
fn logical_character_and_committed_text_are_independent_channels() {
    let key = normalize_key_event(raw_key(
        PhysicalKey::Code(KeyCode::KeyA),
        Key::Character("界".into()),
        ModifiersState::empty(),
        ElementState::Pressed,
        false,
    ));

    assert_eq!(key.logical, LogicalKey::Character("界".into()));
    assert_eq!(normalize_committed_text(committed_text(None)), Ok(None));
}

#[test]
fn direction_keys_map_both_physical_and_logical_identity() {
    let cases = [
        (
            KeyCode::ArrowUp,
            WinitNamedKey::ArrowUp,
            PhysicalKeyCode::ArrowUp,
            NamedKey::ArrowUp,
        ),
        (
            KeyCode::ArrowDown,
            WinitNamedKey::ArrowDown,
            PhysicalKeyCode::ArrowDown,
            NamedKey::ArrowDown,
        ),
        (
            KeyCode::ArrowLeft,
            WinitNamedKey::ArrowLeft,
            PhysicalKeyCode::ArrowLeft,
            NamedKey::ArrowLeft,
        ),
        (
            KeyCode::ArrowRight,
            WinitNamedKey::ArrowRight,
            PhysicalKeyCode::ArrowRight,
            NamedKey::ArrowRight,
        ),
    ];

    for (raw_physical, raw_logical, physical, logical) in cases {
        let event = normalize_key_event(pressed(raw_physical, raw_logical));
        assert_eq!(event.physical, Some(physical));
        assert_eq!(event.logical, LogicalKey::Named(logical));
    }
}

#[test]
fn space_and_physical_key_r_keep_their_distinct_channels() {
    let space = normalize_key_event(pressed(KeyCode::Space, WinitNamedKey::Space));
    let restart = normalize_key_event(raw_key(
        PhysicalKey::Code(KeyCode::KeyR),
        Key::Character("r".into()),
        ModifiersState::empty(),
        ElementState::Pressed,
        false,
    ));

    assert_eq!(space.physical, Some(PhysicalKeyCode::Space));
    assert_eq!(space.logical, LogicalKey::Named(NamedKey::Space));
    assert_eq!(restart.physical, Some(PhysicalKeyCode::KeyR));
    assert_eq!(restart.logical, LogicalKey::Character("r".into()));
}

#[test]
fn logical_characters_preserve_the_layout_label_without_committing_text() {
    let character = normalize_key_event(raw_key(
        PhysicalKey::Code(KeyCode::KeyA),
        Key::Character("界".into()),
        ModifiersState::SHIFT,
        ElementState::Pressed,
        false,
    ));
    let character_space = normalize_key_event(raw_key(
        PhysicalKey::Code(KeyCode::Space),
        Key::Character(" ".into()),
        ModifiersState::empty(),
        ElementState::Pressed,
        false,
    ));

    assert_eq!(character.logical, LogicalKey::Character("界".into()));
    assert_eq!(character_space.logical, LogicalKey::Named(NamedKey::Space));
}

#[test]
fn unknown_physical_and_logical_keys_are_unidentified() {
    let cases = [
        raw_key(
            PhysicalKey::Code(KeyCode::Backquote),
            Key::Named(WinitNamedKey::CapsLock),
            ModifiersState::empty(),
            ElementState::Pressed,
            false,
        ),
        raw_key(
            PhysicalKey::Unidentified(NativeKeyCode::Windows(0x1234)),
            Key::Unidentified(NativeKey::Windows(0x4321)),
            ModifiersState::empty(),
            ElementState::Pressed,
            false,
        ),
        raw_key(
            PhysicalKey::Unidentified(NativeKeyCode::Unidentified),
            Key::Dead(Some('^')),
            ModifiersState::empty(),
            ElementState::Pressed,
            false,
        ),
    ];

    for raw in cases {
        let event = normalize_key_event(raw);
        assert_eq!(event.physical, Some(PhysicalKeyCode::Unidentified));
        assert_eq!(event.logical, LogicalKey::Unidentified);
    }
}

#[test]
fn modifier_snapshot_maps_all_supported_flags() {
    let event = normalize_key_event(raw_key(
        PhysicalKey::Code(KeyCode::KeyA),
        Key::Character("A".into()),
        ModifiersState::SHIFT
            | ModifiersState::CONTROL
            | ModifiersState::ALT
            | ModifiersState::SUPER,
        ElementState::Pressed,
        false,
    ));

    assert_eq!(
        event.modifiers,
        Modifiers {
            shift: true,
            control: true,
            alt: true,
            super_key: true,
        }
    );
}

#[test]
fn event_state_and_repeat_map_to_press_repeat_and_release() {
    let cases = [
        (ElementState::Pressed, false, KeyPhase::Press),
        (ElementState::Pressed, true, KeyPhase::Repeat),
        (ElementState::Released, false, KeyPhase::Release),
        (ElementState::Released, true, KeyPhase::Release),
    ];

    for (state, repeat, phase) in cases {
        let event = normalize_key_event(raw_key(
            PhysicalKey::Code(KeyCode::ArrowLeft),
            Key::Named(WinitNamedKey::ArrowLeft),
            ModifiersState::empty(),
            state,
            repeat,
        ));
        assert_eq!(event.phase, phase);
    }
}

#[test]
fn all_punctum_physical_key_variants_have_winit_mappings() {
    let cases = [
        (KeyCode::KeyA, PhysicalKeyCode::KeyA),
        (KeyCode::KeyB, PhysicalKeyCode::KeyB),
        (KeyCode::KeyC, PhysicalKeyCode::KeyC),
        (KeyCode::KeyD, PhysicalKeyCode::KeyD),
        (KeyCode::KeyE, PhysicalKeyCode::KeyE),
        (KeyCode::KeyF, PhysicalKeyCode::KeyF),
        (KeyCode::KeyG, PhysicalKeyCode::KeyG),
        (KeyCode::KeyH, PhysicalKeyCode::KeyH),
        (KeyCode::KeyI, PhysicalKeyCode::KeyI),
        (KeyCode::KeyJ, PhysicalKeyCode::KeyJ),
        (KeyCode::KeyK, PhysicalKeyCode::KeyK),
        (KeyCode::KeyL, PhysicalKeyCode::KeyL),
        (KeyCode::KeyM, PhysicalKeyCode::KeyM),
        (KeyCode::KeyN, PhysicalKeyCode::KeyN),
        (KeyCode::KeyO, PhysicalKeyCode::KeyO),
        (KeyCode::KeyP, PhysicalKeyCode::KeyP),
        (KeyCode::KeyQ, PhysicalKeyCode::KeyQ),
        (KeyCode::KeyR, PhysicalKeyCode::KeyR),
        (KeyCode::KeyS, PhysicalKeyCode::KeyS),
        (KeyCode::KeyT, PhysicalKeyCode::KeyT),
        (KeyCode::KeyU, PhysicalKeyCode::KeyU),
        (KeyCode::KeyV, PhysicalKeyCode::KeyV),
        (KeyCode::KeyW, PhysicalKeyCode::KeyW),
        (KeyCode::KeyX, PhysicalKeyCode::KeyX),
        (KeyCode::KeyY, PhysicalKeyCode::KeyY),
        (KeyCode::KeyZ, PhysicalKeyCode::KeyZ),
        (KeyCode::Digit0, PhysicalKeyCode::Digit0),
        (KeyCode::Digit1, PhysicalKeyCode::Digit1),
        (KeyCode::Digit2, PhysicalKeyCode::Digit2),
        (KeyCode::Digit3, PhysicalKeyCode::Digit3),
        (KeyCode::Digit4, PhysicalKeyCode::Digit4),
        (KeyCode::Digit5, PhysicalKeyCode::Digit5),
        (KeyCode::Digit6, PhysicalKeyCode::Digit6),
        (KeyCode::Digit7, PhysicalKeyCode::Digit7),
        (KeyCode::Digit8, PhysicalKeyCode::Digit8),
        (KeyCode::Digit9, PhysicalKeyCode::Digit9),
        (KeyCode::ArrowUp, PhysicalKeyCode::ArrowUp),
        (KeyCode::ArrowDown, PhysicalKeyCode::ArrowDown),
        (KeyCode::ArrowLeft, PhysicalKeyCode::ArrowLeft),
        (KeyCode::ArrowRight, PhysicalKeyCode::ArrowRight),
        (KeyCode::Home, PhysicalKeyCode::Home),
        (KeyCode::End, PhysicalKeyCode::End),
        (KeyCode::PageUp, PhysicalKeyCode::PageUp),
        (KeyCode::PageDown, PhysicalKeyCode::PageDown),
        (KeyCode::Enter, PhysicalKeyCode::Enter),
        (KeyCode::Escape, PhysicalKeyCode::Escape),
        (KeyCode::Space, PhysicalKeyCode::Space),
        (KeyCode::Tab, PhysicalKeyCode::Tab),
        (KeyCode::Backspace, PhysicalKeyCode::Backspace),
        (KeyCode::Insert, PhysicalKeyCode::Insert),
        (KeyCode::Delete, PhysicalKeyCode::Delete),
        (KeyCode::ShiftLeft, PhysicalKeyCode::ShiftLeft),
        (KeyCode::ShiftRight, PhysicalKeyCode::ShiftRight),
        (KeyCode::ControlLeft, PhysicalKeyCode::ControlLeft),
        (KeyCode::ControlRight, PhysicalKeyCode::ControlRight),
        (KeyCode::AltLeft, PhysicalKeyCode::AltLeft),
        (KeyCode::AltRight, PhysicalKeyCode::AltRight),
        (KeyCode::SuperLeft, PhysicalKeyCode::SuperLeft),
        (KeyCode::SuperRight, PhysicalKeyCode::SuperRight),
        (KeyCode::F1, PhysicalKeyCode::F1),
        (KeyCode::F2, PhysicalKeyCode::F2),
        (KeyCode::F3, PhysicalKeyCode::F3),
        (KeyCode::F4, PhysicalKeyCode::F4),
        (KeyCode::F5, PhysicalKeyCode::F5),
        (KeyCode::F6, PhysicalKeyCode::F6),
        (KeyCode::F7, PhysicalKeyCode::F7),
        (KeyCode::F8, PhysicalKeyCode::F8),
        (KeyCode::F9, PhysicalKeyCode::F9),
        (KeyCode::F10, PhysicalKeyCode::F10),
        (KeyCode::F11, PhysicalKeyCode::F11),
        (KeyCode::F12, PhysicalKeyCode::F12),
    ];

    for (raw, expected) in cases {
        let event = normalize_key_event(raw_key(
            PhysicalKey::Code(raw),
            Key::Unidentified(NativeKey::Unidentified),
            ModifiersState::empty(),
            ElementState::Pressed,
            false,
        ));
        assert_eq!(event.physical, Some(expected));
    }
}

#[test]
fn all_punctum_named_key_variants_have_winit_mappings() {
    let cases = [
        (WinitNamedKey::Enter, NamedKey::Enter),
        (WinitNamedKey::Escape, NamedKey::Escape),
        (WinitNamedKey::Backspace, NamedKey::Backspace),
        (WinitNamedKey::Tab, NamedKey::Tab),
        (WinitNamedKey::Space, NamedKey::Space),
        (WinitNamedKey::ArrowUp, NamedKey::ArrowUp),
        (WinitNamedKey::ArrowDown, NamedKey::ArrowDown),
        (WinitNamedKey::ArrowLeft, NamedKey::ArrowLeft),
        (WinitNamedKey::ArrowRight, NamedKey::ArrowRight),
        (WinitNamedKey::Home, NamedKey::Home),
        (WinitNamedKey::End, NamedKey::End),
        (WinitNamedKey::PageUp, NamedKey::PageUp),
        (WinitNamedKey::PageDown, NamedKey::PageDown),
        (WinitNamedKey::Insert, NamedKey::Insert),
        (WinitNamedKey::Delete, NamedKey::Delete),
        (WinitNamedKey::Shift, NamedKey::Shift),
        (WinitNamedKey::Control, NamedKey::Control),
        (WinitNamedKey::Alt, NamedKey::Alt),
        (WinitNamedKey::Super, NamedKey::Super),
        (WinitNamedKey::F1, NamedKey::Function(1)),
        (WinitNamedKey::F2, NamedKey::Function(2)),
        (WinitNamedKey::F3, NamedKey::Function(3)),
        (WinitNamedKey::F4, NamedKey::Function(4)),
        (WinitNamedKey::F5, NamedKey::Function(5)),
        (WinitNamedKey::F6, NamedKey::Function(6)),
        (WinitNamedKey::F7, NamedKey::Function(7)),
        (WinitNamedKey::F8, NamedKey::Function(8)),
        (WinitNamedKey::F9, NamedKey::Function(9)),
        (WinitNamedKey::F10, NamedKey::Function(10)),
        (WinitNamedKey::F11, NamedKey::Function(11)),
        (WinitNamedKey::F12, NamedKey::Function(12)),
    ];

    for (raw, expected) in cases {
        let event = normalize_key_event(pressed(KeyCode::Backquote, raw));
        assert_eq!(event.logical, LogicalKey::Named(expected));
    }
}
