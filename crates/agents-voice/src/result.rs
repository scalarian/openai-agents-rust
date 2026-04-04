use serde::{Deserialize, Serialize};

use crate::events::VoiceStreamEvent;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StreamedAudioResult {
    pub transcript: Vec<String>,
    pub audio_chunks: usize,
    pub events: Vec<VoiceStreamEvent>,
}
