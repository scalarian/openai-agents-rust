use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, FunctionTool, InputItem, MCPServer, MCPTool, MCPUtil, Model, ModelProvider,
    ModelRequest, ModelResponse, OutputItem, Result as AgentsResult, RunContext, RunContextWrapper,
    Runner, ToolFilter, ToolFilterStatic, ToolOutput, Usage,
};
use serde_json::{Value, json};

#[derive(Clone, Default)]
struct SampleFilesystemServer;

#[async_trait]
impl MCPServer for SampleFilesystemServer {
    fn name(&self) -> &str {
        "Filesystem Server"
    }

    fn require_approval(&self) -> Option<&openai_agents::RequireApprovalSetting> {
        static APPROVAL: std::sync::OnceLock<openai_agents::RequireApprovalSetting> =
            std::sync::OnceLock::new();
        Some(APPROVAL.get_or_init(|| {
            openai_agents::RequireApprovalSetting::tool_mapping(BTreeMap::from([(
                "read_file".to_owned(),
                true,
            )]))
        }))
    }

    async fn connect(&self) -> Result<(), AgentsError> {
        Ok(())
    }

    async fn cleanup(&self) -> Result<(), AgentsError> {
        Ok(())
    }

    async fn list_tools(&self) -> Result<Vec<MCPTool>, AgentsError> {
        Ok(vec![
            tool("list_directory", "List files in the allowed directory."),
            tool("read_file", "Read a file from the allowed directory."),
            tool("write_file", "Write a file in the allowed directory."),
        ])
    }

    async fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
        _meta: Option<Value>,
    ) -> Result<ToolOutput, AgentsError> {
        match tool_name {
            "list_directory" => Ok(json!(["books.txt", "favorite_songs.txt"]).into()),
            "read_file" => {
                let path = arguments
                    .get("path")
                    .and_then(Value::as_str)
                    .unwrap_or("books.txt");
                Ok(format!("contents of {path}: 1. To Kill a Mockingbird").into())
            }
            "write_file" => Ok("write completed".into()),
            _ => Err(AgentsError::message(format!("unknown tool `{tool_name}`"))),
        }
    }
}

fn tool(name: &str, description: &str) -> MCPTool {
    MCPTool {
        name: name.to_owned(),
        description: Some(description.to_owned()),
        input_schema: Some(json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" }
            }
        })),
        ..MCPTool::default()
    }
}

#[derive(Clone, Default)]
struct PrefetchedToolsModel;

#[async_trait]
impl Model for PrefetchedToolsModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if let Some(listing) = latest_tool_output(&request.input, "list_directory") {
            vec![OutputItem::Text {
                text: format!("Available files from prefetched MCP tools: {listing}"),
            }]
        } else if request
            .tools
            .iter()
            .any(|tool| tool.name == "list_directory")
        {
            vec![OutputItem::ToolCall {
                call_id: "call-list".to_owned(),
                tool_name: "list_directory".to_owned(),
                arguments: json!({}),
                namespace: None,
            }]
        } else {
            vec![OutputItem::Text {
                text: "No prefetched MCP tools are available.".to_owned(),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 8,
                output_tokens: 8,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct PrefetchedToolsProvider {
    model: Arc<PrefetchedToolsModel>,
}

impl ModelProvider for PrefetchedToolsProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
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
        value.get("output").map(Value::to_string)
    })
}

fn agent_with_tools(name: &str, tools: Vec<FunctionTool>) -> Agent {
    let mut builder = Agent::builder(name).instructions("Use the prefetched MCP tools.");
    for tool in tools {
        builder = builder.function_tool(tool);
    }
    builder.build()
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let server: Arc<dyn MCPServer> = Arc::new(SampleFilesystemServer);
    server.connect().await?;
    let run_context = RunContextWrapper::new(RunContext::default());
    let fetcher = Agent::builder("ToolFetcher")
        .instructions("Prefetch MCP tools.")
        .build();

    println!("=== Fetching all tools ===");
    let all_tools = MCPUtil::get_all_function_tools_connected(
        &[server.clone()],
        None,
        run_context.clone(),
        fetcher.clone(),
        None,
    )
    .await?;
    for tool in &all_tools {
        println!(
            "- {} approval_required={}",
            tool.definition.name, tool.needs_approval
        );
    }

    let prefetched_agent = agent_with_tools("Prefetched MCP Assistant", all_tools);
    let result = Runner::new()
        .with_model_provider(Arc::new(PrefetchedToolsProvider::default()))
        .run(&prefetched_agent, "List files in the allowed directory.")
        .await?;
    println!("{}", result.final_output.unwrap_or_default());

    let filter = ToolFilter::Static(ToolFilterStatic {
        allowed_tool_names: Some(vec!["read_file".to_owned(), "list_directory".to_owned()]),
        blocked_tool_names: Some(vec!["write_file".to_owned()]),
    });
    println!("\n=== After applying tool filter ===");
    let filtered_tools = MCPUtil::get_all_function_tools_connected(
        &[server.clone()],
        Some(&filter),
        run_context,
        fetcher,
        None,
    )
    .await?;
    for tool in &filtered_tools {
        println!("- {}", tool.definition.name);
    }
    server.cleanup().await?;
    Ok(())
}
