use std::time::{Duration, Instant};

use game_native_target::PresentOutcome;

const REPORT_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone, Copy)]
pub(crate) struct FrameSample {
    pub(crate) total: Duration,
    pub(crate) advance: Duration,
    pub(crate) model: Duration,
    pub(crate) tree: Duration,
    pub(crate) layout: Duration,
    pub(crate) plan: Duration,
    pub(crate) present: Duration,
    pub(crate) commands: usize,
    pub(crate) action_hits: usize,
    pub(crate) interaction_targets: usize,
    pub(crate) instances: u64,
    pub(crate) outcome: PresentOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PerfReport {
    pub(crate) elapsed_s: f64,
    pub(crate) frames: u64,
    pub(crate) fps: f64,
    pub(crate) frame_ms: f64,
    pub(crate) max_frame_ms: f64,
    pub(crate) advance_ms: f64,
    pub(crate) model_ms: f64,
    pub(crate) tree_ms: f64,
    pub(crate) layout_ms: f64,
    pub(crate) plan_ms: f64,
    pub(crate) present_ms: f64,
    pub(crate) commands: usize,
    pub(crate) action_hits: usize,
    pub(crate) interaction_targets: usize,
    pub(crate) instances: u64,
    pub(crate) outcome: PresentOutcome,
}

impl PerfReport {
    pub(crate) fn dominant_stage(self) -> (&'static str, f64) {
        let mut dominant = ("advance", self.advance_ms);
        for candidate in [
            ("model", self.model_ms),
            ("tree", self.tree_ms),
            ("layout", self.layout_ms),
            ("plan", self.plan_ms),
            ("present", self.present_ms),
        ] {
            if candidate.1 > dominant.1 {
                dominant = candidate;
            }
        }
        dominant
    }
}

impl std::fmt::Display for PerfReport {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "elapsed={:.2}s frames={} fps={:.1} frame={:.2}ms max={:.2}ms advance={:.2}ms model={:.2}ms tree={:.2}ms layout={:.2}ms plan={:.2}ms present={:.2}ms commands={} action_hits={} targets={} instances={} outcome={:?}",
            self.elapsed_s,
            self.frames,
            self.fps,
            self.frame_ms,
            self.max_frame_ms,
            self.advance_ms,
            self.model_ms,
            self.tree_ms,
            self.layout_ms,
            self.plan_ms,
            self.present_ms,
            self.commands,
            self.action_hits,
            self.interaction_targets,
            self.instances,
            self.outcome,
        )
    }
}

pub(crate) struct FrameMetrics {
    window_started: Instant,
    window_frames: u32,
    total: Duration,
    max_frame: Duration,
    advance: Duration,
    model: Duration,
    tree: Duration,
    layout: Duration,
    plan: Duration,
    present: Duration,
    session_started: Instant,
    session_frames: u64,
    session_total: Duration,
    session_max_frame: Duration,
    session_advance: Duration,
    session_model: Duration,
    session_tree: Duration,
    session_layout: Duration,
    session_plan: Duration,
    session_present: Duration,
    session_last_sample: Option<FrameSample>,
}

#[derive(Clone, Copy)]
struct TimingTotals {
    elapsed: Duration,
    frames: u64,
    total: Duration,
    max_frame: Duration,
    advance: Duration,
    model: Duration,
    tree: Duration,
    layout: Duration,
    plan: Duration,
    present: Duration,
}

impl FrameMetrics {
    pub(crate) fn new(now: Instant) -> Self {
        Self {
            window_started: now,
            window_frames: 0,
            total: Duration::ZERO,
            max_frame: Duration::ZERO,
            advance: Duration::ZERO,
            model: Duration::ZERO,
            tree: Duration::ZERO,
            layout: Duration::ZERO,
            plan: Duration::ZERO,
            present: Duration::ZERO,
            session_started: now,
            session_frames: 0,
            session_total: Duration::ZERO,
            session_max_frame: Duration::ZERO,
            session_advance: Duration::ZERO,
            session_model: Duration::ZERO,
            session_tree: Duration::ZERO,
            session_layout: Duration::ZERO,
            session_plan: Duration::ZERO,
            session_present: Duration::ZERO,
            session_last_sample: None,
        }
    }

    pub(crate) fn record(&mut self, now: Instant, sample: FrameSample) -> Option<PerfReport> {
        self.window_frames = self.window_frames.saturating_add(1);
        self.total = self.total.saturating_add(sample.total);
        self.max_frame = self.max_frame.max(sample.total);
        self.advance = self.advance.saturating_add(sample.advance);
        self.model = self.model.saturating_add(sample.model);
        self.tree = self.tree.saturating_add(sample.tree);
        self.layout = self.layout.saturating_add(sample.layout);
        self.plan = self.plan.saturating_add(sample.plan);
        self.present = self.present.saturating_add(sample.present);
        self.session_frames = self.session_frames.saturating_add(1);
        self.session_total = self.session_total.saturating_add(sample.total);
        self.session_max_frame = self.session_max_frame.max(sample.total);
        self.session_advance = self.session_advance.saturating_add(sample.advance);
        self.session_model = self.session_model.saturating_add(sample.model);
        self.session_tree = self.session_tree.saturating_add(sample.tree);
        self.session_layout = self.session_layout.saturating_add(sample.layout);
        self.session_plan = self.session_plan.saturating_add(sample.plan);
        self.session_present = self.session_present.saturating_add(sample.present);
        self.session_last_sample = Some(sample);

        let elapsed = now.saturating_duration_since(self.window_started);
        if elapsed < REPORT_INTERVAL || self.window_frames == 0 {
            return None;
        }

        let report = make_report(
            TimingTotals {
                elapsed,
                frames: u64::from(self.window_frames),
                total: self.total,
                max_frame: self.max_frame,
                advance: self.advance,
                model: self.model,
                tree: self.tree,
                layout: self.layout,
                plan: self.plan,
                present: self.present,
            },
            sample,
        );
        self.reset(now);
        Some(report)
    }

    pub(crate) fn finish(&self, now: Instant) -> Option<PerfReport> {
        let sample = self.session_last_sample?;
        if self.session_frames == 0 {
            return None;
        }
        Some(make_report(
            TimingTotals {
                elapsed: now.saturating_duration_since(self.session_started),
                frames: self.session_frames,
                total: self.session_total,
                max_frame: self.session_max_frame,
                advance: self.session_advance,
                model: self.session_model,
                tree: self.session_tree,
                layout: self.session_layout,
                plan: self.session_plan,
                present: self.session_present,
            },
            sample,
        ))
    }

    fn reset(&mut self, now: Instant) {
        self.window_started = now;
        self.window_frames = 0;
        self.total = Duration::ZERO;
        self.max_frame = Duration::ZERO;
        self.advance = Duration::ZERO;
        self.model = Duration::ZERO;
        self.tree = Duration::ZERO;
        self.layout = Duration::ZERO;
        self.plan = Duration::ZERO;
        self.present = Duration::ZERO;
    }
}

fn make_report(totals: TimingTotals, sample: FrameSample) -> PerfReport {
    let frames_as_float = totals.frames as f64;
    let elapsed_seconds = totals.elapsed.as_secs_f64();
    PerfReport {
        elapsed_s: elapsed_seconds,
        frames: totals.frames,
        fps: if elapsed_seconds > 0.0 {
            frames_as_float / elapsed_seconds
        } else {
            0.0
        },
        frame_ms: average_millis(totals.total, frames_as_float),
        max_frame_ms: totals.max_frame.as_secs_f64() * 1000.0,
        advance_ms: average_millis(totals.advance, frames_as_float),
        model_ms: average_millis(totals.model, frames_as_float),
        tree_ms: average_millis(totals.tree, frames_as_float),
        layout_ms: average_millis(totals.layout, frames_as_float),
        plan_ms: average_millis(totals.plan, frames_as_float),
        present_ms: average_millis(totals.present, frames_as_float),
        commands: sample.commands,
        action_hits: sample.action_hits,
        interaction_targets: sample.interaction_targets,
        instances: sample.instances,
        outcome: sample.outcome,
    }
}

fn average_millis(duration: Duration, frames: f64) -> f64 {
    if frames > 0.0 {
        duration.as_secs_f64() * 1000.0 / frames
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::{FrameMetrics, FrameSample};
    use game_native_target::PresentOutcome;
    use std::time::{Duration, Instant};

    fn sample(total: Duration) -> FrameSample {
        FrameSample {
            total,
            advance: Duration::from_millis(1),
            model: Duration::from_millis(2),
            tree: Duration::from_millis(3),
            layout: Duration::from_millis(4),
            plan: Duration::from_millis(5),
            present: Duration::from_millis(6),
            commands: 12,
            action_hits: 3,
            interaction_targets: 2,
            instances: 768,
            outcome: PresentOutcome::Presented,
        }
    }

    #[test]
    fn reports_after_one_second_and_resets_the_window() -> Result<(), Box<dyn std::error::Error>> {
        let start = Instant::now();
        let mut metrics = FrameMetrics::new(start);
        assert!(
            metrics
                .record(
                    start + Duration::from_millis(500),
                    sample(Duration::from_millis(16))
                )
                .is_none()
        );
        let Some(report) = metrics.record(
            start + Duration::from_millis(1_100),
            sample(Duration::from_millis(20)),
        ) else {
            return Err(std::io::Error::other(
                "the second sample did not close the metrics window",
            )
            .into());
        };

        assert_eq!(report.commands, 12);
        assert_eq!(report.instances, 768);
        assert!(report.fps > 0.0);
        assert_eq!(report.max_frame_ms, 20.0);
        assert!(
            metrics
                .record(
                    start + Duration::from_millis(1_200),
                    sample(Duration::from_millis(16))
                )
                .is_none()
        );
        let final_report = metrics
            .finish(start + Duration::from_millis(1_300))
            .ok_or_else(|| std::io::Error::other("the final metrics report is missing"))?;
        assert_eq!(final_report.frames, 3);
        Ok(())
    }
}
