use punctum_input::{KeyEvent, KeyPhase, LogicalKey, NamedKey};

use crate::{EditorIntent, EditorTool, layout};
use map_project::{Collision, MapEventKind};

pub fn key_intent(event: &KeyEvent, selected: usize, atomic_count: usize) -> Option<EditorIntent> {
    if event.phase == KeyPhase::Release {
        return None;
    }
    match &event.logical {
        LogicalKey::Character(value)
            if event.modifiers.control && value.eq_ignore_ascii_case("s") =>
        {
            Some(EditorIntent::Save)
        }
        LogicalKey::Character(value)
            if event.modifiers.control && value.eq_ignore_ascii_case("z") =>
        {
            Some(EditorIntent::Undo)
        }
        LogicalKey::Character(value)
            if event.modifiers.control && value.eq_ignore_ascii_case("y") =>
        {
            Some(EditorIntent::Redo)
        }
        LogicalKey::Character(value) if value.eq_ignore_ascii_case("v") => {
            Some(EditorIntent::SelectTool(EditorTool::Visual))
        }
        LogicalKey::Character(value) if value.eq_ignore_ascii_case("a") => {
            Some(EditorIntent::AddLayer)
        }
        LogicalKey::Character(value) if value.eq_ignore_ascii_case("d") => {
            Some(EditorIntent::RemoveLayer)
        }
        LogicalKey::Named(NamedKey::Delete) => Some(EditorIntent::DeleteMaterial),
        LogicalKey::Character(value) if value == "1" => Some(EditorIntent::SelectTool(
            EditorTool::Collision(Collision::Walkable),
        )),
        LogicalKey::Character(value) if value == "2" => Some(EditorIntent::SelectTool(
            EditorTool::Collision(Collision::Blocked),
        )),
        LogicalKey::Character(value) if value == "3" => Some(EditorIntent::SelectTool(
            EditorTool::Event(Some(MapEventKind::Encounter)),
        )),
        LogicalKey::Character(value) if value == "4" => {
            Some(EditorIntent::SelectTool(EditorTool::Event(None)))
        }
        LogicalKey::Named(NamedKey::PageUp) if atomic_count > 0 => {
            Some(EditorIntent::SelectAtomic(selected.saturating_sub(1)))
        }
        LogicalKey::Named(NamedKey::PageDown) if atomic_count > 0 => Some(
            EditorIntent::SelectAtomic((selected + 1).min(atomic_count - 1)),
        ),
        LogicalKey::Named(NamedKey::Function(1)) => Some(EditorIntent::ToggleHelp),
        _ => None,
    }
}

pub fn wheel_intent(direction: f32, selected: usize, atomic_count: usize) -> Option<EditorIntent> {
    if direction == 0.0 || atomic_count == 0 {
        return None;
    }
    let page = selected / layout::ASSET_PAGE_SIZE;
    let maximum_page = atomic_count.saturating_sub(1) / layout::ASSET_PAGE_SIZE;
    let next_page = if direction > 0.0 {
        page.saturating_sub(1)
    } else {
        (page + 1).min(maximum_page)
    };
    Some(EditorIntent::SelectAtomic(
        next_page * layout::ASSET_PAGE_SIZE,
    ))
}

#[cfg(test)]
mod tests {
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
}
