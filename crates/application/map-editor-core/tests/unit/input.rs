use punctum_input::{KeyEvent, KeyPhase, LogicalKey, Modifiers, NamedKey};

use super::*;

fn key(logical: LogicalKey, control: bool) -> KeyEvent {
    KeyEvent {
        physical: None,
        logical,
        modifiers: Modifiers {
            control,
            ..Modifiers::default()
        },
        phase: KeyPhase::Press,
    }
}

#[test]
fn keyboard_contract_maps_every_editor_command() {
    let cases = [
        (LogicalKey::Character("s".into()), true, EditorIntent::Save),
        (LogicalKey::Character("Z".into()), true, EditorIntent::Undo),
        (LogicalKey::Character("y".into()), true, EditorIntent::Redo),
        (
            LogicalKey::Character("V".into()),
            false,
            EditorIntent::SelectTool(EditorTool::Visual),
        ),
        (
            LogicalKey::Character("a".into()),
            false,
            EditorIntent::AddLayer,
        ),
        (
            LogicalKey::Character("D".into()),
            false,
            EditorIntent::RemoveLayer,
        ),
        (
            LogicalKey::Named(NamedKey::Delete),
            false,
            EditorIntent::DeleteMaterial,
        ),
        (
            LogicalKey::Character("1".into()),
            false,
            EditorIntent::SelectTool(EditorTool::Collision(Collision::Walkable)),
        ),
        (
            LogicalKey::Character("2".into()),
            false,
            EditorIntent::SelectTool(EditorTool::Collision(Collision::Blocked)),
        ),
        (
            LogicalKey::Character("3".into()),
            false,
            EditorIntent::SelectTool(EditorTool::Event(Some(MapEventKind::Encounter))),
        ),
        (
            LogicalKey::Character("4".into()),
            false,
            EditorIntent::SelectTool(EditorTool::Event(None)),
        ),
        (
            LogicalKey::Named(NamedKey::Function(1)),
            false,
            EditorIntent::ToggleHelp,
        ),
    ];
    for (logical, control, expected) in cases {
        assert_eq!(key_intent(&key(logical, control), 3, 10), Some(expected));
    }
}

#[test]
fn keyboard_and_wheel_navigation_obey_boundaries() {
    assert_eq!(
        key_intent(&key(LogicalKey::Named(NamedKey::PageUp), false), 0, 10),
        Some(EditorIntent::SelectAtomic(0))
    );
    assert_eq!(
        key_intent(&key(LogicalKey::Named(NamedKey::PageDown), false), 9, 10),
        Some(EditorIntent::SelectAtomic(9))
    );
    assert_eq!(
        wheel_intent(-1.0, 0, layout::ASSET_PAGE_SIZE + 1),
        Some(EditorIntent::SelectAtomic(layout::ASSET_PAGE_SIZE))
    );
    assert_eq!(
        wheel_intent(1.0, layout::ASSET_PAGE_SIZE, layout::ASSET_PAGE_SIZE + 1),
        Some(EditorIntent::SelectAtomic(0))
    );
    assert_eq!(wheel_intent(0.0, 0, 1), None);
    assert_eq!(wheel_intent(1.0, 0, 0), None);
    assert_eq!(
        key_intent(&key(LogicalKey::Named(NamedKey::PageDown), false), 0, 0),
        None
    );
    let mut released = key(LogicalKey::Character("v".into()), false);
    released.phase = KeyPhase::Release;
    assert_eq!(key_intent(&released, 0, 1), None);
    assert_eq!(
        key_intent(&key(LogicalKey::Unidentified, false), 0, 1),
        None
    );
}
