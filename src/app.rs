use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Work,
    Break,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Running,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseType {
    White,
    Pink,
    Brown,
}

impl NoiseType {
    pub fn next(self) -> Self {
        match self {
            NoiseType::White => NoiseType::Pink,
            NoiseType::Pink => NoiseType::Brown,
            NoiseType::Brown => NoiseType::White,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            NoiseType::White => "white",
            NoiseType::Pink => "pink",
            NoiseType::Brown => "brown",
        }
    }
}

impl std::fmt::Display for NoiseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub struct AppState {
    pub phase: Phase,
    pub status: Status,
    pub set_index: u32,
    pub sets: u32,
    pub remaining: Duration,
    pub noise: NoiseType,
    pub volume: f32,
    pub work_duration: Duration,
    pub break_duration: Duration,
    pub finished: bool,
}

impl AppState {
    pub fn new(
        work_duration: Duration,
        break_duration: Duration,
        sets: u32,
        noise: NoiseType,
        volume: f32,
    ) -> Self {
        Self {
            phase: Phase::Work,
            status: Status::Running,
            set_index: 1,
            sets,
            remaining: work_duration,
            noise,
            volume,
            work_duration,
            break_duration,
            finished: false,
        }
    }

    /// Advance the timer by `delta`. Only counts down when Running.
    pub fn tick(&mut self, delta: Duration) {
        if self.finished || self.status != Status::Running {
            return;
        }

        self.remaining = self.remaining.saturating_sub(delta);

        if self.remaining == Duration::ZERO {
            self.transition();
        }
    }

    /// Toggle pause/resume.
    pub fn toggle_pause(&mut self) {
        if self.finished {
            return;
        }
        self.status = match self.status {
            Status::Running => Status::Paused,
            Status::Paused => Status::Running,
        };
    }

    /// Skip to the next phase immediately.
    pub fn skip(&mut self) {
        if self.finished {
            return;
        }
        self.transition();
    }

    fn transition(&mut self) {
        match self.phase {
            Phase::Work => {
                self.phase = Phase::Break;
                self.remaining = self.break_duration;
            }
            Phase::Break => {
                self.set_index += 1;
                if self.set_index > self.sets {
                    self.finished = true;
                    self.status = Status::Paused;
                    self.remaining = Duration::ZERO;
                    return;
                }
                self.phase = Phase::Work;
                self.remaining = self.work_duration;
            }
        }
    }

    /// Cycle noise type: white -> pink -> brown -> white.
    pub fn cycle_noise(&mut self) {
        self.noise = self.noise.next();
    }

    /// Adjust volume by `delta` (positive or negative), clamped to [0.0, 1.0].
    pub fn adjust_volume(&mut self, delta: f32) {
        self.volume = (self.volume + delta).clamp(0.0, 1.0);
    }

    /// Format remaining time as MM:SS.
    pub fn remaining_display(&self) -> String {
        let total_secs = self.remaining.as_secs();
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        format!("{mins:02}:{secs:02}")
    }

    /// Format phase and progress, e.g., "WORK 2/4".
    pub fn phase_display(&self) -> String {
        let phase_str = match self.phase {
            Phase::Work => "WORK",
            Phase::Break => "BREAK",
        };
        format!("{phase_str} {}/{}", self.set_index, self.sets)
    }

    /// Format noise and volume, e.g., "pink vol 0.50".
    #[allow(dead_code)]
    pub fn noise_display(&self) -> String {
        format!("{} vol {:.2}", self.noise, self.volume)
    }

    /// Returns the elapsed fraction of the current phase (0.0 = just started, 1.0 = done).
    pub fn progress(&self) -> f64 {
        let total = match self.phase {
            Phase::Work => self.work_duration,
            Phase::Break => self.break_duration,
        };
        if total.is_zero() {
            return 1.0;
        }
        1.0 - (self.remaining.as_secs_f64() / total.as_secs_f64())
    }

    /// Returns the duration of the current phase.
    #[allow(dead_code)]
    pub fn phase_duration(&self) -> Duration {
        match self.phase {
            Phase::Work => self.work_duration,
            Phase::Break => self.break_duration,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state(work_secs: u64, break_secs: u64, sets: u32) -> AppState {
        AppState::new(
            Duration::from_secs(work_secs),
            Duration::from_secs(break_secs),
            sets,
            NoiseType::White,
            0.5,
        )
    }

    #[test]
    fn new_state_defaults() {
        let s = make_state(1500, 300, 4);
        assert_eq!(s.phase, Phase::Work);
        assert_eq!(s.status, Status::Running);
        assert_eq!(s.set_index, 1);
        assert_eq!(s.sets, 4);
        assert_eq!(s.remaining, Duration::from_secs(1500));
        assert!(!s.finished);
    }

    #[test]
    fn tick_decrements_when_running() {
        let mut s = make_state(10, 5, 1);
        s.tick(Duration::from_secs(3));
        assert_eq!(s.remaining, Duration::from_secs(7));
    }

    #[test]
    fn tick_does_not_decrement_when_paused() {
        let mut s = make_state(10, 5, 1);
        s.status = Status::Paused;
        s.tick(Duration::from_secs(3));
        assert_eq!(s.remaining, Duration::from_secs(10));
    }

    #[test]
    fn tick_does_not_decrement_when_finished() {
        let mut s = make_state(10, 5, 1);
        s.finished = true;
        s.tick(Duration::from_secs(3));
        assert_eq!(s.remaining, Duration::from_secs(10));
    }

    #[test]
    fn tick_work_to_break_transition() {
        let mut s = make_state(10, 5, 2);
        s.tick(Duration::from_secs(10));
        assert_eq!(s.phase, Phase::Break);
        assert_eq!(s.remaining, Duration::from_secs(5));
        assert_eq!(s.set_index, 1); // same set_index during break
        assert!(!s.finished);
    }

    #[test]
    fn tick_break_to_work_transition() {
        let mut s = make_state(10, 5, 2);
        // Work -> Break
        s.tick(Duration::from_secs(10));
        assert_eq!(s.phase, Phase::Break);
        assert_eq!(s.set_index, 1);

        // Break -> Work
        s.tick(Duration::from_secs(5));
        assert_eq!(s.phase, Phase::Work);
        assert_eq!(s.remaining, Duration::from_secs(10));
        assert_eq!(s.set_index, 2);
        assert!(!s.finished);
    }

    #[test]
    fn tick_saturating_sub_overflow() {
        let mut s = make_state(5, 3, 2);
        // Tick with more than remaining -- should saturate to 0 and transition
        s.tick(Duration::from_secs(100));
        assert_eq!(s.phase, Phase::Break);
        assert_eq!(s.remaining, Duration::from_secs(3));
    }

    #[test]
    fn full_session_finishes() {
        let mut s = make_state(10, 5, 2);

        // Set 1: Work
        s.tick(Duration::from_secs(10));
        assert_eq!(s.phase, Phase::Break);
        assert_eq!(s.set_index, 1);

        // Set 1: Break
        s.tick(Duration::from_secs(5));
        assert_eq!(s.phase, Phase::Work);
        assert_eq!(s.set_index, 2);

        // Set 2: Work
        s.tick(Duration::from_secs(10));
        assert_eq!(s.phase, Phase::Break);
        assert_eq!(s.set_index, 2);

        // Set 2: Break -> finished
        s.tick(Duration::from_secs(5));
        assert!(s.finished);
        assert_eq!(s.remaining, Duration::ZERO);
    }

    #[test]
    fn single_set_finishes() {
        let mut s = make_state(10, 5, 1);

        // Work -> Break
        s.tick(Duration::from_secs(10));
        assert_eq!(s.phase, Phase::Break);

        // Break -> finished
        s.tick(Duration::from_secs(5));
        assert!(s.finished);
    }

    #[test]
    fn tick_no_op_after_finished() {
        let mut s = make_state(10, 5, 1);
        s.tick(Duration::from_secs(10)); // Work -> Break
        s.tick(Duration::from_secs(5)); // Break -> finished
        assert!(s.finished);

        // Further ticks should be no-ops
        s.tick(Duration::from_secs(100));
        assert!(s.finished);
        assert_eq!(s.remaining, Duration::ZERO);
    }

    #[test]
    fn toggle_pause_running_to_paused() {
        let mut s = make_state(10, 5, 1);
        assert_eq!(s.status, Status::Running);
        s.toggle_pause();
        assert_eq!(s.status, Status::Paused);
    }

    #[test]
    fn toggle_pause_paused_to_running() {
        let mut s = make_state(10, 5, 1);
        s.toggle_pause(); // Running -> Paused
        s.toggle_pause(); // Paused -> Running
        assert_eq!(s.status, Status::Running);
    }

    #[test]
    fn toggle_pause_no_op_when_finished() {
        let mut s = make_state(10, 5, 1);
        s.finished = true;
        s.status = Status::Paused;
        s.toggle_pause();
        assert_eq!(s.status, Status::Paused); // no change
    }

    #[test]
    fn skip_work_to_break() {
        let mut s = make_state(10, 5, 2);
        s.tick(Duration::from_secs(3)); // partially through work
        s.skip();
        assert_eq!(s.phase, Phase::Break);
        assert_eq!(s.remaining, Duration::from_secs(5));
        assert_eq!(s.set_index, 1);
    }

    #[test]
    fn skip_break_to_work() {
        let mut s = make_state(10, 5, 2);
        s.skip(); // Work -> Break
        s.skip(); // Break -> Work
        assert_eq!(s.phase, Phase::Work);
        assert_eq!(s.remaining, Duration::from_secs(10));
        assert_eq!(s.set_index, 2);
    }

    #[test]
    fn skip_last_break_finishes() {
        let mut s = make_state(10, 5, 1);
        s.skip(); // Work -> Break
        s.skip(); // Break -> finished
        assert!(s.finished);
    }

    #[test]
    fn skip_no_op_when_finished() {
        let mut s = make_state(10, 5, 1);
        s.finished = true;
        let phase_before = s.phase;
        s.skip();
        assert_eq!(s.phase, phase_before); // no change
    }

    #[test]
    fn cycle_noise() {
        let mut s = make_state(10, 5, 1);
        assert_eq!(s.noise, NoiseType::White);
        s.cycle_noise();
        assert_eq!(s.noise, NoiseType::Pink);
        s.cycle_noise();
        assert_eq!(s.noise, NoiseType::Brown);
        s.cycle_noise();
        assert_eq!(s.noise, NoiseType::White);
    }

    #[test]
    fn adjust_volume_up() {
        let mut s = make_state(10, 5, 1);
        s.volume = 0.5;
        s.adjust_volume(0.1);
        assert!((s.volume - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn adjust_volume_clamp_max() {
        let mut s = make_state(10, 5, 1);
        s.volume = 0.9;
        s.adjust_volume(0.5);
        assert!((s.volume - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn adjust_volume_clamp_min() {
        let mut s = make_state(10, 5, 1);
        s.volume = 0.1;
        s.adjust_volume(-0.5);
        assert!((s.volume - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn remaining_display_format() {
        let mut s = make_state(90, 5, 1);
        assert_eq!(s.remaining_display(), "01:30");
        s.remaining = Duration::from_secs(0);
        assert_eq!(s.remaining_display(), "00:00");
        s.remaining = Duration::from_secs(3661);
        assert_eq!(s.remaining_display(), "61:01");
    }

    #[test]
    fn phase_display_format() {
        let mut s = make_state(10, 5, 4);
        assert_eq!(s.phase_display(), "WORK 1/4");
        s.phase = Phase::Break;
        assert_eq!(s.phase_display(), "BREAK 1/4");
        s.set_index = 3;
        assert_eq!(s.phase_display(), "BREAK 3/4");
    }

    #[test]
    fn noise_display_format() {
        let s = make_state(10, 5, 1);
        assert_eq!(s.noise_display(), "white vol 0.50");
    }

    #[test]
    fn noise_type_as_str() {
        assert_eq!(NoiseType::White.as_str(), "white");
        assert_eq!(NoiseType::Pink.as_str(), "pink");
        assert_eq!(NoiseType::Brown.as_str(), "brown");
    }

    #[test]
    fn noise_type_display() {
        assert_eq!(format!("{}", NoiseType::White), "white");
        assert_eq!(format!("{}", NoiseType::Pink), "pink");
        assert_eq!(format!("{}", NoiseType::Brown), "brown");
    }

    #[test]
    fn pause_prevents_tick_then_resume_allows() {
        let mut s = make_state(10, 5, 1);
        s.toggle_pause();
        s.tick(Duration::from_secs(5));
        assert_eq!(s.remaining, Duration::from_secs(10)); // no change

        s.toggle_pause();
        s.tick(Duration::from_secs(3));
        assert_eq!(s.remaining, Duration::from_secs(7)); // decremented
    }

    #[test]
    fn multiple_small_ticks() {
        let mut s = make_state(5, 3, 1);
        for _ in 0..5 {
            s.tick(Duration::from_secs(1));
        }
        // Should have transitioned to Break
        assert_eq!(s.phase, Phase::Break);
        assert_eq!(s.remaining, Duration::from_secs(3));
    }

    #[test]
    fn progress_at_start() {
        let s = make_state(10, 5, 1);
        assert!((s.progress() - 0.0).abs() < 0.01);
    }

    #[test]
    fn progress_halfway() {
        let mut s = make_state(10, 5, 1);
        s.tick(Duration::from_secs(5));
        assert!((s.progress() - 0.5).abs() < 0.01);
    }

    #[test]
    fn progress_during_break() {
        let mut s = make_state(10, 5, 2);
        s.tick(Duration::from_secs(10)); // Work -> Break
        assert!((s.progress() - 0.0).abs() < 0.01); // Break just started
        s.tick(Duration::from_secs(3));
        assert!((s.progress() - 0.6).abs() < 0.01); // 3/5 elapsed
    }

    #[test]
    fn progress_with_zero_duration() {
        let s = make_state(0, 0, 1);
        // Zero duration should return 1.0 (completed)
        assert!((s.progress() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn progress_at_completion() {
        let mut s = make_state(10, 5, 1);
        s.tick(Duration::from_secs(10)); // Work -> Break
        s.tick(Duration::from_secs(5)); // Break -> finished
        assert!(s.finished);
        // Progress of a finished state (remaining=0, duration=break)
        assert!((s.progress() - 1.0).abs() < 0.01);
    }
}
