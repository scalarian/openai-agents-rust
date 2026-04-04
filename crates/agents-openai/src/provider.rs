use std::sync::Arc;

use agents_core::{Model, ModelProvider};

use crate::defaults::{OpenAIApi, default_openai_api};
use crate::models::{OpenAIChatCompletionsModel, OpenAIResponsesModel};

#[derive(Clone, Debug, Default)]
pub struct OpenAIProvider {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub api: Option<OpenAIApi>,
}

impl OpenAIProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    pub fn with_api(mut self, api: OpenAIApi) -> Self {
        self.api = Some(api);
        self
    }
}

impl ModelProvider for OpenAIProvider {
    fn resolve(&self, model: Option<&str>) -> Arc<dyn Model> {
        match self.api.unwrap_or_else(default_openai_api) {
            OpenAIApi::ChatCompletions => Arc::new(OpenAIChatCompletionsModel::new(
                model.unwrap_or("gpt-4.1"),
                self.api_key.clone(),
                self.base_url.clone(),
            )),
            OpenAIApi::Responses => Arc::new(OpenAIResponsesModel::new(
                model.unwrap_or("gpt-5"),
                self.api_key.clone(),
                self.base_url.clone(),
            )),
        }
    }
}
