use std::sync::Arc;

use agents_core::{Model, ModelProvider};

use crate::providers::any_llm_model::{AnyLLMApi, AnyLLMModel};

/// Provider that routes model requests through an any-llm compatible gateway.
#[derive(Clone, Debug, Default)]
pub struct AnyLLMProvider {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub api: Option<AnyLLMApi>,
}

impl AnyLLMProvider {
    pub fn new() -> Self {
        Self::default()
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
}

impl ModelProvider for AnyLLMProvider {
    fn resolve(&self, model: Option<&str>) -> Arc<dyn Model> {
        let model_name = model.unwrap_or("openai/gpt-5");
        let mut resolved = AnyLLMModel::new(model_name);
        if let Some(base_url) = &self.base_url {
            resolved = resolved.with_base_url(base_url.clone());
        }
        if let Some(api_key) = &self.api_key {
            resolved = resolved.with_api_key(api_key.clone());
        }
        if let Some(api) = self.api {
            resolved = resolved.with_api(api);
        }
        Arc::new(resolved)
    }
}
