use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, RunItem, Runner, StreamEvent, Usage, function_tool,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize, JsonSchema)]
struct NoArgs {}

#[derive(Clone, Default)]
struct JokeModel;

#[async_trait]
impl Model for JokeModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if let Some(count) = tool_result_count(&request.input) {
            vec![OutputItem::Text {
                text: format_jokes(count),
            }]
        } else {
            vec![OutputItem::ToolCall {
                call_id: "call-how-many-jokes".to_owned(),
                tool_name: "how_many_jokes".to_owned(),
                arguments: json!({}),
                namespace: None,
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 5,
                output_tokens: 9,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct JokeProvider {
    model: Arc<JokeModel>,
}

impl ModelProvider for JokeProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

fn tool_result_count(input: &[InputItem]) -> Option<u64> {
    input.iter().find_map(|item| {
        let InputItem::Json { value } = item else {
            return None;
        };
        let output = value.get("output")?;
        match output.get("type").and_then(Value::as_str) {
            Some("json") => output.get("value").and_then(Value::as_u64),
            Some("text") => output
                .get("text")
                .and_then(Value::as_str)
                .and_then(|text| text.parse::<u64>().ok()),
            _ => None,
        }
    })
}

fn format_jokes(count: u64) -> String {
    let mut lines = vec![format!("Here are {count} short jokes:")];
    for index in 1..=count {
        lines.push(format!(
            "{index}. Why did the test pass? It had good assertions."
        ));
    }
    lines.join("\n")
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let how_many_jokes = function_tool(
        "how_many_jokes",
        "Return the number of jokes to tell.",
        |_ctx, _args: NoArgs| async move { Ok::<_, AgentsError>(json!(3)) },
    )?;

    let agent = Agent::builder("Joker")
        .instructions("First call the `how_many_jokes` tool, then tell that many jokes.")
        .function_tool(how_many_jokes)
        .build();

    let runner = Runner::new().with_model_provider(Arc::new(JokeProvider::default()));
    let streamed = runner.run_streamed(&agent, "Hello").await?;

    println!("=== Run starting ===");
    let mut events = streamed.stream_events();
    while let Some(event) = events.next().await {
        match event {
            StreamEvent::AgentUpdated(update) => {
                println!("Agent updated: {}", update.new_agent.name);
            }
            StreamEvent::RunItemEvent(event) => match &event.item {
                RunItem::ToolCall { tool_name, .. } => {
                    println!("-- Tool was called: {tool_name}");
                }
                RunItem::ToolCallOutput { output, .. } => {
                    println!("-- Tool output: {}", output_text(output));
                }
                RunItem::MessageOutput { content } => {
                    println!("-- Message output:\n{}", output_text(content));
                }
                RunItem::HandoffCall { .. }
                | RunItem::CustomToolCall { .. }
                | RunItem::CustomToolCallOutput { .. }
                | RunItem::HandoffOutput { .. }
                | RunItem::Reasoning { .. } => {}
            },
            StreamEvent::RawResponseEvent(_) | StreamEvent::Lifecycle(_) => {}
        }
    }
    println!("=== Run complete ===");

    Ok(())
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
