use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, GenerateDynamicPromptData, Model, ModelProvider, ModelRequest,
    ModelResponse, OutputItem, Prompt, Result as AgentsResult, Runner, Usage,
};
use serde_json::json;

#[derive(Clone, Default)]
struct PromptEchoModel;

#[async_trait]
impl Model for PromptEchoModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let prompt = request.prompt.unwrap_or_default();
        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text {
                text: format!(
                    "prompt_id={} style={}",
                    prompt.id,
                    prompt
                        .variables
                        .get("poem_style")
                        .cloned()
                        .unwrap_or_else(|| json!("unknown"))
                ),
            }],
            usage: Usage {
                input_tokens: 3,
                output_tokens: 5,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct PromptEchoProvider {
    model: Arc<PromptEchoModel>,
}

impl ModelProvider for PromptEchoProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

fn static_prompt(prompt_id: &str) -> Prompt {
    Prompt {
        id: prompt_id.to_owned(),
        version: Some("1".to_owned()),
        variables: [("poem_style".to_owned(), json!("limerick"))].into(),
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let dynamic = std::env::args().any(|arg| arg == "--dynamic");
    let prompt_id =
        std::env::var("OPENAI_AGENTS_PROMPT_ID").unwrap_or_else(|_| "pmpt_example".to_owned());

    let agent = if dynamic {
        Agent::builder("assistant")
            .dynamic_prompt(move |_data: GenerateDynamicPromptData| {
                let prompt_id = prompt_id.clone();
                async move {
                    Ok(Prompt {
                        id: prompt_id,
                        version: Some("1".to_owned()),
                        variables: [("poem_style".to_owned(), json!("haiku"))].into(),
                    })
                }
            })
            .build()
    } else {
        Agent::builder("assistant")
            .prompt(static_prompt(&prompt_id))
            .build()
    };

    let result = Runner::new()
        .with_model_provider(Arc::new(PromptEchoProvider::default()))
        .run(&agent, "Tell me about recursion.")
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
