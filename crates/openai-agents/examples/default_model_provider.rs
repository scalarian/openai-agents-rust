use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, Usage, run, set_default_agent_runner,
};

#[derive(Clone, Default)]
struct DefaultProviderModel;

#[async_trait]
impl Model for DefaultProviderModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let model_name = request
            .model
            .unwrap_or_else(|| "default-custom-model".to_owned());
        Ok(ModelResponse {
            model: Some(model_name.clone()),
            output: vec![OutputItem::Text {
                text: format!("default_runner_model={model_name}"),
            }],
            usage: Usage {
                input_tokens: 3,
                output_tokens: 3,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct DefaultProvider {
    model: Arc<DefaultProviderModel>,
}

impl ModelProvider for DefaultProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let default_runner = Runner::new().with_model_provider(Arc::new(DefaultProvider::default()));
    set_default_agent_runner(Some(default_runner));

    let agent = Agent::builder("Assistant")
        .instructions("Use the globally configured runner.")
        .model("global-custom-model")
        .build();

    let result = run(&agent, "Which model handled this?").await?;
    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
