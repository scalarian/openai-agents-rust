use agents_core::{Model, ModelRequest, ModelResponse, Result};
use agents_openai::{OpenAIChatCompletionsModel, OpenAIClientOptions, OpenAIResponsesModel};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Internal normalized chat-completion message used by the AnyLLM adapter.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalChatCompletionMessage {
    pub role: String,
    pub content: Option<String>,
    pub reasoning_content: String,
}

/// Transport surface exposed by any-llm.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnyLLMApi {
    Responses,
    ChatCompletions,
}

/// Model adapter for OpenAI-compatible any-llm gateways.
#[derive(Clone, Debug)]
pub struct AnyLLMModel {
    pub model: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub api: Option<AnyLLMApi>,
}

impl AnyLLMModel {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            base_url: None,
            api_key: None,
            api: None,
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn with_api(mut self, api: AnyLLMApi) -> Self {
        self.api = Some(api);
        self
    }

    fn client_options(&self) -> OpenAIClientOptions {
        let mut options = OpenAIClientOptions::new(self.api_key.clone());
        if let Some(base_url) = &self.base_url {
            options = options.with_base_url(base_url.clone());
        }
        options
    }
}

#[async_trait]
impl Model for AnyLLMModel {
    async fn generate(&self, request: ModelRequest) -> Result<ModelResponse> {
        match self.api.unwrap_or(AnyLLMApi::ChatCompletions) {
            AnyLLMApi::Responses => {
                let model = OpenAIResponsesModel::new(self.model.clone(), self.client_options());
                model.generate(request).await
            }
            AnyLLMApi::ChatCompletions => {
                let model =
                    OpenAIChatCompletionsModel::new(self.model.clone(), self.client_options());
                model.generate(request).await
            }
        }
    }
}
