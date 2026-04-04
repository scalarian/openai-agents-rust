use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use agents_core::Result;

use crate::events::VoiceStreamEvent;
use crate::input::AudioInput;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TTSModelSettings {
    pub model: Option<String>,
    pub voice: Option<String>,
    pub speed: Option<f32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct STTModelSettings {
    pub model: Option<String>,
    pub language: Option<String>,
    pub prompt: Option<String>,
}

#[async_trait]
pub trait StreamedTranscriptionSession: Send + Sync {
    async fn push_audio(&mut self, chunk: &[u8]) -> Result<()>;
    async fn finish(&mut self) -> Result<String>;
}

#[async_trait]
pub trait STTModel: Send + Sync {
    async fn transcribe(&self, input: &AudioInput, settings: &STTModelSettings) -> Result<String>;
    async fn start_session(
        &self,
        settings: &STTModelSettings,
    ) -> Result<Box<dyn StreamedTranscriptionSession>>;
}

#[async_trait]
pub trait TTSModel: Send + Sync {
    async fn synthesize(
        &self,
        text: &str,
        settings: &TTSModelSettings,
    ) -> Result<Vec<VoiceStreamEvent>>;
}

pub trait VoiceModelProvider: Send + Sync {
    fn stt_model(&self) -> Box<dyn STTModel>;
    fn tts_model(&self) -> Box<dyn TTSModel>;
}
