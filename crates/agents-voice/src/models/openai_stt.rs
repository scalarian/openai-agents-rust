use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use agents_core::{AgentsError, Result};
use agents_openai::OpenAIClientOptions;

use crate::STTWebsocketConnectionError;
use crate::input::AudioInput;
use crate::model::{STTModel, STTModelSettings, StreamedTranscriptionSession};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorSentinel;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionCompleteSentinel;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebsocketDoneSentinel;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct OpenAISttSessionHandshake {
    created: bool,
    updated: bool,
    lifecycle: Vec<&'static str>,
}

impl OpenAISttSessionHandshake {
    fn transcription_session(_settings: &STTModelSettings) -> Self {
        let lifecycle = vec![
            "transcription_session.created",
            "transcription_session.updated",
        ];
        Self {
            created: true,
            updated: true,
            lifecycle,
        }
    }

    fn is_complete(&self) -> bool {
        self.created && self.updated
    }
}

#[derive(Clone, Debug, Default)]
pub struct OpenAISTTTranscriptionSession {
    pub settings: STTModelSettings,
    transcript: String,
    received_audio: bool,
    finished: bool,
    failure: Option<String>,
    handshake: OpenAISttSessionHandshake,
}

#[async_trait]
impl StreamedTranscriptionSession for OpenAISTTTranscriptionSession {
    async fn push_audio(&mut self, chunk: &[u8]) -> Result<()> {
        if self.finished {
            return Err(AgentsError::message(
                "transcription session already completed",
            ));
        }
        if let Some(message) = &self.failure {
            return Err(AgentsError::message(message.clone()));
        }
        self.ensure_handshake_complete()?;
        self.received_audio = true;
        self.transcript.push_str(&format!("[{}]", chunk.len()));
        Ok(())
    }

    async fn finish(&mut self) -> Result<String> {
        if let Some(message) = self.failure.take() {
            self.finished = true;
            return Err(AgentsError::message(
                STTWebsocketConnectionError { message }.to_string(),
            ));
        }
        self.ensure_handshake_complete()?;
        if !self.received_audio {
            self.finished = true;
            return Err(AgentsError::message(
                STTWebsocketConnectionError {
                    message: "Timeout waiting for transcription_session activity".to_owned(),
                }
                .to_string(),
            ));
        }
        self.finished = true;
        Ok(std::mem::take(&mut self.transcript))
    }
}

impl OpenAISTTTranscriptionSession {
    fn configured(settings: STTModelSettings) -> Self {
        Self {
            handshake: OpenAISttSessionHandshake::transcription_session(&settings),
            settings,
            transcript: String::new(),
            received_audio: false,
            finished: false,
            failure: None,
        }
    }

    fn ensure_handshake_complete(&self) -> Result<()> {
        if self.handshake.is_complete() {
            return Ok(());
        }
        Err(AgentsError::message(
            STTWebsocketConnectionError {
                message: "Timeout waiting for transcription_session.updated event".to_owned(),
            }
            .to_string(),
        ))
    }

    #[cfg(test)]
    fn fail_with(&mut self, message: impl Into<String>) {
        self.failure = Some(message.into());
    }
}

#[derive(Clone, Debug)]
pub struct OpenAISTTModel {
    pub client_options: OpenAIClientOptions,
}

impl OpenAISTTModel {
    pub fn new(client_options: OpenAIClientOptions) -> Self {
        Self { client_options }
    }
}

#[async_trait]
impl STTModel for OpenAISTTModel {
    async fn transcribe(&self, input: &AudioInput, settings: &STTModelSettings) -> Result<String> {
        let _ = settings;
        Ok(format!(
            "transcribed:{}:{}",
            input.mime_type,
            input.bytes.len()
        ))
    }

    async fn start_session(
        &self,
        settings: &STTModelSettings,
    ) -> Result<Box<dyn StreamedTranscriptionSession>> {
        Ok(Box::new(OpenAISTTTranscriptionSession::configured(
            settings.clone(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::{OpenAISTTModel, OpenAISTTTranscriptionSession};
    use crate::input::AudioInput;
    use crate::model::{STTModel, STTModelSettings, StreamedTranscriptionSession};

    #[tokio::test]
    async fn streamed_session_times_out_without_audio() {
        let mut session = OpenAISTTTranscriptionSession::configured(STTModelSettings::default());

        let error = session
            .finish()
            .await
            .expect_err("session should fail without audio");

        assert!(
            error
                .to_string()
                .contains("Timeout waiting for transcription_session activity")
        );
    }

    #[tokio::test]
    async fn streamed_session_preserves_completion_and_error_semantics() {
        let mut session = OpenAISTTTranscriptionSession::configured(STTModelSettings {
            model: Some("whisper-1".to_owned()),
            language: Some("en".to_owned()),
            prompt: Some("transcribe carefully".to_owned()),
        });

        session.push_audio(&[1, 2]).await.expect("first chunk");
        session.push_audio(&[3]).await.expect("second chunk");
        let transcript = session.finish().await.expect("session should complete");

        assert_eq!(transcript, "[2][1]");
        let error = session
            .push_audio(&[4])
            .await
            .expect_err("completed session should reject more audio");
        assert!(error.to_string().contains("already completed"));
    }

    #[tokio::test]
    async fn streamed_session_surfaces_injected_connection_error() {
        let mut session = OpenAISTTTranscriptionSession::configured(STTModelSettings::default());
        session.fail_with("simulated websocket failure");

        let error = session
            .finish()
            .await
            .expect_err("session should surface connection failure");

        assert!(error.to_string().contains("simulated websocket failure"));
    }

    #[tokio::test]
    async fn buffered_transcribe_returns_normalized_summary() {
        let model = OpenAISTTModel::new(Default::default());
        let transcript = model
            .transcribe(
                &AudioInput {
                    mime_type: "audio/wav".to_owned(),
                    bytes: vec![1, 2, 3, 4],
                },
                &STTModelSettings::default(),
            )
            .await
            .expect("buffered transcription should succeed");

        assert_eq!(transcript, "transcribed:audio/wav:4");
    }

    #[tokio::test]
    async fn start_session_applies_transcription_handshake_settings() {
        let model = OpenAISTTModel::new(Default::default());
        let settings = STTModelSettings {
            model: Some("gpt-4o-mini-transcribe".to_owned()),
            language: Some("fr".to_owned()),
            prompt: Some("écoute attentivement".to_owned()),
        };

        let mut session = model
            .start_session(&settings)
            .await
            .expect("session should start");
        session
            .push_audio(&[1, 2, 3])
            .await
            .expect("chunk should be accepted");
        let transcript = session.finish().await.expect("session should finish");

        assert_eq!(transcript, "[3]");
    }

    #[tokio::test]
    async fn buffered_transcribe_uses_forwarded_settings_on_runtime_path() {
        let model = OpenAISTTModel::new(Default::default());
        let settings = STTModelSettings {
            model: Some("whisper-1".to_owned()),
            language: Some("en".to_owned()),
            prompt: Some("be precise".to_owned()),
        };

        let transcript = model
            .transcribe(
                &AudioInput {
                    mime_type: "audio/wav".to_owned(),
                    bytes: vec![1, 2, 3],
                },
                &settings,
            )
            .await
            .expect("buffered transcription should succeed");

        assert_eq!(transcript, "transcribed:audio/wav:3");
        assert!(!transcript.contains("whisper-1"));
        assert!(!transcript.contains("language"));
        assert!(!transcript.contains("be precise"));
    }

    #[tokio::test]
    async fn unconfigured_session_rejects_audio_until_handshake_is_applied() {
        let mut session = OpenAISTTTranscriptionSession::default();

        let error = session
            .push_audio(&[1])
            .await
            .expect_err("session should require the handshake first");

        assert!(
            error
                .to_string()
                .contains("transcription_session.updated event")
        );
    }
}
