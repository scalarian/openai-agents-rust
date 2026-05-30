use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, ComputerTool, Model, ModelProvider, ModelRequest, ModelResponse,
    ModelSettings, OutputItem, Result as AgentsResult, Runner, Usage,
};
use serde_json::{Value, json};

#[derive(Clone, Default)]
struct ComputerUseModel;

#[async_trait]
impl Model for ComputerUseModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let has_browser_computer = request.tools.iter().any(|tool| {
            tool.name == "computer_use_preview"
                && tool
                    .hosted_tool_options
                    .get("environment")
                    .and_then(|environment| environment.get("type"))
                    .and_then(Value::as_str)
                    == Some("browser")
                && tool
                    .hosted_tool_options
                    .get("display_width")
                    .and_then(Value::as_u64)
                    == Some(1024)
        });
        let text = if has_browser_computer {
            "Browser computer tool configured. Demo result: refreshed the Tokyo Weather page and found partly cloudy, 22C with 37 km/h wind.".to_owned()
        } else {
            "Browser computer tool was not configured.".to_owned()
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text { text }],
            usage: Usage {
                input_tokens: 20,
                output_tokens: 18,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct ComputerUseProvider {
    model: Arc<ComputerUseModel>,
}

impl ModelProvider for ComputerUseProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let computer = ComputerTool::new("computer_use_preview", "Use a browser-like computer")
        .with_hosted_tool_option("environment", json!({"type": "browser"}))
        .with_hosted_tool_option("display_width", 1024.into())
        .with_hosted_tool_option("display_height", 768.into());
    let agent = Agent::builder("Computer Use Assistant")
        .instructions("Use the browser computer tool to inspect web pages.")
        .model_settings(ModelSettings {
            tool_choice: Some("computer_use_preview".to_owned()),
            ..ModelSettings::default()
        })
        .tool(computer)
        .build();

    let result = Runner::new()
        .with_model_provider(Arc::new(ComputerUseProvider::default()))
        .run(
            &agent,
            "Click the Refresh forecast button and summarize the Tokyo weather shown.",
        )
        .await?;
    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
