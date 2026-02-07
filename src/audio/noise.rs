use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::time::Duration;

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
        1
    }

    fn sample_rate(&self) -> u32 {
        44100
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
        1
    }

    fn sample_rate(&self) -> u32 {
        44100
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

// ---------------------------------------------------------------------------
// Brown Noise (random walk)
// ---------------------------------------------------------------------------

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
        self.x += self.rng.random_range(-0.05f32..0.05f32);
        self.x = self.x.clamp(-1.0, 1.0);
        Some(self.x)
    }
}

impl rodio::Source for BrownNoiseSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        44100
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
