use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use agents_core::{AgentsError, Result, default_openai_key};

use crate::RealtimeAudioFormat;
use crate::config::RealtimeSessionModelSettings;
use crate::model::{RealtimeModel, RealtimeModelConfig};
use crate::model_events::{
    RealtimeModelAudioDoneEvent, RealtimeModelAudioInterruptedEvent, RealtimeModelEvent,
    RealtimeModelResponseDoneEvent, RealtimeModelTranscriptDeltaEvent,
};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportConfig {
    pub api_key: Option<String>,
    pub websocket_url: Option<String>,
    pub call_id: Option<String>,
    #[serde(default)]
    pub query_params: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NormalizedRealtimeSessionPayload {
    pub model: Option<String>,
    pub input_audio_format: Option<RealtimeAudioFormat>,
    pub output_audio_format: Option<RealtimeAudioFormat>,
    pub payload: Value,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealtimeTransportRuntimeState {
    pub connected: bool,
    pub connection_url: Option<String>,
    pub api_key_present: bool,
    pub call_id: Option<String>,
    #[serde(default)]
    pub query_params: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeRuntimeState {
    pub model: Option<String>,
    pub transport: RealtimeTransportRuntimeState,
    pub last_session_payload: Option<NormalizedRealtimeSessionPayload>,
}

pub fn get_api_key(config: &TransportConfig) -> Option<String> {
    config.api_key.clone().or_else(default_openai_key)
}

pub fn get_server_event_type_adapter(event_type: &str) -> &str {
    match event_type {
        "response.audio_transcript.delta" | "response.output_audio_transcript.delta" => {
            "transcript_delta"
        }
        "response.done" => "response_done",
        other => other,
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OpenAIRealtimeWebSocketModel {
    pub config: RealtimeModelConfig,
    pub transport: TransportConfig,
    pub connected: bool,
    pub last_connection_url: Option<String>,
    pub last_session_payload: Option<NormalizedRealtimeSessionPayload>,
    pub applied_settings: Option<RealtimeSessionModelSettings>,
}

impl OpenAIRealtimeWebSocketModel {
    pub fn connection_url(&self) -> String {
        let base = self
            .transport
            .websocket_url
            .clone()
            .unwrap_or_else(|| "wss://api.openai.com/v1/realtime".to_owned());
        let mut url = if base.starts_with("https://") {
            base.replacen("https://", "wss://", 1)
        } else if base.starts_with("http://") {
            base.replacen("http://", "ws://", 1)
        } else {
            base
        };

        let mut query_params = self.transport.query_params.clone();
        if let Some(call_id) = &self.transport.call_id {
            query_params
                .entry("call_id".to_owned())
                .or_insert(call_id.clone());
        } else if let Some(model) = &self.config.model {
            query_params
                .entry("model".to_owned())
                .or_insert_with(|| model.clone());
        }

        if query_params.is_empty() {
            return url;
        }

        let separator = if url.contains('?') { '&' } else { '?' };
        let query = query_params
            .into_iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join("&");
        url.push(separator);
        url.push_str(&query);
        url
    }

    pub fn normalize_session_payload(payload: &Value) -> Option<NormalizedRealtimeSessionPayload> {
        let session_type = payload.get("type").and_then(Value::as_str)?;
        if session_type == "transcription" {
            return None;
        }
        if session_type != "realtime" {
            return None;
        }

        Some(NormalizedRealtimeSessionPayload {
            model: payload
                .get("model")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            input_audio_format: payload
                .get("audio")
                .and_then(|audio| audio.get("input"))
                .and_then(|input| input.get("format"))
                .and_then(Self::audio_format_from_value),
            output_audio_format: payload
                .get("audio")
                .and_then(|audio| audio.get("output"))
                .and_then(|output| output.get("format"))
                .and_then(Self::audio_format_from_value),
            payload: payload.clone(),
        })
    }

    fn audio_format_from_value(value: &Value) -> Option<RealtimeAudioFormat> {
        match value {
            Value::String(format) => Some(crate::to_realtime_audio_format(format)),
            Value::Object(map) => map
                .get("type")
                .and_then(Value::as_str)
                .map(crate::to_realtime_audio_format),
            _ => None,
        }
    }

    fn audio_format_payload(format: &RealtimeAudioFormat) -> Value {
        match format {
            RealtimeAudioFormat::Pcm16 => serde_json::json!({
                "type": "audio/pcm",
                "rate": 24_000,
            }),
            RealtimeAudioFormat::G711Ulaw => serde_json::json!({
                "type": "audio/pcmu",
            }),
            RealtimeAudioFormat::G711Alaw => serde_json::json!({
                "type": "audio/pcma",
            }),
            RealtimeAudioFormat::Custom(custom) => Value::String(custom.clone()),
        }
    }

    fn config_payload<T: Serialize>(value: &T) -> Option<Value> {
        let payload = serde_json::to_value(value).ok()?;
        match payload {
            Value::Null => None,
            Value::Object(ref map) if map.is_empty() => None,
            other => Some(other),
        }
    }

    fn session_payload_from_settings(&self, settings: &RealtimeSessionModelSettings) -> Value {
        let input_audio_format = settings
            .audio
            .as_ref()
            .and_then(|audio| audio.input.as_ref())
            .and_then(|input| input.format.clone())
            .or_else(|| settings.input_audio_format.clone());
        let output_audio_format = settings
            .audio
            .as_ref()
            .and_then(|audio| audio.output.as_ref())
            .and_then(|output| output.format.clone())
            .or_else(|| settings.output_audio_format.clone());
        let input_transcription = settings
            .audio
            .as_ref()
            .and_then(|audio| audio.input.as_ref())
            .and_then(|input| input.transcription.clone())
            .or_else(|| settings.input_audio_transcription.clone());
        let input_noise_reduction = settings
            .audio
            .as_ref()
            .and_then(|audio| audio.input.as_ref())
            .and_then(|input| input.noise_reduction.clone())
            .or_else(|| settings.input_audio_noise_reduction.clone());
        let turn_detection = settings
            .audio
            .as_ref()
            .and_then(|audio| audio.input.as_ref())
            .and_then(|input| input.turn_detection.clone())
            .or_else(|| settings.turn_detection.clone());
        let output_voice = settings
            .audio
            .as_ref()
            .and_then(|audio| audio.output.as_ref())
            .and_then(|output| output.voice.clone())
            .or_else(|| settings.voice.clone());
        let output_speed = settings
            .audio
            .as_ref()
            .and_then(|audio| audio.output.as_ref())
            .and_then(|output| output.speed)
            .or(settings.speed);

        let mut payload = serde_json::json!({
            "type": "realtime",
            "model": settings.model_name.clone().or_else(|| self.config.model.clone()),
        });
        let Some(session) = payload.as_object_mut() else {
            return payload;
        };

        if let Some(instructions) = &settings.instructions {
            session.insert(
                "instructions".to_owned(),
                Value::String(instructions.clone()),
            );
        }
        if let Some(modalities) = &settings.modalities {
            session.insert("modalities".to_owned(), serde_json::json!(modalities));
        }
        if let Some(output_modalities) = &settings.output_modalities {
            session.insert(
                "output_modalities".to_owned(),
                serde_json::json!(output_modalities),
            );
        }
        if let Some(tool_choice) = &settings.tool_choice {
            session.insert("tool_choice".to_owned(), Value::String(tool_choice.clone()));
        }
        if let Some(tracing) = settings.tracing.as_ref().and_then(Self::config_payload) {
            session.insert("tracing".to_owned(), tracing);
        }

        let mut audio = serde_json::Map::new();
        let mut input = serde_json::Map::new();
        if let Some(input_audio_format) = input_audio_format {
            input.insert(
                "format".to_owned(),
                Self::audio_format_payload(&input_audio_format),
            );
        }
        if let Some(transcription) = input_transcription.as_ref().and_then(Self::config_payload) {
            input.insert("transcription".to_owned(), transcription);
        }
        if let Some(noise_reduction) = input_noise_reduction
            .as_ref()
            .and_then(Self::config_payload)
        {
            input.insert("noise_reduction".to_owned(), noise_reduction);
        }
        if let Some(turn_detection) = turn_detection.as_ref().and_then(Self::config_payload) {
            input.insert("turn_detection".to_owned(), turn_detection);
        }
        if !input.is_empty() {
            audio.insert("input".to_owned(), Value::Object(input));
        }

        let mut output = serde_json::Map::new();
        if let Some(output_audio_format) = output_audio_format {
            output.insert(
                "format".to_owned(),
                Self::audio_format_payload(&output_audio_format),
            );
        }
        if let Some(output_voice) = output_voice {
            output.insert("voice".to_owned(), Value::String(output_voice));
        }
        if let Some(output_speed) = output_speed {
            output.insert("speed".to_owned(), serde_json::json!(output_speed));
        }
        if !output.is_empty() {
            audio.insert("output".to_owned(), Value::Object(output));
        }

        if !audio.is_empty() {
            session.insert("audio".to_owned(), Value::Object(audio));
        }

        payload
    }

    pub fn runtime_state(&self) -> RealtimeRuntimeState {
        RealtimeRuntimeState {
            model: self.config.model.clone(),
            transport: RealtimeTransportRuntimeState {
                connected: self.connected,
                connection_url: self.last_connection_url.clone(),
                api_key_present: get_api_key(&self.transport).is_some(),
                call_id: self.transport.call_id.clone(),
                query_params: self.transport.query_params.clone(),
            },
            last_session_payload: self.last_session_payload.clone(),
        }
    }
}

#[async_trait]
impl RealtimeModel for OpenAIRealtimeWebSocketModel {
    async fn connect(&mut self) -> Result<()> {
        self.last_connection_url = Some(self.connection_url());
        self.connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }

    async fn send_text(&mut self, text: &str) -> Result<Vec<RealtimeModelEvent>> {
        Ok(vec![
            RealtimeModelEvent::TranscriptDelta(RealtimeModelTranscriptDeltaEvent {
                text: text.to_owned(),
            }),
            RealtimeModelEvent::ResponseDone(RealtimeModelResponseDoneEvent {
                response_id: None,
                request_id: None,
                payload: Some(Value::String(text.to_owned())),
            }),
        ])
    }

    async fn send_audio(&mut self, bytes: &[u8]) -> Result<Vec<RealtimeModelEvent>> {
        Ok(vec![RealtimeModelEvent::AudioDone(
            RealtimeModelAudioDoneEvent {
                total_bytes: bytes.len(),
            },
        )])
    }

    async fn interrupt(&mut self) -> Result<Vec<RealtimeModelEvent>> {
        Ok(vec![RealtimeModelEvent::AudioInterrupted(
            RealtimeModelAudioInterruptedEvent {
                reason: Some("interrupted".to_owned()),
            },
        )])
    }

    async fn update_session(
        &mut self,
        settings: &RealtimeSessionModelSettings,
    ) -> Result<Vec<RealtimeModelEvent>> {
        let merged_settings = self
            .applied_settings
            .as_ref()
            .map(|current| current.merge(settings))
            .unwrap_or_else(|| settings.clone());
        if let Some(model_name) = &merged_settings.model_name {
            self.config.model = Some(model_name.clone());
        }
        let payload = self.session_payload_from_settings(&merged_settings);
        self.last_session_payload = Self::normalize_session_payload(&payload);
        self.applied_settings = Some(merged_settings);
        Ok(Vec::new())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OpenAIRealtimeSIPModel {
    pub config: RealtimeModelConfig,
    pub transport: TransportConfig,
    pub connected: bool,
    pub last_connection_url: Option<String>,
    pub last_session_payload: Option<NormalizedRealtimeSessionPayload>,
    pub applied_settings: Option<RealtimeSessionModelSettings>,
}

impl OpenAIRealtimeSIPModel {
    pub fn connection_url(&self) -> String {
        OpenAIRealtimeWebSocketModel {
            config: self.config.clone(),
            transport: self.transport.clone(),
            connected: self.connected,
            last_connection_url: self.last_connection_url.clone(),
            last_session_payload: self.last_session_payload.clone(),
            applied_settings: self.applied_settings.clone(),
        }
        .connection_url()
    }

    pub fn runtime_state(&self) -> RealtimeRuntimeState {
        RealtimeRuntimeState {
            model: self.config.model.clone(),
            transport: RealtimeTransportRuntimeState {
                connected: self.connected,
                connection_url: self.last_connection_url.clone(),
                api_key_present: get_api_key(&self.transport).is_some(),
                call_id: self.transport.call_id.clone(),
                query_params: self.transport.query_params.clone(),
            },
            last_session_payload: self.last_session_payload.clone(),
        }
    }
}

#[async_trait]
impl RealtimeModel for OpenAIRealtimeSIPModel {
    async fn connect(&mut self) -> Result<()> {
        if self.transport.call_id.is_none() {
            return Err(AgentsError::message(
                "OpenAIRealtimeSIPModel requires `call_id` in the transport configuration.",
            ));
        }
        self.last_connection_url = Some(self.connection_url());
        self.connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }

    async fn send_text(&mut self, text: &str) -> Result<Vec<RealtimeModelEvent>> {
        Ok(vec![
            RealtimeModelEvent::TranscriptDelta(RealtimeModelTranscriptDeltaEvent {
                text: text.to_owned(),
            }),
            RealtimeModelEvent::ResponseDone(RealtimeModelResponseDoneEvent {
                response_id: None,
                request_id: None,
                payload: Some(serde_json::json!({
                    "transport": "sip",
                    "text": text,
                })),
            }),
        ])
    }

    async fn send_audio(&mut self, bytes: &[u8]) -> Result<Vec<RealtimeModelEvent>> {
        Ok(vec![RealtimeModelEvent::AudioDone(
            RealtimeModelAudioDoneEvent {
                total_bytes: bytes.len(),
            },
        )])
    }

    async fn interrupt(&mut self) -> Result<Vec<RealtimeModelEvent>> {
        Ok(vec![RealtimeModelEvent::AudioInterrupted(
            RealtimeModelAudioInterruptedEvent {
                reason: Some("interrupted".to_owned()),
            },
        )])
    }

    async fn update_session(
        &mut self,
        settings: &RealtimeSessionModelSettings,
    ) -> Result<Vec<RealtimeModelEvent>> {
        let merged_settings = self
            .applied_settings
            .as_ref()
            .map(|current| current.merge(settings))
            .unwrap_or_else(|| settings.clone());
        if let Some(model_name) = &merged_settings.model_name {
            self.config.model = Some(model_name.clone());
        }
        let payload = OpenAIRealtimeWebSocketModel {
            config: self.config.clone(),
            transport: self.transport.clone(),
            connected: self.connected,
            last_connection_url: self.last_connection_url.clone(),
            last_session_payload: self.last_session_payload.clone(),
            applied_settings: self.applied_settings.clone(),
        }
        .session_payload_from_settings(&merged_settings);
        self.last_session_payload =
            OpenAIRealtimeWebSocketModel::normalize_session_payload(&payload);
        self.applied_settings = Some(merged_settings);
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapts_known_server_event_types() {
        assert_eq!(
            get_server_event_type_adapter("response.audio_transcript.delta"),
            "transcript_delta"
        );
        assert_eq!(
            get_server_event_type_adapter("response.done"),
            "response_done"
        );
        assert_eq!(
            get_server_event_type_adapter("response.output_audio_transcript.delta"),
            "transcript_delta"
        );
        assert_eq!(get_server_event_type_adapter("custom"), "custom");
    }

    #[test]
    fn normalizes_http_transport_url_and_query_parameters() {
        let model = OpenAIRealtimeWebSocketModel {
            config: RealtimeModelConfig {
                model: Some("gpt-realtime".to_owned()),
            },
            transport: TransportConfig {
                api_key: None,
                websocket_url: Some("https://api.openai.com/v1/realtime".to_owned()),
                call_id: None,
                query_params: BTreeMap::new(),
            },
            connected: false,
            last_connection_url: None,
            last_session_payload: None,
            applied_settings: None,
        };

        assert_eq!(
            model.connection_url(),
            "wss://api.openai.com/v1/realtime?model=gpt-realtime"
        );
    }

    #[test]
    fn normalizes_realtime_session_payload_and_extracts_audio_format() {
        let payload = serde_json::json!({
            "type": "realtime",
            "model": "gpt-realtime-1.5",
            "audio": {
                "output": {
                    "format": { "type": "audio/pcmu" }
                }
            }
        });

        let normalized = OpenAIRealtimeWebSocketModel::normalize_session_payload(&payload)
            .expect("payload should normalize");
        assert_eq!(
            normalized.output_audio_format,
            Some(crate::RealtimeAudioFormat::G711Ulaw)
        );

        let transcription = serde_json::json!({ "type": "transcription" });
        assert!(OpenAIRealtimeWebSocketModel::normalize_session_payload(&transcription).is_none());
    }

    #[tokio::test]
    async fn websocket_model_tracks_connection_and_updates_session_model() {
        let mut model = OpenAIRealtimeWebSocketModel::default();
        model.connect().await.expect("connect should succeed");
        assert!(model.connected);
        assert_eq!(
            model.runtime_state().transport.connection_url.as_deref(),
            Some("wss://api.openai.com/v1/realtime")
        );

        let events = model.send_text("hello").await.expect("text should send");
        assert!(matches!(
            events.first(),
            Some(RealtimeModelEvent::TranscriptDelta(_))
        ));

        model
            .update_session(&RealtimeSessionModelSettings {
                model_name: Some("gpt-realtime-updated".to_owned()),
                audio: Some(crate::RealtimeAudioConfig {
                    output: Some(crate::RealtimeAudioOutputConfig {
                        voice: Some("marin".to_owned()),
                        speed: Some(1.25),
                        ..crate::RealtimeAudioOutputConfig::default()
                    }),
                    ..crate::RealtimeAudioConfig::default()
                }),
                ..RealtimeSessionModelSettings::default()
            })
            .await
            .expect("session should update");
        assert_eq!(model.config.model.as_deref(), Some("gpt-realtime-updated"));
        let runtime_state = model.runtime_state();
        let output = runtime_state
            .last_session_payload
            .as_ref()
            .and_then(|payload| payload.payload.get("audio"))
            .and_then(|audio| audio.get("output"))
            .expect("output payload should exist");
        assert_eq!(output.get("voice").and_then(Value::as_str), Some("marin"));
        assert_eq!(output.get("speed").and_then(Value::as_f64), Some(1.25));

        model.disconnect().await.expect("disconnect should succeed");
        assert!(!model.connected);
    }

    #[tokio::test]
    async fn sip_model_supports_text_audio_and_interrupt() {
        let mut model = OpenAIRealtimeSIPModel {
            transport: TransportConfig {
                call_id: Some("call_123".to_owned()),
                ..TransportConfig::default()
            },
            ..OpenAIRealtimeSIPModel::default()
        };
        model.connect().await.expect("connect should succeed");
        assert!(model.connected);
        assert_eq!(
            model.runtime_state().transport.call_id.as_deref(),
            Some("call_123")
        );

        let text_events = model.send_text("hello").await.expect("text should send");
        assert!(matches!(
            text_events.last(),
            Some(RealtimeModelEvent::ResponseDone(_))
        ));

        let audio_events = model
            .send_audio(&[1, 2, 3])
            .await
            .expect("audio should send");
        assert!(matches!(
            audio_events.first(),
            Some(RealtimeModelEvent::AudioDone(_))
        ));

        let interrupt_events = model.interrupt().await.expect("interrupt should succeed");
        assert!(matches!(
            interrupt_events.first(),
            Some(RealtimeModelEvent::AudioInterrupted(_))
        ));

        model
            .update_session(&RealtimeSessionModelSettings {
                audio: Some(crate::RealtimeAudioConfig {
                    output: Some(crate::RealtimeAudioOutputConfig {
                        voice: Some("verse".to_owned()),
                        speed: Some(1.5),
                        ..crate::RealtimeAudioOutputConfig::default()
                    }),
                    ..crate::RealtimeAudioConfig::default()
                }),
                ..RealtimeSessionModelSettings::default()
            })
            .await
            .expect("session should update");
        let runtime_state = model.runtime_state();
        assert_eq!(
            runtime_state.transport.connection_url.as_deref(),
            Some("wss://api.openai.com/v1/realtime?call_id=call_123")
        );
        let output = runtime_state
            .last_session_payload
            .as_ref()
            .and_then(|payload| payload.payload.get("audio"))
            .and_then(|audio| audio.get("output"))
            .expect("output payload should exist");
        assert_eq!(output.get("voice").and_then(Value::as_str), Some("verse"));
        assert_eq!(output.get("speed").and_then(Value::as_f64), Some(1.5));
    }

    #[tokio::test]
    async fn sip_model_requires_call_id_before_connect() {
        let mut model = OpenAIRealtimeSIPModel::default();
        let error = model.connect().await.expect_err("connect should fail");
        assert!(error.to_string().contains("call_id"));
    }
}
