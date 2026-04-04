use std::sync::Arc;

use agents_core::{Model, ModelProvider};

use crate::providers::litellm_model::LitellmModel;

/// Provider that routes requests through a LiteLLM gateway.
#[derive(Clone, Debug, Default)]
pub struct LitellmProvider {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
}

impl LitellmProvider {
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
}

impl ModelProvider for LitellmProvider {
    fn resolve(&self, model: Option<&str>) -> Arc<dyn Model> {
        let model_name = model.unwrap_or("gpt-4.1");
        let mut resolved = LitellmModel::new(model_name);
        if let Some(base_url) = &self.base_url {
            resolved = resolved.with_base_url(base_url.clone());
        }
        if let Some(api_key) = &self.api_key {
            resolved = resolved.with_api_key(api_key.clone());
        }
        Arc::new(resolved)
    }
}
