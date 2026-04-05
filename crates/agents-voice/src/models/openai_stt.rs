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

#[derive(Clone, Debug, Default)]
pub struct OpenAISTTTranscriptionSession {
    pub settings: STTModelSettings,
    transcript: String,
    received_audio: bool,
    finished: bool,
    failure: Option<String>,
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
    async fn transcribe(&self, input: &AudioInput, _settings: &STTModelSettings) -> Result<String> {
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
        Ok(Box::new(OpenAISTTTranscriptionSession {
            settings: settings.clone(),
            transcript: String::new(),
            received_audio: false,
            finished: false,
            failure: None,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::{OpenAISTTModel, OpenAISTTTranscriptionSession};
    use crate::input::AudioInput;
    use crate::model::{STTModel, STTModelSettings, StreamedTranscriptionSession};

    #[tokio::test]
    async fn streamed_session_times_out_without_audio() {
        let mut session = OpenAISTTTranscriptionSession::default();

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
        let mut session = OpenAISTTTranscriptionSession {
            settings: STTModelSettings {
                model: Some("whisper-1".to_owned()),
                language: Some("en".to_owned()),
                prompt: Some("transcribe carefully".to_owned()),
            },
            ..OpenAISTTTranscriptionSession::default()
        };

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
        let mut session = OpenAISTTTranscriptionSession::default();
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
}
