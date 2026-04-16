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

impl RealtimeInputAudioTranscriptionConfig {
    pub fn merge(&self, update: &Self) -> Self {
        Self {
            language: update.language.clone().or_else(|| self.language.clone()),
            model: update.model.clone().or_else(|| self.model.clone()),
            prompt: update.prompt.clone().or_else(|| self.prompt.clone()),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeInputAudioNoiseReductionConfig {
    pub kind: Option<String>,
}

impl RealtimeInputAudioNoiseReductionConfig {
    pub fn merge(&self, update: &Self) -> Self {
        Self {
            kind: update.kind.clone().or_else(|| self.kind.clone()),
        }
    }
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

impl RealtimeTurnDetectionConfig {
    pub fn merge(&self, update: &Self) -> Self {
        Self {
            kind: update.kind.clone().or_else(|| self.kind.clone()),
            create_response: update.create_response.or(self.create_response),
            eagerness: update.eagerness.clone().or_else(|| self.eagerness.clone()),
            interrupt_response: update.interrupt_response.or(self.interrupt_response),
            prefix_padding_ms: update.prefix_padding_ms.or(self.prefix_padding_ms),
            silence_duration_ms: update.silence_duration_ms.or(self.silence_duration_ms),
            threshold: update.threshold.or(self.threshold),
            idle_timeout_ms: update.idle_timeout_ms.or(self.idle_timeout_ms),
            model_version: update
                .model_version
                .clone()
                .or_else(|| self.model_version.clone()),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeAudioInputConfig {
    pub format: Option<RealtimeAudioFormat>,
    pub noise_reduction: Option<RealtimeInputAudioNoiseReductionConfig>,
    pub transcription: Option<RealtimeInputAudioTranscriptionConfig>,
    pub turn_detection: Option<RealtimeTurnDetectionConfig>,
}

impl RealtimeAudioInputConfig {
    pub fn merge(&self, update: &Self) -> Self {
        Self {
            format: update.format.clone().or_else(|| self.format.clone()),
            noise_reduction: match (&self.noise_reduction, &update.noise_reduction) {
                (Some(current), Some(next)) => Some(current.merge(next)),
                (None, Some(next)) => Some(next.clone()),
                (Some(current), None) => Some(current.clone()),
                (None, None) => None,
            },
            transcription: match (&self.transcription, &update.transcription) {
                (Some(current), Some(next)) => Some(current.merge(next)),
                (None, Some(next)) => Some(next.clone()),
                (Some(current), None) => Some(current.clone()),
                (None, None) => None,
            },
            turn_detection: match (&self.turn_detection, &update.turn_detection) {
                (Some(current), Some(next)) => Some(current.merge(next)),
                (None, Some(next)) => Some(next.clone()),
                (Some(current), None) => Some(current.clone()),
                (None, None) => None,
            },
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeAudioOutputConfig {
    pub format: Option<RealtimeAudioFormat>,
    pub voice: Option<String>,
    pub speed: Option<f32>,
}

impl RealtimeAudioOutputConfig {
    pub fn merge(&self, update: &Self) -> Self {
        Self {
            format: update.format.clone().or_else(|| self.format.clone()),
            voice: update.voice.clone().or_else(|| self.voice.clone()),
            speed: update.speed.or(self.speed),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeAudioConfig {
    pub input: Option<RealtimeAudioInputConfig>,
    pub output: Option<RealtimeAudioOutputConfig>,
}

impl RealtimeAudioConfig {
    pub fn merge(&self, update: &Self) -> Self {
        Self {
            input: match (&self.input, &update.input) {
                (Some(current), Some(next)) => Some(current.merge(next)),
                (None, Some(next)) => Some(next.clone()),
                (Some(current), None) => Some(current.clone()),
                (None, None) => None,
            },
            output: match (&self.output, &update.output) {
                (Some(current), Some(next)) => Some(current.merge(next)),
                (None, Some(next)) => Some(next.clone()),
                (Some(current), None) => Some(current.clone()),
                (None, None) => None,
            },
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeModelTracingConfig {
    pub workflow_name: Option<String>,
    pub group_id: Option<String>,
    pub metadata: Option<serde_json::Map<String, Value>>,
}

impl RealtimeModelTracingConfig {
    pub fn merge(&self, update: &Self) -> Self {
        Self {
            workflow_name: update
                .workflow_name
                .clone()
                .or_else(|| self.workflow_name.clone()),
            group_id: update.group_id.clone().or_else(|| self.group_id.clone()),
            metadata: update.metadata.clone().or_else(|| self.metadata.clone()),
        }
    }
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

impl RealtimeSessionModelSettings {
    pub fn merge(&self, update: &Self) -> Self {
        Self {
            model_name: update
                .model_name
                .clone()
                .or_else(|| self.model_name.clone()),
            instructions: update
                .instructions
                .clone()
                .or_else(|| self.instructions.clone()),
            modalities: update
                .modalities
                .clone()
                .or_else(|| self.modalities.clone()),
            output_modalities: update
                .output_modalities
                .clone()
                .or_else(|| self.output_modalities.clone()),
            audio: match (&self.audio, &update.audio) {
                (Some(current), Some(next)) => Some(current.merge(next)),
                (None, Some(next)) => Some(next.clone()),
                (Some(current), None) => Some(current.clone()),
                (None, None) => None,
            },
            voice: update.voice.clone().or_else(|| self.voice.clone()),
            speed: update.speed.or(self.speed),
            input_audio_format: update
                .input_audio_format
                .clone()
                .or_else(|| self.input_audio_format.clone()),
            output_audio_format: update
                .output_audio_format
                .clone()
                .or_else(|| self.output_audio_format.clone()),
            input_audio_transcription: match (
                &self.input_audio_transcription,
                &update.input_audio_transcription,
            ) {
                (Some(current), Some(next)) => Some(current.merge(next)),
                (None, Some(next)) => Some(next.clone()),
                (Some(current), None) => Some(current.clone()),
                (None, None) => None,
            },
            input_audio_noise_reduction: match (
                &self.input_audio_noise_reduction,
                &update.input_audio_noise_reduction,
            ) {
                (Some(current), Some(next)) => Some(current.merge(next)),
                (None, Some(next)) => Some(next.clone()),
                (Some(current), None) => Some(current.clone()),
                (None, None) => None,
            },
            turn_detection: match (&self.turn_detection, &update.turn_detection) {
                (Some(current), Some(next)) => Some(current.merge(next)),
                (None, Some(next)) => Some(next.clone()),
                (Some(current), None) => Some(current.clone()),
                (None, None) => None,
            },
            tool_choice: update
                .tool_choice
                .clone()
                .or_else(|| self.tool_choice.clone()),
            tracing: match (&self.tracing, &update.tracing) {
                (Some(current), Some(next)) => Some(current.merge(next)),
                (None, Some(next)) => Some(next.clone()),
                (Some(current), None) => Some(current.clone()),
                (None, None) => None,
            },
        }
    }
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
