use serde::{Deserialize, Serialize};

use crate::calculate_audio_length_ms;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelAudioState {
    pub sample_rate_hz: u32,
    pub sample_count: usize,
    pub playing: bool,
}

impl ModelAudioState {
    pub fn length_ms(&self) -> u64 {
        calculate_audio_length_ms(self.sample_count, self.sample_rate_hz)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelAudioTracker {
    state: ModelAudioState,
}

impl ModelAudioTracker {
    pub fn new(sample_rate_hz: u32) -> Self {
        Self {
            state: ModelAudioState {
                sample_rate_hz,
                sample_count: 0,
                playing: false,
            },
        }
    }

    pub fn push_samples(&mut self, count: usize) {
        self.state.sample_count += count;
        self.state.playing = self.state.sample_count > 0;
    }

    pub fn reset(&mut self) {
        self.state.sample_count = 0;
        self.state.playing = false;
    }

    pub fn state(&self) -> &ModelAudioState {
        &self.state
    }
}
