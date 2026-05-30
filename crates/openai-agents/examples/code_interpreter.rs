use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use openai_agents::{
    Agent, AgentsError, CodeInterpreterToolOptions, Model, ModelProvider, ModelRequest,
    ModelResponse, OutputItem, Result as AgentsResult, RunItem, Runner, StreamEvent, Usage,
    code_interpreter_tool_with_options,
};
use serde_json::{Value, json};

#[derive(Clone, Default)]
struct CodeInterpreterModel;

#[async_trait]
impl Model for CodeInterpreterModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let has_auto_container = request.tools.iter().any(|tool| {
            tool.name == "code_interpreter"
                && tool
                    .hosted_tool_options
                    .get("container")
                    .and_then(|container| container.get("type"))
                    .and_then(Value::as_str)
                    == Some("auto")
        });
        let output = if has_auto_container {
            vec![
                OutputItem::Json {
                    value: json!({
                        "type": "code_interpreter_call",
                        "id": "ci_sqrt",
                        "code": "import math\nanswer = math.sqrt(273 * 312821 + 1782)\nprint(answer)",
                        "outputs": [{
                            "type": "logs",
                            "logs": "9241.315653087497\n"
                        }]
                    }),
                },
                OutputItem::Text {
                    text: "The Python code computes sqrt(273 * 312821 + 1782) = 9241.315653087497."
                        .to_owned(),
                },
            ]
        } else {
            vec![OutputItem::Text {
                text: "No code interpreter tool was configured.".to_owned(),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 22,
                output_tokens: 18,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct CodeInterpreterProvider {
    model: Arc<CodeInterpreterModel>,
}

impl ModelProvider for CodeInterpreterProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("Code interpreter")
        .model("gpt-5.5")
        .instructions(
            "Always use the code interpreter tool to solve numeric problems, and show the code you ran when possible.",
        )
        .tool(code_interpreter_tool_with_options(
            CodeInterpreterToolOptions {
                container: Some(json!({"type": "auto"})),
            },
        ))
        .build();

    println!("Solving math problem with the code interpreter...");
    let streamed = Runner::new()
        .with_model_provider(Arc::new(CodeInterpreterProvider::default()))
        .run_streamed(
            &agent,
            "Use the code interpreter tool to calculate the square root of 273 * 312821 + 1782. Show the Python code you ran and then provide the numeric answer.",
        )
        .await?;

    let mut saw_code_interpreter_call = false;
    let mut events = streamed.stream_events();
    while let Some(event) = events.next().await {
        let StreamEvent::RunItemEvent(event) = event else {
            continue;
        };

        if let Some(code) = code_interpreter_code(&event.item) {
            saw_code_interpreter_call = true;
            println!("Code interpreter code:\n```\n{code}\n```\n");
            continue;
        }

        println!("Other event: {}", run_item_kind(&event.item));
    }

    if !saw_code_interpreter_call {
        println!("No code_interpreter_call item was emitted.");
    }
    let result = streamed.wait_for_completion().await?;
    println!("Final output: {}", result.final_output.unwrap_or_default());
    Ok(())
}

fn code_interpreter_code(item: &RunItem) -> Option<&str> {
    let RunItem::MessageOutput {
        content: OutputItem::Json { value },
    } = item
    else {
        return None;
    };
    if value.get("type").and_then(Value::as_str) != Some("code_interpreter_call") {
        return None;
    }
    value.get("code").and_then(Value::as_str)
}

fn run_item_kind(item: &RunItem) -> &'static str {
    match item {
        RunItem::MessageOutput { .. } => "message_output",
        RunItem::ToolCall { .. } => "tool_call",
        RunItem::ToolCallOutput { .. } => "tool_call_output",
        RunItem::CustomToolCall { .. } => "custom_tool_call",
        RunItem::CustomToolCallOutput { .. } => "custom_tool_call_output",
        RunItem::HandoffCall { .. } => "handoff_call",
        RunItem::HandoffOutput { .. } => "handoff_output",
        RunItem::Reasoning { .. } => "reasoning",
    }
}
