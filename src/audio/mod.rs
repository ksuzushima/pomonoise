pub mod noise;

use crate::app::NoiseType;
use noise::{BrownNoiseSource, PinkNoiseSource, WhiteNoiseSource};
use rodio::{OutputStream, OutputStreamHandle, Sink};

/// AudioEngine controls noise playback.
pub struct AudioEngine {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sink: Option<Sink>,
    current_volume: f32,
}

impl AudioEngine {
    /// Create a new AudioEngine (initializes the audio output stream).
    pub fn new() -> Self {
        let (_stream, stream_handle) =
            OutputStream::try_default().expect("failed to open audio output stream");
        Self {
            _stream,
            stream_handle,
            sink: None,
            current_volume: 0.5,
        }
    }

    /// Start playing the specified noise at the given volume.
    pub fn start(&mut self, noise: NoiseType, volume: f32) {
        self.stop();
        self.current_volume = volume;

        let sink = Sink::try_new(&self.stream_handle).expect("failed to create audio sink");
        match noise {
            NoiseType::White => sink.append(WhiteNoiseSource::new()),
            NoiseType::Pink => sink.append(PinkNoiseSource::new()),
            NoiseType::Brown => sink.append(BrownNoiseSource::new()),
        }
        sink.set_volume(volume);
        self.sink = Some(sink);
    }

    /// Stop playback.
    pub fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
    }

    /// Update the volume of the currently playing noise.
    pub fn set_volume(&mut self, volume: f32) {
        self.current_volume = volume;
        if let Some(ref sink) = self.sink {
            sink.set_volume(volume);
        }
    }

    /// Switch to a different noise type (while keeping playback going).
    pub fn set_noise(&mut self, noise: NoiseType) {
        let volume = self.current_volume;
        self.start(noise, volume);
    }
}
