use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VoicePipelineConfig {
    pub stream_audio: bool,
    pub split_sentences: bool,
}
