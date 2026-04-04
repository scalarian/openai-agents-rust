use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::RealtimeAudioFormat;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeClientMessage {
    pub kind: String,
    #[serde(default)]
    pub other_data: serde_json::Map<String, Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeInputAudioTranscriptionConfig {
    pub language: Option<String>,
    pub model: Option<String>,
    pub prompt: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeInputAudioNoiseReductionConfig {
    pub kind: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeTurnDetectionConfig {
    pub kind: Option<String>,
    pub create_response: Option<bool>,
    pub eagerness: Option<String>,
    pub interrupt_response: Option<bool>,
    pub prefix_padding_ms: Option<u64>,
    pub silence_duration_ms: Option<u64>,
    pub threshold: Option<f32>,
    pub idle_timeout_ms: Option<u64>,
    pub model_version: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeAudioInputConfig {
    pub format: Option<RealtimeAudioFormat>,
    pub noise_reduction: Option<RealtimeInputAudioNoiseReductionConfig>,
    pub transcription: Option<RealtimeInputAudioTranscriptionConfig>,
    pub turn_detection: Option<RealtimeTurnDetectionConfig>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeAudioOutputConfig {
    pub format: Option<RealtimeAudioFormat>,
    pub voice: Option<String>,
    pub speed: Option<f32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeAudioConfig {
    pub input: Option<RealtimeAudioInputConfig>,
    pub output: Option<RealtimeAudioOutputConfig>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeModelTracingConfig {
    pub workflow_name: Option<String>,
    pub group_id: Option<String>,
    pub metadata: Option<serde_json::Map<String, Value>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeSessionModelSettings {
    pub model_name: Option<String>,
    pub instructions: Option<String>,
    pub modalities: Option<Vec<String>>,
    pub output_modalities: Option<Vec<String>>,
    pub audio: Option<RealtimeAudioConfig>,
    pub voice: Option<String>,
    pub speed: Option<f32>,
    pub input_audio_format: Option<RealtimeAudioFormat>,
    pub output_audio_format: Option<RealtimeAudioFormat>,
    pub input_audio_transcription: Option<RealtimeInputAudioTranscriptionConfig>,
    pub input_audio_noise_reduction: Option<RealtimeInputAudioNoiseReductionConfig>,
    pub turn_detection: Option<RealtimeTurnDetectionConfig>,
    pub tool_choice: Option<String>,
    pub tracing: Option<RealtimeModelTracingConfig>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeGuardrailsSettings {
    pub debounce_text_length: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeRunConfig {
    pub model_settings: Option<RealtimeSessionModelSettings>,
    pub guardrails_settings: Option<RealtimeGuardrailsSettings>,
    pub tracing_disabled: Option<bool>,
    pub async_tool_calls: Option<bool>,
}
