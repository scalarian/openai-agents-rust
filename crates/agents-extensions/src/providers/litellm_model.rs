use agents_core::{InputItem, Model, ModelRequest, ModelResponse, Result, ToolDefinition};
use agents_openai::{Converter, OpenAIChatCompletionsModel, OpenAIClientOptions};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Internal normalized message surface used by the LiteLLM adapter.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalChatCompletionMessage {
    pub role: String,
    pub content: Option<String>,
    pub reasoning_content: String,
    pub thinking_blocks: Option<Vec<Value>>,
}

/// Internal tool-call payload that preserves provider-specific metadata.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InternalToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
    pub extra_content: Option<Value>,
}

/// Helper for generating LiteLLM-compatible payloads from shared model input.
#[derive(Clone, Debug, Default)]
pub struct LitellmConverter;

impl LitellmConverter {
    pub fn payload(
        model: &str,
        instructions: Option<&str>,
        input: &[InputItem],
        tools: &[ToolDefinition],
    ) -> Value {
        Converter::payload(model, instructions, input, tools)
    }
}

/// Model adapter for LiteLLM-compatible chat-completions gateways.
#[derive(Clone, Debug)]
pub struct LitellmModel {
    pub model: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
}

impl LitellmModel {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            base_url: None,
            api_key: None,
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

    fn client_options(&self) -> OpenAIClientOptions {
        let mut options = OpenAIClientOptions::new(self.api_key.clone());
        if let Some(base_url) = &self.base_url {
            options = options.with_base_url(base_url.clone());
        }
        options
    }
}

#[async_trait]
impl Model for LitellmModel {
    async fn generate(&self, request: ModelRequest) -> Result<ModelResponse> {
        let model = OpenAIChatCompletionsModel::new(self.model.clone(), self.client_options());
        model.generate(request).await
    }
}
