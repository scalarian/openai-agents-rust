use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, Model, ModelProvider, ModelRequest, ModelResponse,
    ModelRetryBackoffSettings, ModelRetrySettings, ModelSettings, OutputItem,
    Result as AgentsResult, RunConfig, Runner, Usage, retry_policies,
};

#[derive(Clone, Default)]
struct FlakyModel {
    attempts: Arc<AtomicUsize>,
}

#[async_trait]
impl Model for FlakyModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let attempt = self.attempts.fetch_add(1, Ordering::SeqCst) + 1;
        if attempt == 1 {
            return Err(AgentsError::message("network error: temporary disconnect"));
        }

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text {
                text: format!("succeeded_on_attempt={attempt}"),
            }],
            usage: Usage {
                input_tokens: 3,
                output_tokens: 3,
            },
            response_id: Some("resp-retry".to_owned()),
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct FlakyProvider {
    model: Arc<FlakyModel>,
}

impl ModelProvider for FlakyProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let provider = Arc::new(FlakyProvider::default());
    let retry = ModelRetrySettings {
        max_retries: Some(2),
        backoff: Some(ModelRetryBackoffSettings {
            initial_delay: Some(0.0),
            max_delay: Some(0.0),
            multiplier: Some(1.0),
            jitter: Some(false),
        }),
        policy: Some(retry_policies::any(vec![
            retry_policies::network_error(),
            retry_policies::http_status([408, 409, 429, 500, 502, 503, 504]),
        ])),
    };
    let run_config = RunConfig {
        model_settings: Some(ModelSettings {
            retry: Some(retry),
            ..ModelSettings::default()
        }),
        ..RunConfig::default()
    };
    let agent = Agent::builder("Assistant")
        .instructions("Explain retries in one sentence.")
        .build();

    let result = Runner::new()
        .with_model_provider(provider.clone())
        .with_config(run_config)
        .run(&agent, "Explain exponential backoff.")
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    println!(
        "model_attempts={}",
        provider.model.attempts.load(Ordering::SeqCst)
    );
    Ok(())
}
