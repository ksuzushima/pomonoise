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
        // TODO: implement
    }

    /// Toggle pause/resume.
    pub fn toggle_pause(&mut self) {
        // TODO: implement
    }

    /// Skip to the next phase immediately.
    pub fn skip(&mut self) {
        // TODO: implement
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
    pub fn noise_display(&self) -> String {
        format!("{} vol {:.2}", self.noise, self.volume)
    }
}
