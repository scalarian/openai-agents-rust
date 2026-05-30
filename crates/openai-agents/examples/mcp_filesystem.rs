use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, MCPServer, MCPTool, Model, ModelProvider, ModelRequest,
    ModelResponse, OutputItem, Result as AgentsResult, Runner, ToolOutput, Usage,
};
use serde_json::{Value, json};

#[derive(Debug)]
struct SampleFilesystemServer;

#[async_trait]
impl MCPServer for SampleFilesystemServer {
    fn name(&self) -> &str {
        "sample-filesystem"
    }

    async fn connect(&self) -> AgentsResult<()> {
        Ok(())
    }

    async fn cleanup(&self) -> AgentsResult<()> {
        Ok(())
    }

    async fn list_tools(&self) -> AgentsResult<Vec<MCPTool>> {
        Ok(vec![
            MCPTool {
                name: "list_directory".to_owned(),
                description: Some("List files in the sample directory.".to_owned()),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false,
                })),
                ..MCPTool::default()
            },
            MCPTool {
                name: "read_file".to_owned(),
                description: Some("Read a sample file by name.".to_owned()),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" }
                    },
                    "required": ["path"],
                    "additionalProperties": false,
                })),
                ..MCPTool::default()
            },
        ])
    }

    async fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
        _meta: Option<Value>,
    ) -> AgentsResult<ToolOutput> {
        match tool_name {
            "list_directory" => Ok(ToolOutput::from(
                "favorite_books.txt\nfavorite_songs.txt\nfavorite_cities.txt",
            )),
            "read_file" => {
                let path = arguments
                    .get("path")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let text = match path {
                    "favorite_books.txt" => "1. A Wizard of Earthsea\n2. The Left Hand of Darkness",
                    "favorite_songs.txt" => "1. This Must Be the Place\n2. Age of Consent",
                    "favorite_cities.txt" => "1. San Francisco\n2. Kyoto",
                    _ => "File not found.",
                };
                Ok(ToolOutput::from(text))
            }
            other => Err(AgentsError::message(format!(
                "unknown sample filesystem tool `{other}`"
            ))),
        }
    }
}

#[derive(Clone, Default)]
struct McpFilesystemModel;

#[async_trait]
impl Model for McpFilesystemModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if let Some(file_text) = tool_output(&request.input, "read_file") {
            let first_line = file_text
                .lines()
                .next()
                .unwrap_or("No favorite book found.");
            vec![OutputItem::Text {
                text: format!(
                    "Your #1 favorite book is {}.",
                    first_line.trim_start_matches("1. ")
                ),
            }]
        } else if request.tools.iter().any(|tool| tool.name == "read_file") {
            vec![OutputItem::ToolCall {
                call_id: "call-read-books".to_owned(),
                tool_name: "read_file".to_owned(),
                arguments: json!({ "path": "favorite_books.txt" }),
                namespace: None,
            }]
        } else {
            vec![OutputItem::Text {
                text: "No MCP filesystem tool was available.".to_owned(),
            }]
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
struct McpFilesystemProvider {
    model: Arc<McpFilesystemModel>,
}

impl ModelProvider for McpFilesystemProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("Assistant")
        .instructions("Use the MCP filesystem tools to answer questions based on sample files.")
        .mcp_server(Arc::new(SampleFilesystemServer))
        .build();

    let result = Runner::new()
        .with_model_provider(Arc::new(McpFilesystemProvider::default()))
        .run(
            &agent,
            "Read favorite_books.txt and tell me my #1 favorite book.",
        )
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
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
