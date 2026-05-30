use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, Usage, function_tool,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize, JsonSchema)]
struct WeatherArgs {
    city: String,
}

#[derive(Clone, Default)]
struct WeatherModel;

#[async_trait]
impl Model for WeatherModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if let Some(weather) = latest_tool_output(&request.input, "get_weather") {
            let city = weather
                .get("city")
                .and_then(Value::as_str)
                .unwrap_or("the requested city");
            vec![OutputItem::Text {
                text: format!("The weather in {city} is sunny."),
            }]
        } else {
            vec![OutputItem::ToolCall {
                call_id: "call-weather".to_owned(),
                tool_name: "get_weather".to_owned(),
                arguments: json!({ "city": "Tokyo" }),
                namespace: None,
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 8,
                output_tokens: 6,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct WeatherProvider {
    model: Arc<WeatherModel>,
}

impl ModelProvider for WeatherProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let get_weather = function_tool(
        "get_weather",
        "Get the current weather information for a specified city.",
        |_ctx, args: WeatherArgs| async move {
            println!("[debug] get_weather called");
            Ok::<_, AgentsError>(json!({
                "city": args.city,
                "temperature_range": "14-20C",
                "conditions": "Sunny with wind."
            }))
        },
    )?;

    let agent = Agent::builder("Hello world")
        .instructions("You are a helpful agent.")
        .function_tool(get_weather)
        .build();

    let result = Runner::new()
        .with_model_provider(Arc::new(WeatherProvider::default()))
        .run(&agent, "What's the weather in Tokyo?")
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}

fn latest_tool_output(input: &[InputItem], tool_name: &str) -> Option<Value> {
    input.iter().rev().find_map(|item| {
        let InputItem::Json { value } = item else {
            return None;
        };
        if value.get("type").and_then(Value::as_str) != Some("tool_call_output")
            || value.get("tool_name").and_then(Value::as_str) != Some(tool_name)
        {
            return None;
        }
        value
            .get("output")
            .and_then(|output| match output.get("type").and_then(Value::as_str) {
                Some("json") => output.get("value").cloned(),
                _ => None,
            })
    })
}
