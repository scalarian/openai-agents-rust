use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, Model, ModelProvider, ModelRequest, ModelResponse, ModelSettings,
    OutputItem, ReasoningSettings, Result as AgentsResult, Runner, Usage,
};

#[derive(Clone, Default)]
struct Gpt5SettingsModel;

#[async_trait]
impl Model for Gpt5SettingsModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let reasoning_effort = request
            .settings
            .reasoning
            .as_ref()
            .and_then(|reasoning| reasoning.effort.as_deref())
            .unwrap_or("unset");
        let verbosity = request.settings.verbosity.as_deref().unwrap_or("unset");
        let model = request.model.clone().unwrap_or_default();

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text {
                text: format!(
                    "model={model}; reasoning_effort={reasoning_effort}; verbosity={verbosity}; recursion calls itself with a smaller version of the same problem."
                ),
            }],
            usage: Usage {
                input_tokens: 12,
                output_tokens: 16,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct Gpt5SettingsProvider {
    model: Arc<Gpt5SettingsModel>,
}

impl ModelProvider for Gpt5SettingsProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("Knowledgable GPT-5 Assistant")
        .instructions("You're a knowledgable assistant. You always provide an interesting answer.")
        .model("gpt-5.5")
        .model_settings(ModelSettings {
            reasoning: Some(ReasoningSettings {
                effort: Some("low".to_owned()),
                summary: None,
            }),
            verbosity: Some("low".to_owned()),
            ..ModelSettings::default()
        })
        .build();

    let runner = Runner::new().with_model_provider(Arc::new(Gpt5SettingsProvider::default()));
    let result = runner
        .run(&agent, "Tell me something about recursion in programming.")
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
