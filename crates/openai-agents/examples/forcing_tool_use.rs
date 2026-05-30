use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, Model, ModelProvider, ModelRequest, ModelResponse, ModelSettings,
    OutputItem, Result as AgentsResult, Runner, ToolUseBehavior, Usage, function_tool,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize, JsonSchema)]
struct WeatherArgs {
    city: String,
}

#[derive(Clone, Default)]
struct ToolChoiceModel;

#[async_trait]
impl Model for ToolChoiceModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if request.settings.tool_choice.as_deref() == Some("required") {
            vec![OutputItem::ToolCall {
                call_id: "call-weather".to_owned(),
                tool_name: "get_weather".to_owned(),
                arguments: json!({ "city": "Tokyo" }),
                namespace: None,
            }]
        } else {
            vec![OutputItem::Text {
                text: "tool_choice was not required".to_owned(),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 3,
                output_tokens: 4,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct ToolChoiceProvider {
    model: Arc<ToolChoiceModel>,
}

impl ModelProvider for ToolChoiceProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let get_weather = function_tool(
        "get_weather",
        "Get the current weather information for a city.",
        |_ctx, args: WeatherArgs| async move {
            Ok::<_, AgentsError>(json!({
                "city": args.city,
                "temperature_range": "14-20C",
                "conditions": "Sunny with wind"
            }))
        },
    )?;

    let agent = Agent::builder("Weather agent")
        .instructions("Use tools for weather requests.")
        .model_settings(ModelSettings {
            tool_choice: Some("required".to_owned()),
            ..ModelSettings::default()
        })
        .tool_use_behavior(ToolUseBehavior::StopOnFirstTool)
        .function_tool(get_weather)
        .build();

    let result = Runner::new()
        .with_model_provider(Arc::new(ToolChoiceProvider::default()))
        .run(&agent, "What's the weather in Tokyo?")
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
