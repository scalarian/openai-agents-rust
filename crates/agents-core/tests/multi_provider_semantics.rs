use std::sync::{Arc, Mutex};

use agents_core::{
    AgentsError, Model, ModelProvider, ModelRequest, ModelResponse, MultiProvider,
    MultiProviderMap, MultiProviderOpenAIPrefixMode, MultiProviderUnknownPrefixMode, Result,
};
use async_trait::async_trait;

#[derive(Default)]
struct CaptureModel {
    seen: Mutex<Vec<Option<String>>>,
}

#[async_trait]
impl Model for CaptureModel {
    async fn generate(&self, request: ModelRequest) -> Result<ModelResponse> {
        self.seen.lock().expect("seen lock").push(request.model);
        Ok(ModelResponse::default())
    }
}

#[derive(Clone)]
struct CaptureProvider {
    model: Arc<CaptureModel>,
}

impl CaptureProvider {
    fn new(model: Arc<CaptureModel>) -> Self {
        Self { model }
    }
}

impl ModelProvider for CaptureProvider {
    fn resolve(&self, model: Option<&str>) -> Arc<dyn Model> {
        Arc::new(CapturedModel {
            inner: self.model.clone(),
            resolved_model: model.map(ToOwned::to_owned),
        })
    }
}

struct CapturedModel {
    inner: Arc<CaptureModel>,
    resolved_model: Option<String>,
}

#[async_trait]
impl Model for CapturedModel {
    async fn generate(&self, mut request: ModelRequest) -> Result<ModelResponse> {
        request.model = self.resolved_model.clone();
        self.inner.generate(request).await
    }
}

struct ErroringModel {
    message: String,
}

#[async_trait]
impl Model for ErroringModel {
    async fn generate(&self, _request: ModelRequest) -> Result<ModelResponse> {
        Err(AgentsError::message(self.message.clone()))
    }
}

#[derive(Clone)]
struct ErroringProvider;

impl ModelProvider for ErroringProvider {
    fn resolve(&self, model: Option<&str>) -> Arc<dyn Model> {
        Arc::new(ErroringModel {
            message: format!("default provider unexpectedly received {:?}", model),
        })
    }
}

#[tokio::test]
async fn multi_provider_routes_models_predictably() {
    let default_capture = Arc::new(CaptureModel::default());
    let openrouter_capture = Arc::new(CaptureModel::default());
    let explicit_openai_capture = Arc::new(CaptureModel::default());

    MultiProvider::new(Arc::new(CaptureProvider::new(default_capture.clone())))
        .resolve(Some("openai/gpt-5"))
        .generate(ModelRequest::default())
        .await
        .expect("alias mode should strip the openai prefix");

    MultiProvider::new(Arc::new(CaptureProvider::new(default_capture.clone())))
        .with_openai_prefix_mode(MultiProviderOpenAIPrefixMode::ModelId)
        .resolve(Some("openai/gpt-5"))
        .generate(ModelRequest::default())
        .await
        .expect("model-id mode should preserve the openai prefix");

    MultiProvider::new(Arc::new(CaptureProvider::new(default_capture.clone())))
        .with_unknown_prefix_mode(MultiProviderUnknownPrefixMode::ModelId)
        .resolve(Some("unknown/gpt-5"))
        .generate(ModelRequest::default())
        .await
        .expect("unknown prefixes should be preserved in model-id mode");

    let unknown_error = MultiProvider::new(Arc::new(ErroringProvider))
        .resolve(Some("unknown/gpt-5"))
        .generate(ModelRequest::default())
        .await
        .expect_err("unknown prefixes should error by default");
    assert!(
        unknown_error
            .to_string()
            .contains("unknown model provider prefix `unknown`")
    );

    let mut provider_map = MultiProviderMap::default();
    provider_map.add_provider(
        "openrouter",
        Arc::new(CaptureProvider::new(openrouter_capture.clone())),
    );
    provider_map.add_provider(
        "openai",
        Arc::new(CaptureProvider::new(explicit_openai_capture.clone())),
    );
    let mapped_provider =
        MultiProvider::new(Arc::new(CaptureProvider::new(default_capture.clone())))
            .with_provider_map(provider_map)
            .with_openai_prefix_mode(MultiProviderOpenAIPrefixMode::ModelId);
    mapped_provider
        .resolve(Some("openrouter/gpt-5"))
        .generate(ModelRequest::default())
        .await
        .expect("explicit provider-map entries should route through the mapped provider");
    mapped_provider
        .resolve(Some("openai/gpt-5"))
        .generate(ModelRequest::default())
        .await
        .expect("explicit openai mappings should override built-in prefix handling");

    let default_seen = default_capture.seen.lock().expect("seen lock");
    assert_eq!(
        default_seen.as_slice(),
        &[
            Some("gpt-5".to_owned()),
            Some("openai/gpt-5".to_owned()),
            Some("unknown/gpt-5".to_owned()),
        ]
    );
    drop(default_seen);

    let openrouter_seen = openrouter_capture.seen.lock().expect("seen lock");
    assert_eq!(openrouter_seen.as_slice(), &[Some("gpt-5".to_owned())]);
    drop(openrouter_seen);

    let explicit_openai_seen = explicit_openai_capture.seen.lock().expect("seen lock");
    assert_eq!(explicit_openai_seen.as_slice(), &[Some("gpt-5".to_owned())]);
}
