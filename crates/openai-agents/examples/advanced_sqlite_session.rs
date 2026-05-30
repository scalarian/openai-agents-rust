use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::extensions::AdvancedSQLiteSession;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, Session, SessionSettings, ToolContext, Usage, function_tool,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize, JsonSchema)]
struct WeatherArgs {
    city: String,
}

#[derive(Clone, Default)]
struct AdvancedSessionModel;

#[async_trait]
impl Model for AdvancedSessionModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let history = request
            .input
            .iter()
            .filter_map(InputItem::as_text)
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();
        let latest = request
            .input
            .iter()
            .rev()
            .find_map(InputItem::as_text)
            .unwrap_or_default()
            .to_lowercase();

        let output = if latest.contains("population") {
            vec![OutputItem::Text {
                text: "San Francisco has about 808,000 residents.".to_owned(),
            }]
        } else if let Some(weather) = tool_output(&request.input, "get_weather") {
            vec![OutputItem::Text {
                text: format!("The remembered city weather is: {weather}"),
            }]
        } else if history.contains("weather") {
            vec![OutputItem::ToolCall {
                call_id: "call-weather".to_owned(),
                tool_name: "get_weather".to_owned(),
                arguments: json!({"city": "San Francisco"}),
                namespace: None,
            }]
        } else if history.contains("golden gate bridge") {
            vec![OutputItem::Text {
                text: "The Golden Gate Bridge is in San Francisco.".to_owned(),
            }]
        } else {
            vec![OutputItem::Text {
                text: "I can answer from the advanced SQLite session context.".to_owned(),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 10,
                output_tokens: 7,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct AdvancedSessionProvider {
    model: Arc<AdvancedSessionModel>,
}

impl ModelProvider for AdvancedSessionProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let get_weather = function_tool(
        "get_weather",
        "Get the weather for a city.",
        |_ctx: ToolContext, args: WeatherArgs| async move {
            let forecast = if args.city.eq_ignore_ascii_case("san francisco") {
                "foggy"
            } else {
                "sunny"
            };
            Ok::<_, AgentsError>(format!("The weather in {} is {forecast}.", args.city))
        },
    )?;
    let agent = Agent::builder("Assistant")
        .instructions("Reply very concisely.")
        .function_tool(get_weather)
        .build();

    let session = AdvancedSQLiteSession::open_with_options(
        "conversation_comprehensive",
        "sqlite::memory:",
        "advanced_agent_sessions",
        "advanced_agent_messages",
        Some(SessionSettings::default()),
    )
    .await?;
    let runner = Runner::new().with_model_provider(Arc::new(AdvancedSessionProvider::default()));

    println!("=== AdvancedSQLiteSession Example ===");
    println!("sessions_table={}", session.sessions_table);
    println!("messages_table={}", session.messages_table);

    let first = runner
        .run_with_session(&agent, "What city is the Golden Gate Bridge in?", &session)
        .await?;
    println!("assistant_1={}", first.final_output.unwrap_or_default());
    println!("usage_1_total={}", total_tokens(first.usage));

    let second = runner
        .run_with_session(&agent, "What's the weather in that city?", &session)
        .await?;
    println!("assistant_2={}", second.final_output.unwrap_or_default());
    println!("usage_2_total={}", total_tokens(second.usage));

    let third = runner
        .run_with_session(&agent, "What's the population of that city?", &session)
        .await?;
    println!("assistant_3={}", third.final_output.unwrap_or_default());
    println!("usage_3_total={}", total_tokens(third.usage));

    let all_items = session.get_items().await?;
    let latest_items = session.get_items_with_limit(Some(3)).await?;
    println!("total_items={}", all_items.len());
    println!("latest_items={}", latest_items.len());
    println!("tool_outputs={}", count_tool_outputs(&all_items));
    Ok(())
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

fn count_tool_outputs(items: &[InputItem]) -> usize {
    items
        .iter()
        .filter(|item| {
            let InputItem::Json { value } = item else {
                return false;
            };
            value.get("type").and_then(Value::as_str) == Some("tool_call_output")
        })
        .count()
}

fn total_tokens(usage: Usage) -> u32 {
    usage.input_tokens + usage.output_tokens
}
