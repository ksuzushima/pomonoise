use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::time::Duration;

/// All noise sources are mono and run at a CD-quality sample rate.
const SAMPLE_RATE: u32 = 44100;
const CHANNELS: u16 = 1;

// ---------------------------------------------------------------------------
// White Noise
// ---------------------------------------------------------------------------

pub struct WhiteNoiseSource {
    rng: SmallRng,
}

impl WhiteNoiseSource {
    pub fn new() -> Self {
        Self {
            rng: SmallRng::from_os_rng(),
        }
    }
}

impl Iterator for WhiteNoiseSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        Some(self.rng.random_range(-1.0f32..1.0f32))
    }
}

impl rodio::Source for WhiteNoiseSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        CHANNELS
    }

    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

// ---------------------------------------------------------------------------
// Pink Noise (Voss-McCartney algorithm)
// ---------------------------------------------------------------------------

const NUM_PINK_ROWS: usize = 16;

pub struct PinkNoiseSource {
    rng: SmallRng,
    rows: [f32; NUM_PINK_ROWS],
    running_sum: f32,
    index: u32,
}

impl PinkNoiseSource {
    pub fn new() -> Self {
        let mut rng = SmallRng::from_os_rng();
        let mut rows = [0.0f32; NUM_PINK_ROWS];
        let mut running_sum = 0.0f32;
        for row in &mut rows {
            let val = rng.random_range(-1.0f32..1.0f32);
            *row = val;
            running_sum += val;
        }
        Self {
            rng,
            rows,
            running_sum,
            index: 0,
        }
    }
}

impl Iterator for PinkNoiseSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        self.index = self.index.wrapping_add(1);
        // Determine which rows to update based on trailing zeros of index
        let changed_bits = (self.index ^ (self.index.wrapping_sub(1))) >> 1;
        for i in 0..NUM_PINK_ROWS {
            if changed_bits & (1 << i) != 0 {
                self.running_sum -= self.rows[i];
                let new_val = self.rng.random_range(-1.0f32..1.0f32);
                self.rows[i] = new_val;
                self.running_sum += new_val;
            }
        }
        // Normalize: running_sum is the sum of NUM_PINK_ROWS values each in [-1,1]
        // So the max magnitude is NUM_PINK_ROWS. Divide by NUM_PINK_ROWS to get [-1,1].
        let sample = self.running_sum / NUM_PINK_ROWS as f32;
        Some(sample.clamp(-1.0, 1.0))
    }
}

impl rodio::Source for PinkNoiseSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        CHANNELS
    }

    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

// ---------------------------------------------------------------------------
// Brown Noise (leaky-integrated random walk)
// ---------------------------------------------------------------------------

/// Random-walk step half-width applied each sample.
const BROWN_STEP: f32 = 0.05;
/// Leak factor (slightly below 1.0) applied to the running value each sample.
/// It pulls the walk gently back toward zero, preventing DC drift and stopping
/// the value from getting pinned at the ±1.0 clamp over long runs (which would
/// otherwise flatten the sound into long constant segments).
const BROWN_LEAK: f32 = 0.995;

pub struct BrownNoiseSource {
    rng: SmallRng,
    x: f32,
}

impl BrownNoiseSource {
    pub fn new() -> Self {
        Self {
            rng: SmallRng::from_os_rng(),
            x: 0.0,
        }
    }
}

impl Iterator for BrownNoiseSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let step = self.rng.random_range(-BROWN_STEP..BROWN_STEP);
        self.x = (self.x * BROWN_LEAK + step).clamp(-1.0, 1.0);
        Some(self.x)
    }
}

impl rodio::Source for BrownNoiseSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        CHANNELS
    }

    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodio::Source;

    const SAMPLE_COUNT: usize = 44100; // 1 second of audio

    // --- White Noise ---

    #[test]
    fn white_noise_samples_in_range() {
        let source = WhiteNoiseSource::new();
        for sample in source.take(SAMPLE_COUNT) {
            assert!(
                (-1.0..=1.0).contains(&sample),
                "white noise sample out of range: {sample}"
            );
        }
    }

    #[test]
    fn white_noise_source_metadata() {
        let source = WhiteNoiseSource::new();
        assert_eq!(source.channels(), 1);
        assert_eq!(source.sample_rate(), 44100);
        assert_eq!(source.total_duration(), None);
        assert_eq!(source.current_frame_len(), None);
    }

    #[test]
    fn white_noise_mean_near_zero() {
        let source = WhiteNoiseSource::new();
        let samples: Vec<f32> = source.take(SAMPLE_COUNT).collect();
        let mean: f32 = samples.iter().sum::<f32>() / samples.len() as f32;
        assert!(
            mean.abs() < 0.05,
            "white noise mean too far from zero: {mean}"
        );
    }

    // --- Pink Noise ---

    #[test]
    fn pink_noise_samples_in_range() {
        let source = PinkNoiseSource::new();
        for sample in source.take(SAMPLE_COUNT) {
            assert!(
                (-1.0..=1.0).contains(&sample),
                "pink noise sample out of range: {sample}"
            );
        }
    }

    #[test]
    fn pink_noise_source_metadata() {
        let source = PinkNoiseSource::new();
        assert_eq!(source.channels(), 1);
        assert_eq!(source.sample_rate(), 44100);
        assert_eq!(source.total_duration(), None);
        assert_eq!(source.current_frame_len(), None);
    }

    #[test]
    fn pink_noise_mean_near_zero() {
        let source = PinkNoiseSource::new();
        let samples: Vec<f32> = source.take(SAMPLE_COUNT).collect();
        let mean: f32 = samples.iter().sum::<f32>() / samples.len() as f32;
        assert!(
            mean.abs() < 0.1,
            "pink noise mean too far from zero: {mean}"
        );
    }

    #[test]
    fn pink_noise_lower_variance_than_white() {
        let white: Vec<f32> = WhiteNoiseSource::new().take(SAMPLE_COUNT).collect();
        let pink: Vec<f32> = PinkNoiseSource::new().take(SAMPLE_COUNT).collect();

        let white_var: f32 = white.iter().map(|s| s * s).sum::<f32>() / white.len() as f32;
        let pink_var: f32 = pink.iter().map(|s| s * s).sum::<f32>() / pink.len() as f32;

        assert!(
            pink_var < white_var,
            "pink noise variance ({pink_var}) should be lower than white ({white_var})"
        );
    }

    // --- Brown Noise ---

    #[test]
    fn brown_noise_samples_in_range() {
        let source = BrownNoiseSource::new();
        for sample in source.take(SAMPLE_COUNT) {
            assert!(
                (-1.0..=1.0).contains(&sample),
                "brown noise sample out of range: {sample}"
            );
        }
    }

    #[test]
    fn brown_noise_source_metadata() {
        let source = BrownNoiseSource::new();
        assert_eq!(source.channels(), 1);
        assert_eq!(source.sample_rate(), 44100);
        assert_eq!(source.total_duration(), None);
        assert_eq!(source.current_frame_len(), None);
    }

    #[test]
    fn brown_noise_starts_at_zero() {
        let mut source = BrownNoiseSource::new();
        let first = source.next().unwrap();
        // First sample is 0.0 * leak + step, so within ±BROWN_STEP.
        assert!(
            first.abs() <= BROWN_STEP,
            "brown noise should start near zero: {first}"
        );
    }

    #[test]
    fn brown_noise_consecutive_samples_close() {
        let source = BrownNoiseSource::new();
        let samples: Vec<f32> = source.take(1000).collect();
        for window in samples.windows(2) {
            let diff = (window[1] - window[0]).abs();
            assert!(
                diff <= 0.1,
                "brown noise step too large: {diff} (from {} to {})",
                window[0],
                window[1]
            );
        }
    }

    #[test]
    fn brown_noise_no_dc_drift() {
        // The leaky integrator keeps the long-run average near zero. A pure
        // random walk is free to wander far from the origin over this many
        // samples, so this guards the leak that distinguishes the two.
        let source = BrownNoiseSource::new();
        let samples: Vec<f32> = source.take(SAMPLE_COUNT * 5).collect();
        let mean = samples.iter().sum::<f32>() / samples.len() as f32;
        assert!(
            mean.abs() < 0.1,
            "brown noise drifted from zero: mean = {mean}"
        );
    }

    #[test]
    fn brown_noise_rarely_hits_clamp() {
        // With the leak pulling values back toward zero, the walk almost never
        // reaches the ±1.0 clamp, avoiding the long flat segments a clamped
        // pure random walk would produce.
        let source = BrownNoiseSource::new();
        let samples: Vec<f32> = source.take(SAMPLE_COUNT * 5).collect();
        let clamped = samples.iter().filter(|s| s.abs() >= 1.0).count();
        let ratio = clamped as f64 / samples.len() as f64;
        assert!(
            ratio < 0.01,
            "brown noise hits the clamp too often: {ratio}"
        );
    }
}
