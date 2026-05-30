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
struct WriteFileArgs {
    filename: String,
    content: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateConfigArgs {
    project_name: String,
    version: String,
    dependencies: Option<Vec<String>>,
}

#[derive(Clone, Default)]
struct CodeGeneratorModel;

#[async_trait]
impl Model for CodeGeneratorModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let tool_outputs = request
            .input
            .iter()
            .filter(|item| matches!(item, InputItem::Json { value } if value.get("type").and_then(Value::as_str) == Some("tool_call_output")))
            .count();

        let output = if tool_outputs >= 2 {
            vec![OutputItem::Text {
                text: "Created a FastAPI project scaffold.".to_owned(),
            }]
        } else {
            vec![
                OutputItem::ToolCall {
                    call_id: "call-write-file".to_owned(),
                    tool_name: "write_file".to_owned(),
                    arguments: json!({
                        "filename": "main.py",
                        "content": "from fastapi import FastAPI\napp = FastAPI()"
                    }),
                    namespace: None,
                },
                OutputItem::ToolCall {
                    call_id: "call-create-config".to_owned(),
                    tool_name: "create_config".to_owned(),
                    arguments: json!({
                        "project_name": "my-app",
                        "version": "1.0.0",
                        "dependencies": ["fastapi", "uvicorn"]
                    }),
                    namespace: None,
                },
            ]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 12,
                output_tokens: 10,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct CodeGeneratorProvider {
    model: Arc<CodeGeneratorModel>,
}

impl ModelProvider for CodeGeneratorProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let write_file = function_tool(
        "write_file",
        "Write content to a file.",
        |_ctx, args: WriteFileArgs| async move {
            Ok::<_, AgentsError>(format!(
                "File {} written successfully ({} bytes)",
                args.filename,
                args.content.len()
            ))
        },
    )?;
    let create_config = function_tool(
        "create_config",
        "Generate a project configuration file.",
        |_ctx, args: CreateConfigArgs| async move {
            let deps = args.dependencies.unwrap_or_default().join(", ");
            Ok::<_, AgentsError>(format!(
                "Config for {} v{} created with {deps}",
                args.project_name, args.version
            ))
        },
    )?;

    let agent = Agent::builder("CodeGenerator")
        .instructions("Use the provided tools to create files and configurations.")
        .function_tool(write_file)
        .function_tool(create_config)
        .build();

    println!("Function call arguments streaming demo");
    let runner = Runner::new().with_model_provider(Arc::new(CodeGeneratorProvider::default()));
    let streamed = runner
        .run_streamed(
            &agent,
            "Create a Python web project called 'my-app' with FastAPI.",
        )
        .await?;

    let mut events = streamed.stream_events();
    while let Some(event) = events.next().await {
        if let StreamEvent::RunItemEvent(event) = event {
            match event.item {
                RunItem::ToolCall {
                    tool_name,
                    arguments,
                    call_id,
                    ..
                } => {
                    println!(
                        "function_call={} id={} arguments={}",
                        tool_name,
                        call_id.unwrap_or_default(),
                        arguments
                    );
                }
                RunItem::ToolCallOutput {
                    tool_name, output, ..
                } => {
                    println!("tool_output={} {}", tool_name, output_text(&output));
                }
                RunItem::MessageOutput { content } => {
                    println!("message={}", output_text(&content));
                }
                RunItem::HandoffCall { .. }
                | RunItem::CustomToolCall { .. }
                | RunItem::CustomToolCallOutput { .. }
                | RunItem::HandoffOutput { .. }
                | RunItem::Reasoning { .. } => {}
            }
        }
    }

    let result = streamed.wait_for_completion().await?;
    println!("result={}", result.final_output.unwrap_or_default());
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
