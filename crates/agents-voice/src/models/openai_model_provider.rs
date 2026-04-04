use reqwest::Client;

use agents_openai::OpenAIClientOptions;

use crate::model::{STTModel, TTSModel, VoiceModelProvider};
use crate::openai_stt::OpenAISTTModel;
use crate::openai_tts::OpenAITTSModel;

pub fn shared_http_client() -> Client {
    Client::new()
}

#[derive(Clone, Debug, Default)]
pub struct OpenAIVoiceModelProvider {
    pub client_options: OpenAIClientOptions,
}

impl OpenAIVoiceModelProvider {
    pub fn new(client_options: OpenAIClientOptions) -> Self {
        Self { client_options }
    }
}

impl VoiceModelProvider for OpenAIVoiceModelProvider {
    fn stt_model(&self) -> Box<dyn STTModel> {
        Box::new(OpenAISTTModel::new(self.client_options.clone()))
    }

    fn tts_model(&self) -> Box<dyn TTSModel> {
        Box::new(OpenAITTSModel::new(self.client_options.clone()))
    }
}
