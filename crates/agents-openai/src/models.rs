use agents_core::{InputItem, Model, ModelRequest, ModelResponse, OutputItem, Result};
use async_trait::async_trait;

#[derive(Clone, Debug)]
pub struct OpenAIResponsesModel {
    model: String,
    api_key: Option<String>,
    base_url: Option<String>,
}

impl OpenAIResponsesModel {
    pub fn new(
        model: impl Into<String>,
        api_key: Option<String>,
        base_url: Option<String>,
    ) -> Self {
        Self {
            model: model.into(),
            api_key,
            base_url,
        }
    }
}

#[async_trait]
impl Model for OpenAIResponsesModel {
    async fn generate(&self, request: ModelRequest) -> Result<ModelResponse> {
        let _ = (&self.api_key, &self.base_url);
        let text = request
            .input
            .iter()
            .rev()
            .find_map(InputItem::as_text)
            .unwrap_or_default();

        Ok(ModelResponse {
            model: Some(self.model.clone()),
            output: vec![OutputItem::Text {
                text: format!("responses:{}:{}", self.model, text),
            }],
            usage: agents_core::Usage::default(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct OpenAIChatCompletionsModel {
    model: String,
    api_key: Option<String>,
    base_url: Option<String>,
}

impl OpenAIChatCompletionsModel {
    pub fn new(
        model: impl Into<String>,
        api_key: Option<String>,
        base_url: Option<String>,
    ) -> Self {
        Self {
            model: model.into(),
            api_key,
            base_url,
        }
    }
}

#[async_trait]
impl Model for OpenAIChatCompletionsModel {
    async fn generate(&self, request: ModelRequest) -> Result<ModelResponse> {
        let _ = (&self.api_key, &self.base_url);
        let text = request
            .input
            .iter()
            .rev()
            .find_map(InputItem::as_text)
            .unwrap_or_default();

        Ok(ModelResponse {
            model: Some(self.model.clone()),
            output: vec![OutputItem::Text {
                text: format!("chat_completions:{}:{}", self.model, text),
            }],
            usage: agents_core::Usage::default(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct OpenAIResponsesWsModel {
    pub model: String,
}

impl OpenAIResponsesWsModel {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
        }
    }
}
