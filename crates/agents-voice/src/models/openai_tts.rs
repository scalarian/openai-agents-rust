use async_trait::async_trait;

use agents_core::Result;
use agents_openai::OpenAIClientOptions;

use crate::events::{VoiceStreamEvent, VoiceStreamEventAudio, VoiceStreamEventLifecycle};
use crate::model::{TTSModel, TTSModelSettings};

#[derive(Clone, Debug)]
pub struct OpenAITTSModel {
    pub client_options: OpenAIClientOptions,
}

impl OpenAITTSModel {
    pub fn new(client_options: OpenAIClientOptions) -> Self {
        Self { client_options }
    }
}

#[async_trait]
impl TTSModel for OpenAITTSModel {
    async fn synthesize(
        &self,
        text: &str,
        _settings: &TTSModelSettings,
    ) -> Result<Vec<VoiceStreamEvent>> {
        Ok(vec![
            VoiceStreamEvent::Lifecycle(VoiceStreamEventLifecycle {
                event: "turn_started".to_owned(),
            }),
            VoiceStreamEvent::Audio(VoiceStreamEventAudio {
                data: Some(text.bytes().map(|b| b as f32).collect()),
            }),
            VoiceStreamEvent::Lifecycle(VoiceStreamEventLifecycle {
                event: "turn_ended".to_owned(),
            }),
        ])
    }
}
