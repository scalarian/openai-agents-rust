use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, Usage,
};

#[derive(Clone, Default)]
struct InstructionEchoModel;

#[async_trait]
impl Model for InstructionEchoModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text {
                text: request.instructions.unwrap_or_default(),
            }],
            usage: Usage {
                input_tokens: 2,
                output_tokens: 4,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct InstructionEchoProvider {
    model: Arc<InstructionEchoModel>,
}

impl ModelProvider for InstructionEchoProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let style = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "haiku".to_owned());

    let agent = Agent::builder("chat_agent")
        .dynamic_instructions(move |_context, _agent| {
            let style = style.clone();
            async move {
                let instructions = match style.as_str() {
                    "pirate" => "Respond as a pirate.",
                    "robot" => "Respond as a robot and say 'beep boop' often.",
                    _ => "Only respond in haikus.",
                };
                Ok(instructions.to_owned())
            }
        })
        .build();

    let result = Runner::new()
        .with_model_provider(Arc::new(InstructionEchoProvider::default()))
        .run(&agent, "Tell me a joke.")
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
