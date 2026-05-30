use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, RunInterruptionKind, RunItem, RunResultStreaming, Runner, StreamEvent,
    Usage, function_tool,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

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

    let streamed = runner
        .run_streamed(&agent, "What is the weather and temperature in Oakland?")
        .await?;
    drain_stream(&streamed).await;
    let mut result = streamed.wait_for_completion().await?;

    while !result.interruptions.is_empty() {
        println!("human_in_the_loop=approval_required");
        let mut state = result
            .durable_state()
            .cloned()
            .ok_or_else(|| AgentsError::message("interrupted run did not include durable state"))?;
        for interruption in &result.interruptions {
            println!(
                "approval_request tool={} call_id={}",
                interruption.tool_name.as_deref().unwrap_or_default(),
                interruption.call_id.as_deref().unwrap_or_default()
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

        let streamed = runner.resume_streamed_with_agent(&state, &agent).await?;
        drain_stream(&streamed).await;
        result = streamed.wait_for_completion().await?;
    }

    println!("final_output={}", result.final_output.unwrap_or_default());
    println!("done=true");
    Ok(())
}

async fn drain_stream(streamed: &RunResultStreaming) {
    let mut events = streamed.stream_events();
    while let Some(event) = events.next().await {
        match event {
            StreamEvent::Lifecycle(event) if event.name == "tool_approval_required" => {
                let tool_name = event
                    .data
                    .as_ref()
                    .and_then(|data| data.get("tool_name"))
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let call_id = event
                    .data
                    .as_ref()
                    .and_then(|data| data.get("call_id"))
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                println!("stream_approval_required tool={tool_name} call_id={call_id}");
            }
            StreamEvent::RunItemEvent(event) => match event.item {
                RunItem::ToolCall { tool_name, .. } => {
                    println!("stream_tool_call={tool_name}");
                }
                RunItem::ToolCallOutput {
                    tool_name, output, ..
                } => {
                    println!("stream_tool_output={} {}", tool_name, output_text(&output));
                }
                RunItem::MessageOutput { content } => {
                    println!("stream_message={}", output_text(&content));
                }
                RunItem::HandoffCall { .. }
                | RunItem::CustomToolCall { .. }
                | RunItem::CustomToolCallOutput { .. }
                | RunItem::HandoffOutput { .. }
                | RunItem::Reasoning { .. } => {}
            },
            StreamEvent::AgentUpdated(_)
            | StreamEvent::RawResponseEvent(_)
            | StreamEvent::Lifecycle(_) => {}
        }
    }
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

fn output_text(output: &OutputItem) -> String {
    match output {
        OutputItem::Text { text } => text.clone(),
        OutputItem::Json { value } => value.to_string(),
        OutputItem::Refusal { refusal } => refusal.clone(),
        OutputItem::ToolCall { .. }
        | OutputItem::CustomToolCall { .. }
        | OutputItem::Handoff { .. }
        | OutputItem::Reasoning { .. } => String::new(),
    }
}
