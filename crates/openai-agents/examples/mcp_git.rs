use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, MCPServer, MCPTool, Model, ModelProvider, ModelRequest,
    ModelResponse, OutputItem, Result as AgentsResult, Runner, ToolOutput, Usage,
};
use serde_json::{Value, json};

#[derive(Clone, Default)]
struct GitServer;

#[async_trait]
impl MCPServer for GitServer {
    fn name(&self) -> &str {
        "Git MCP Server"
    }

    async fn connect(&self) -> Result<(), AgentsError> {
        Ok(())
    }

    async fn cleanup(&self) -> Result<(), AgentsError> {
        Ok(())
    }

    async fn list_tools(&self) -> Result<Vec<MCPTool>, AgentsError> {
        Ok(vec![
            MCPTool {
                name: "git_log".to_owned(),
                description: Some("Return recent commits for a repository.".to_owned()),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "repo_path": { "type": "string" }
                    },
                    "required": ["repo_path"]
                })),
                ..MCPTool::default()
            },
            MCPTool {
                name: "git_status".to_owned(),
                description: Some("Return the current repository status.".to_owned()),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "repo_path": { "type": "string" }
                    },
                    "required": ["repo_path"]
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
    ) -> Result<ToolOutput, AgentsError> {
        let repo_path = arguments
            .get("repo_path")
            .and_then(Value::as_str)
            .unwrap_or(".");
        match tool_name {
            "git_log" => Ok(format!(
                "Recent commits for {repo_path}: Alice 3 commits, Bob 2 commits, Chandra 1 commit."
            )
            .into()),
            "git_status" => Ok(format!("{repo_path}: working tree clean").into()),
            _ => Err(AgentsError::message(format!("unknown tool `{tool_name}`"))),
        }
    }
}

#[derive(Clone, Default)]
struct GitModel;

#[async_trait]
impl Model for GitModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if let Some(log) = latest_tool_output(&request.input, "git_log") {
            vec![OutputItem::Text {
                text: format!("Most frequent contributor: Alice. Source: {log}"),
            }]
        } else if request.tools.iter().any(|tool| tool.name == "git_log") {
            vec![OutputItem::ToolCall {
                call_id: "call-git-log".to_owned(),
                tool_name: "git_log".to_owned(),
                arguments: json!({ "repo_path": "." }),
                namespace: None,
            }]
        } else {
            vec![OutputItem::Text {
                text: "No git MCP tools are available.".to_owned(),
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
struct GitProvider {
    model: Arc<GitModel>,
}

impl ModelProvider for GitProvider {
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
        value
            .get("output")
            .and_then(|output| output.get("text"))
            .and_then(Value::as_str)
            .map(str::to_owned)
    })
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let directory_path = std::env::args().nth(1).unwrap_or_else(|| ".".to_owned());
    let git_server: Arc<dyn MCPServer> = Arc::new(GitServer);
    let agent = Agent::builder("Assistant")
        .instructions(format!(
            "Answer questions about the git repository at {directory_path}; use that for repo_path."
        ))
        .mcp_server(git_server)
        .build();
    let runner = Runner::new().with_model_provider(Arc::new(GitProvider::default()));

    let result = runner
        .run(&agent, "Who's the most frequent contributor?")
        .await?;
    println!("{}", result.final_output.unwrap_or_default());

    Ok(())
}
