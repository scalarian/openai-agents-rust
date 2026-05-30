use std::fs;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, RunInterruptionKind, RunState, Runner, Usage, function_tool,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

const RESULT_PATH: &str = ".cache/agent_patterns/human_in_the_loop/result.json";

#[derive(Debug, Deserialize, JsonSchema)]
struct CityArgs {
    city: String,
}

#[derive(Clone, Default)]
struct WeatherModel;

#[async_trait]
impl Model for WeatherModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let weather = tool_output(&request.input, "get_weather");
        let temperature = tool_output(&request.input, "get_temperature");
        let output = match (weather, temperature) {
            (Some(weather), Some(temperature)) => vec![OutputItem::Text {
                text: format!("{weather} {temperature}"),
            }],
            _ => vec![
                OutputItem::ToolCall {
                    call_id: "call-weather".to_owned(),
                    tool_name: "get_weather".to_owned(),
                    arguments: json!({"city": "Oakland"}),
                    namespace: None,
                },
                OutputItem::ToolCall {
                    call_id: "call-temperature".to_owned(),
                    tool_name: "get_temperature".to_owned(),
                    arguments: json!({"city": "Oakland"}),
                    namespace: None,
                },
            ],
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 12,
                output_tokens: 8,
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
        "Get the weather for a city.",
        |_ctx, args: CityArgs| async move {
            Ok::<_, AgentsError>(format!("The weather in {} is sunny.", args.city))
        },
    )?;
    let get_temperature = function_tool(
        "get_temperature",
        "Get the temperature for a city.",
        |_ctx, args: CityArgs| async move {
            Ok::<_, AgentsError>(format!(
                "The temperature in {} is 20 degrees Celsius.",
                args.city
            ))
        },
    )?
    .with_needs_approval_function(|_ctx, args, _call_id| async move {
        Ok(args
            .get("city")
            .and_then(Value::as_str)
            .is_some_and(|city| city == "Oakland"))
    });

    let agent = Agent::builder("Weather Assistant")
        .instructions("Answer weather and temperature questions using the available tools.")
        .function_tool(get_weather)
        .function_tool(get_temperature)
        .build();
    let runner = Runner::new().with_model_provider(Arc::new(WeatherProvider::default()));
    let mut result = runner
        .run(&agent, "What is the weather and temperature in Oakland?")
        .await?;

    while !result.interruptions.is_empty() {
        println!("run_interrupted=tool_approval_required");
        let state = result
            .durable_state()
            .cloned()
            .ok_or_else(|| AgentsError::message("interrupted run did not include durable state"))?;
        save_state(&state)?;
        println!("state_saved={RESULT_PATH}");

        let mut state = load_state()?;
        for interruption in &result.interruptions {
            println!(
                "approval_request tool={} call_id={} reason={}",
                interruption.tool_name.as_deref().unwrap_or_default(),
                interruption.call_id.as_deref().unwrap_or_default(),
                interruption.reason.as_deref().unwrap_or_default()
            );
            if matches!(interruption.kind, Some(RunInterruptionKind::ToolApproval)) {
                state.approve_for_tool(
                    interruption.call_id.clone().unwrap_or_default(),
                    interruption.tool_name.clone(),
                    Some("approved by reviewer".to_owned()),
                );
                println!(
                    "approved={}",
                    interruption.tool_name.as_deref().unwrap_or_default()
                );
            }
        }

        result = runner.resume_with_agent(&state, &agent).await?;
    }

    println!("final_output={}", result.final_output.unwrap_or_default());
    Ok(())
}

fn save_state(state: &RunState) -> Result<(), AgentsError> {
    let path = Path::new(RESULT_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| AgentsError::message(error.to_string()))?;
    }
    fs::write(path, state.to_json_string()?)
        .map_err(|error| AgentsError::message(error.to_string()))
}

fn load_state() -> Result<RunState, AgentsError> {
    let state_json =
        fs::read_to_string(RESULT_PATH).map_err(|error| AgentsError::message(error.to_string()))?;
    RunState::from_json_str(&state_json)
}

fn tool_output(input: &[InputItem], tool_name: &str) -> Option<String> {
    input.iter().find_map(|item| {
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
            .and_then(|output| output.get("text"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    })
}
