use std::time::Duration;

use crate::{UiId, UiInteractionTarget, UiPixelOffset};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiButtonState {
    pub selected: bool,
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
    pub disabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiRipple {
    pub target: UiId,
    pub origin: UiPixelOffset,
    pub color: crate::UiColor,
    pub elapsed_ms: u32,
    pub duration_ms: u32,
}

impl UiRipple {
    pub fn progress(&self) -> f32 {
        if self.duration_ms == 0 {
            return 1.0;
        }
        (self.elapsed_ms as f32 / self.duration_ms as f32).min(1.0)
    }

    pub fn radius(&self, bounds: crate::UiRect) -> u32 {
        let left = self.origin.x.saturating_sub(bounds.x as i32).max(0) as u32;
        let top = self.origin.y.saturating_sub(bounds.y as i32).max(0) as u32;
        let right = bounds
            .x
            .saturating_add(bounds.width)
            .saturating_sub(self.origin.x.max(0) as u32);
        let bottom = bounds
            .y
            .saturating_add(bounds.height)
            .saturating_sub(self.origin.y.max(0) as u32);
        let radius = left.max(top).max(right).max(bottom);
        ((radius as f32) * self.progress()).ceil() as u32
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UiInteractionSnapshot {
    pub hovered: Option<UiId>,
    pub pressed: Option<UiId>,
    pub focused: Option<UiId>,
    pub ripples: Vec<UiRipple>,
}

impl UiInteractionSnapshot {
    pub fn button_state(&self, target: &UiInteractionTarget) -> UiButtonState {
        UiButtonState {
            selected: target.style.selected,
            hovered: self.hovered == Some(target.id),
            pressed: self.pressed == Some(target.id),
            focused: self.focused == Some(target.id),
            disabled: target.style.disabled,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiMotionPolicy {
    #[default]
    Full,
    Reduced,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UiInteraction {
    snapshot: UiInteractionSnapshot,
    motion: UiMotionPolicy,
}

impl UiInteraction {
    pub const fn new(motion: UiMotionPolicy) -> Self {
        Self {
            snapshot: UiInteractionSnapshot {
                hovered: None,
                pressed: None,
                focused: None,
                ripples: Vec::new(),
            },
            motion,
        }
    }

    pub fn snapshot(&self) -> &UiInteractionSnapshot {
        &self.snapshot
    }

    pub const fn motion(&self) -> UiMotionPolicy {
        self.motion
    }

    pub fn pointer_move(&mut self, targets: &[UiInteractionTarget], x: u32, y: u32) -> bool {
        let hovered = target_at(targets, x, y).map(|target| target.id);
        if self.snapshot.hovered == hovered {
            return false;
        }
        self.snapshot.hovered = hovered;
        true
    }

    pub fn pointer_leave(&mut self) -> bool {
        let changed = self.snapshot.hovered.is_some() || self.snapshot.pressed.is_some();
        self.snapshot.hovered = None;
        self.snapshot.pressed = None;
        changed
    }

    pub fn press(&mut self, targets: &[UiInteractionTarget], x: u32, y: u32) -> bool {
        let Some(target) = target_at(targets, x, y) else {
            self.snapshot.pressed = None;
            return false;
        };
        self.snapshot.hovered = Some(target.id);
        self.snapshot.pressed = Some(target.id);
        self.snapshot.focused = Some(target.id);
        if self.motion == UiMotionPolicy::Full
            && target.style.ripple_duration_ms != 0
            && target.style.ripple_color.alpha != 0
        {
            self.snapshot
                .ripples
                .retain(|ripple| ripple.target != target.id);
            self.snapshot.ripples.push(UiRipple {
                target: target.id,
                origin: UiPixelOffset::new(x as i32, y as i32),
                color: target.style.ripple_color,
                elapsed_ms: 0,
                duration_ms: target.style.ripple_duration_ms,
            });
        }
        true
    }

    pub fn release(&mut self, targets: &[UiInteractionTarget], x: u32, y: u32) -> Option<UiId> {
        let pressed = self.snapshot.pressed.take()?;
        let released = target_at(targets, x, y)
            .filter(|target| target.id == pressed)
            .map(|target| target.id);
        self.snapshot.hovered = target_at(targets, x, y).map(|target| target.id);
        released
    }

    pub fn focus(&mut self, id: Option<UiId>) {
        self.snapshot.focused = id;
    }

    pub fn clear_transient(&mut self) {
        self.snapshot.hovered = None;
        self.snapshot.pressed = None;
        self.snapshot.focused = None;
    }

    pub fn advance(&mut self, elapsed: Duration) -> bool {
        if self.snapshot.ripples.is_empty() {
            return false;
        }
        let elapsed_ms = elapsed.as_millis().min(u128::from(u32::MAX)) as u32;
        for ripple in &mut self.snapshot.ripples {
            ripple.elapsed_ms = ripple.elapsed_ms.saturating_add(elapsed_ms);
        }
        let before = self.snapshot.ripples.len();
        self.snapshot
            .ripples
            .retain(|ripple| ripple.elapsed_ms < ripple.duration_ms);
        before != self.snapshot.ripples.len() || elapsed_ms != 0
    }

    pub fn next_delay(&self) -> Option<Duration> {
        self.snapshot
            .ripples
            .iter()
            .map(|ripple| {
                Duration::from_millis(u64::from(
                    ripple.duration_ms.saturating_sub(ripple.elapsed_ms),
                ))
            })
            .min()
    }

    pub fn reconcile(&mut self, targets: &[UiInteractionTarget]) -> bool {
        let hovered = self.snapshot.hovered.filter(|id| {
            targets
                .iter()
                .any(|target| target.id == *id && !target.style.disabled)
        });
        let pressed = self.snapshot.pressed.filter(|id| {
            targets
                .iter()
                .any(|target| target.id == *id && !target.style.disabled)
        });
        let focused = self
            .snapshot
            .focused
            .filter(|id| targets.iter().any(|target| target.id == *id));
        let changed = self.snapshot.hovered != hovered
            || self.snapshot.pressed != pressed
            || self.snapshot.focused != focused;
        self.snapshot.hovered = hovered;
        self.snapshot.pressed = pressed;
        self.snapshot.focused = focused;
        changed
    }
}

fn target_at(targets: &[UiInteractionTarget], x: u32, y: u32) -> Option<&UiInteractionTarget> {
    targets
        .iter()
        .rev()
        .find(|target| !target.style.disabled && target.bounds.contains(x, y))
}
