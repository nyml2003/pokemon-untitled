//! GBA 风格玩家控制的固定语义与默认键盘映射。

use punctum_input::{KeyEvent, PhysicalKeyCode};

/// 玩家可用的十个离散控制。
///
/// 页面和战斗只消费这些语义，不得解释平台键码或新增页面私有快捷键。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameControl {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    L,
    R,
    Start,
    Select,
}

impl GameControl {
    /// 从默认键盘布局识别一个无修饰键的玩家控制。
    ///
    /// `WASD` 是十字键，`J/K` 是 A/B，`Q/E` 是 L/R，`R/L` 是 Start/Select。
    pub const fn from_key_event(event: &KeyEvent) -> Option<Self> {
        if event.modifiers.shift
            || event.modifiers.control
            || event.modifiers.alt
            || event.modifiers.super_key
        {
            return None;
        }
        match event.physical {
            Some(PhysicalKeyCode::KeyW) => Some(Self::Up),
            Some(PhysicalKeyCode::KeyS) => Some(Self::Down),
            Some(PhysicalKeyCode::KeyA) => Some(Self::Left),
            Some(PhysicalKeyCode::KeyD) => Some(Self::Right),
            Some(PhysicalKeyCode::KeyJ) => Some(Self::A),
            Some(PhysicalKeyCode::KeyK) => Some(Self::B),
            Some(PhysicalKeyCode::KeyQ) => Some(Self::L),
            Some(PhysicalKeyCode::KeyE) => Some(Self::R),
            Some(PhysicalKeyCode::KeyR) => Some(Self::Start),
            Some(PhysicalKeyCode::KeyL) => Some(Self::Select),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GameControl;
    use punctum_input::{KeyEvent, KeyPhase, LogicalKey, Modifiers, PhysicalKeyCode};

    fn key(physical: PhysicalKeyCode) -> KeyEvent {
        KeyEvent {
            physical: Some(physical),
            logical: LogicalKey::Unidentified,
            modifiers: Modifiers::default(),
            phase: KeyPhase::Press,
        }
    }

    #[test]
    fn maps_only_the_default_ten_controls() {
        let cases = [
            (PhysicalKeyCode::KeyW, GameControl::Up),
            (PhysicalKeyCode::KeyS, GameControl::Down),
            (PhysicalKeyCode::KeyA, GameControl::Left),
            (PhysicalKeyCode::KeyD, GameControl::Right),
            (PhysicalKeyCode::KeyJ, GameControl::A),
            (PhysicalKeyCode::KeyK, GameControl::B),
            (PhysicalKeyCode::KeyQ, GameControl::L),
            (PhysicalKeyCode::KeyE, GameControl::R),
            (PhysicalKeyCode::KeyR, GameControl::Start),
            (PhysicalKeyCode::KeyL, GameControl::Select),
        ];
        for (physical, control) in cases {
            assert_eq!(GameControl::from_key_event(&key(physical)), Some(control));
        }
        assert_eq!(
            GameControl::from_key_event(&key(PhysicalKeyCode::Enter)),
            None
        );
        assert_eq!(
            GameControl::from_key_event(&key(PhysicalKeyCode::ArrowUp)),
            None
        );
    }

    #[test]
    fn rejects_modified_controls() {
        let mut event = key(PhysicalKeyCode::KeyJ);
        event.modifiers.control = true;
        assert_eq!(GameControl::from_key_event(&event), None);
    }
}
