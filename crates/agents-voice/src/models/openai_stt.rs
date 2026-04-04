use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use agents_core::Result;
use agents_openai::OpenAIClientOptions;

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
}

#[async_trait]
impl StreamedTranscriptionSession for OpenAISTTTranscriptionSession {
    async fn push_audio(&mut self, chunk: &[u8]) -> Result<()> {
        self.transcript.push_str(&format!("[{}]", chunk.len()));
        Ok(())
    }

    async fn finish(&mut self) -> Result<String> {
        Ok(std::mem::take(&mut self.transcript))
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
        }))
    }
}
