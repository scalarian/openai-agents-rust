use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, MCPServer, MCPTool, Model, ModelProvider, ModelRequest,
    ModelResponse, OutputItem, Result as AgentsResult, Runner, ToolFilter, ToolFilterStatic,
    ToolOutput, Usage,
};
use serde_json::{Value, json};

#[derive(Debug)]
struct FilteredFilesystemServer;

#[async_trait]
impl MCPServer for FilteredFilesystemServer {
    fn name(&self) -> &str {
        "filtered-filesystem"
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
                description: Some("List files.".to_owned()),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false,
                })),
                ..MCPTool::default()
            },
            MCPTool {
                name: "write_file".to_owned(),
                description: Some("Write a file.".to_owned()),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "text": { "type": "string" }
                    },
                    "required": ["path", "text"],
                    "additionalProperties": false,
                })),
                ..MCPTool::default()
            },
        ])
    }

    async fn call_tool(
        &self,
        tool_name: &str,
        _arguments: Value,
        _meta: Option<Value>,
    ) -> AgentsResult<ToolOutput> {
        match tool_name {
            "list_directory" => Ok(ToolOutput::from("favorite_books.txt\nfavorite_songs.txt")),
            "write_file" => Ok(ToolOutput::from("write should have been blocked")),
            other => Err(AgentsError::message(format!(
                "unknown filtered filesystem tool `{other}`"
            ))),
        }
    }
}

#[derive(Clone, Default)]
struct ToolFilterModel;

#[async_trait]
impl Model for ToolFilterModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let has_list = request
            .tools
            .iter()
            .any(|tool| tool.name == "list_directory");
        let has_write = request.tools.iter().any(|tool| tool.name == "write_file");
        let output = if let Some(listing) = tool_output(&request.input, "list_directory") {
            vec![OutputItem::Text {
                text: format!("Available files: {listing}. write_file was blocked."),
            }]
        } else if has_list && !has_write {
            vec![OutputItem::ToolCall {
                call_id: "call-list".to_owned(),
                tool_name: "list_directory".to_owned(),
                arguments: json!({}),
                namespace: None,
            }]
        } else {
            vec![OutputItem::Text {
                text: format!(
                    "unexpected_tool_filter_state has_list={has_list} has_write={has_write}"
                ),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 10,
                output_tokens: 8,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct ToolFilterProvider {
    model: Arc<ToolFilterModel>,
}

impl ModelProvider for ToolFilterProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("MCP Assistant")
        .instructions("Use only available MCP filesystem tools.")
        .mcp_server(Arc::new(FilteredFilesystemServer))
        .mcp_tool_filter(ToolFilter::Static(ToolFilterStatic {
            allowed_tool_names: Some(vec!["list_directory".to_owned()]),
            blocked_tool_names: Some(vec!["write_file".to_owned()]),
        }))
        .build();

    let result = Runner::new()
        .with_model_provider(Arc::new(ToolFilterProvider::default()))
        .run(
            &agent,
            "List files, then explain whether writing is available.",
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
