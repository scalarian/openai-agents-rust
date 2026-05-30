use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse,
    OpenAIConversationsSession, OutputItem, Result as AgentsResult, RunInterruptionKind, Runner,
    Session, ToolContext, Usage, function_tool,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize, JsonSchema)]
struct WeatherArgs {
    location: String,
}

#[derive(Clone, Default)]
struct OpenAISessionHitlModel;

#[async_trait]
impl Model for OpenAISessionHitlModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if let Some(weather) = latest_tool_output(&request.input, "get_weather") {
            vec![OutputItem::Text {
                text: format!("The weather in Oakland is {weather}."),
            }]
        } else {
            vec![OutputItem::ToolCall {
                call_id: "call-weather".to_owned(),
                tool_name: "get_weather".to_owned(),
                arguments: json!({ "location": "Oakland" }),
                namespace: None,
            }]
        };
        let suffix = request.previous_response_id.as_deref().unwrap_or("start");

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 14,
                output_tokens: 10,
            },
            response_id: Some(format!("resp_{suffix}")),
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct OpenAISessionHitlProvider {
    model: Arc<OpenAISessionHitlModel>,
}

impl ModelProvider for OpenAISessionHitlProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let get_weather = function_tool(
        "get_weather",
        "Get weather for a location.",
        |_ctx: ToolContext, args: WeatherArgs| async move {
            let weather = if args.location.eq_ignore_ascii_case("oakland") {
                "sunny, 72 F"
            } else {
                "weather data unavailable"
            };
            Ok::<_, AgentsError>(weather.to_owned())
        },
    )?
    .with_needs_approval(true);

    let agent = Agent::builder("HITL Assistant")
        .instructions("Always use available tools when appropriate. Keep responses concise.")
        .function_tool(get_weather)
        .build();
    let session = OpenAIConversationsSession::new("conversation_hitl_123");
    let runner = Runner::new().with_model_provider(Arc::new(OpenAISessionHitlProvider::default()));

    println!("=== OpenAI Session + HITL Example ===");
    println!("conversation_id={}", session.conversation_id().await);

    let mut result = runner
        .run_with_session(&agent, "What's the weather in Oakland?", &session)
        .await?;
    while !result.interruptions.is_empty() {
        let mut state = result
            .durable_state()
            .cloned()
            .ok_or_else(|| AgentsError::message("interrupted run did not include state"))?;
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
                    Some("approved by operator".to_owned()),
                );
                println!("Approved tool call.");
            }
        }
        result = runner
            .resume_with_agent_and_session(&state, &agent, &session)
            .await?;
    }

    println!("Assistant: {}", result.final_output.unwrap_or_default());
    println!(
        "last_response_id={}",
        session.last_response_id().await.unwrap_or_default()
    );
    println!("session_items={}", session.get_items().await?.len());
    Ok(())
}

fn latest_tool_output(input: &[InputItem], tool_name: &str) -> Option<String> {
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
            .and_then(|output| output.get("text"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    })
}
