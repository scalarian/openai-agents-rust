use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, Usage,
};

#[derive(Clone)]
struct NamedModel {
    name: String,
}

#[async_trait]
impl Model for NamedModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let model_name = request.model.unwrap_or_else(|| self.name.clone());
        Ok(ModelResponse {
            model: Some(model_name.clone()),
            output: vec![OutputItem::Text {
                text: format!("resolved_model={model_name}"),
            }],
            usage: Usage {
                input_tokens: 4,
                output_tokens: 4,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct PerAgentProvider;

impl ModelProvider for PerAgentProvider {
    fn resolve(&self, model: Option<&str>) -> Arc<dyn Model> {
        Arc::new(NamedModel {
            name: model.unwrap_or("default-model").to_owned(),
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("Assistant")
        .instructions("Use the model configured on this agent.")
        .model("custom-agent-model")
        .build();

    let runner = Runner::new().with_model_provider(Arc::new(PerAgentProvider));
    let result = runner.run(&agent, "Which model handled this?").await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
