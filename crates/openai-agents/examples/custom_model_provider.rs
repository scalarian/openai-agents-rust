use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, RunOptions, Runner, Usage, function_tool,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize, JsonSchema)]
struct WeatherArgs {
    city: String,
}

#[derive(Clone)]
struct CustomModel {
    provider_name: &'static str,
}

#[async_trait]
impl Model for CustomModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if let Some(weather) = tool_output_text(&request.input) {
            vec![OutputItem::Text {
                text: format!(
                    "{} haiku:\nTokyo sunshine\n{weather}\nUmbrellas can rest",
                    self.provider_name
                ),
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
                input_tokens: 6,
                output_tokens: 10,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone)]
struct CustomModelProvider {
    model: Arc<CustomModel>,
}

impl CustomModelProvider {
    fn new(provider_name: &'static str) -> Self {
        Self {
            model: Arc::new(CustomModel { provider_name }),
        }
    }
}

impl ModelProvider for CustomModelProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

fn tool_output_text(input: &[InputItem]) -> Option<String> {
    input.iter().find_map(|item| {
        let InputItem::Json { value } = item else {
            return None;
        };
        let output = value.get("output")?;
        match output.get("type").and_then(Value::as_str) {
            Some("text") => output
                .get("text")
                .and_then(Value::as_str)
                .map(str::to_owned),
            Some("json") => output.get("value").map(Value::to_string),
            _ => None,
        }
    })
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let get_weather = function_tool(
        "get_weather",
        "Get the weather for a city.",
        |_ctx, args: WeatherArgs| async move {
            println!("[debug] getting weather for {}", args.city);
            Ok::<_, AgentsError>(format!("The weather in {} is sunny.", args.city))
        },
    )?;

    let agent = Agent::builder("Assistant")
        .instructions("You only respond in haikus.")
        .function_tool(get_weather)
        .build();

    let runner = Runner::new();
    let result = runner
        .run_with_options(
            &agent,
            vec![InputItem::Text {
                text: "What's the weather in Tokyo?".to_owned(),
            }],
            RunOptions {
                model_provider: Some(Arc::new(CustomModelProvider::new("custom-provider"))),
                ..RunOptions::default()
            },
        )
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
