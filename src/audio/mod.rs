pub mod noise;

use crate::app::NoiseType;

/// AudioEngine controls noise playback.
pub struct AudioEngine {
    // TODO: implement fields
}

impl AudioEngine {
    /// Create a new AudioEngine (initializes the audio output stream).
    pub fn new() -> Self {
        // TODO: implement
        Self {}
    }

    /// Start playing the specified noise at the given volume.
    pub fn start(&mut self, _noise: NoiseType, _volume: f32) {
        // TODO: implement
    }

    /// Stop playback.
    pub fn stop(&mut self) {
        // TODO: implement
    }

    /// Update the volume of the currently playing noise.
    pub fn set_volume(&mut self, _volume: f32) {
        // TODO: implement
    }

    /// Switch to a different noise type (while keeping playback going).
    pub fn set_noise(&mut self, _noise: NoiseType) {
        // TODO: implement
    }
}
