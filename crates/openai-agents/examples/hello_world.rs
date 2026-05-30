use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, Usage,
};

#[derive(Clone, Default)]
struct HaikuModel;

#[async_trait]
impl Model for HaikuModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text {
                text: "Function calls itself,\nLooping through smaller questions,\nBase case ends the path."
                    .to_owned(),
            }],
            usage: Usage {
                input_tokens: 12,
                output_tokens: 17,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct HaikuProvider {
    model: Arc<HaikuModel>,
}

impl ModelProvider for HaikuProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("Assistant")
        .instructions("You only respond in haikus.")
        .build();

    let result = Runner::new()
        .with_model_provider(Arc::new(HaikuProvider::default()))
        .run(&agent, "Tell me about recursion in programming.")
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
