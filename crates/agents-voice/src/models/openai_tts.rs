use async_trait::async_trait;

use agents_core::Result;
use agents_openai::OpenAIClientOptions;

use crate::events::{VoiceStreamEvent, VoiceStreamEventAudio, VoiceStreamEventLifecycle};
use crate::model::{TTSModel, TTSModelSettings};

const DEFAULT_VOICE: &str = "ash";

#[derive(Clone, Debug, PartialEq)]
struct OpenAITtsRequest {
    model: Option<String>,
    voice: String,
    speed: Option<f32>,
    text: String,
}

#[derive(Clone, Debug)]
pub struct OpenAITTSModel {
    pub client_options: OpenAIClientOptions,
}

impl OpenAITTSModel {
    pub fn new(client_options: OpenAIClientOptions) -> Self {
        Self { client_options }
    }

    fn build_request(&self, text: &str, settings: &TTSModelSettings) -> OpenAITtsRequest {
        OpenAITtsRequest {
            model: settings.model.clone(),
            voice: settings
                .voice
                .clone()
                .unwrap_or_else(|| DEFAULT_VOICE.to_owned()),
            speed: settings.speed,
            text: text.to_owned(),
        }
    }
}

#[async_trait]
impl TTSModel for OpenAITTSModel {
    async fn synthesize(
        &self,
        text: &str,
        settings: &TTSModelSettings,
    ) -> Result<Vec<VoiceStreamEvent>> {
        let request = self.build_request(text, settings);
        Ok(vec![
            VoiceStreamEvent::Lifecycle(VoiceStreamEventLifecycle {
                event: "turn_started".to_owned(),
            }),
            VoiceStreamEvent::Audio(VoiceStreamEventAudio {
                data: Some(request.text.bytes().map(|b| b as f32).collect()),
            }),
            VoiceStreamEvent::Lifecycle(VoiceStreamEventLifecycle {
                event: "turn_ended".to_owned(),
            }),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_VOICE, OpenAITTSModel};
    use crate::model::{TTSModel, TTSModelSettings};

    #[test]
    fn tts_request_defaults_to_ash_voice() {
        let model = OpenAITTSModel::new(Default::default());
        let request = model.build_request("hello", &TTSModelSettings::default());

        assert_eq!(request.voice, DEFAULT_VOICE);
        assert_eq!(request.text, "hello");
        assert_eq!(request.speed, None);
    }

    #[tokio::test]
    async fn tts_synthesize_keeps_custom_voice_and_speed_settings() {
        let model = OpenAITTSModel::new(Default::default());
        let settings = TTSModelSettings {
            model: Some("gpt-4o-mini-tts".to_owned()),
            voice: Some("fable".to_owned()),
            speed: Some(1.5),
        };
        let request = model.build_request("hi", &settings);

        assert_eq!(request.model.as_deref(), Some("gpt-4o-mini-tts"));
        assert_eq!(request.voice, "fable");
        assert_eq!(request.speed, Some(1.5));

        let events = model
            .synthesize("hi", &settings)
            .await
            .expect("tts synthesis should succeed");

        assert_eq!(events.len(), 3);
    }
}
