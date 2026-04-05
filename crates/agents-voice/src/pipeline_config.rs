use serde::{Deserialize, Serialize};

use crate::model::{STTModelSettings, TTSModelSettings};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VoicePipelineConfig {
    pub stream_audio: bool,
    pub split_sentences: bool,
    pub stt_settings: STTModelSettings,
    pub tts_settings: TTSModelSettings,
}
