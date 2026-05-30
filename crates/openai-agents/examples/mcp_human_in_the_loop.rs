use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, MCPServer, MCPTool, MCPToolAnnotations, Model, ModelProvider,
    ModelRequest, ModelResponse, OutputItem, Result as AgentsResult, RunInterruptionKind, Runner,
    ToolOutput, Usage,
};
use serde_json::{Value, json};

#[derive(Default)]
struct RepoMcpState {
    tool_calls: AtomicUsize,
}

struct RepoMcpServer {
    state: Arc<RepoMcpState>,
}

#[async_trait]
impl MCPServer for RepoMcpServer {
    fn name(&self) -> &str {
        "deepwiki"
    }

    async fn connect(&self) -> AgentsResult<()> {
        Ok(())
    }

    async fn cleanup(&self) -> AgentsResult<()> {
        Ok(())
    }

    async fn list_tools(&self) -> AgentsResult<Vec<MCPTool>> {
        Ok(vec![MCPTool {
            name: "repo_language".to_owned(),
            description: Some("Look up the primary language for a repository.".to_owned()),
            input_schema: Some(json!({
                "type": "object",
                "properties": {
                    "repo": {"type": "string"}
                },
                "required": ["repo"]
            })),
            title: None,
            annotations: Some(MCPToolAnnotations {
                title: Some("Repository language".to_owned()),
            }),
            meta: None,
            namespace: Some("mcp".to_owned()),
            requires_approval: true,
        }])
    }

    async fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
        _meta: Option<Value>,
    ) -> AgentsResult<ToolOutput> {
        self.state.tool_calls.fetch_add(1, Ordering::SeqCst);
        let repo = arguments
            .get("repo")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        Ok(ToolOutput::from(format!(
            "{tool_name}: {repo} is primarily written in Rust"
        )))
    }
}

#[derive(Clone, Default)]
struct McpHitlModel {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl Model for McpHitlModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        let output = if call == 0 {
            vec![OutputItem::ToolCall {
                call_id: "call-repo-language".to_owned(),
                tool_name: "repo_language".to_owned(),
                arguments: json!({"repo": "openai/codex"}),
                namespace: Some("mcp".to_owned()),
            }]
        } else {
            let tool_output = tool_output_text(&request.input).unwrap_or_default();
            vec![OutputItem::Text {
                text: format!("Approved MCP result: {tool_output}"),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 24,
                output_tokens: 12,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone)]
struct McpHitlProvider {
    model: Arc<McpHitlModel>,
}

impl ModelProvider for McpHitlProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let state = Arc::new(RepoMcpState::default());
    let server = Arc::new(RepoMcpServer {
        state: state.clone(),
    }) as Arc<dyn MCPServer>;
    let agent = Agent::builder("MCP Assistant")
        .instructions(
            "Use the MCP repository lookup tool. Approval is required before the MCP call can run.",
        )
        .mcp_server(server)
        .build();
    let runner = Runner::new().with_model_provider(Arc::new(McpHitlProvider {
        model: Arc::new(McpHitlModel::default()),
    }));

    let initial = runner
        .run(
            &agent,
            "Which language is the repository openai/codex written in?",
        )
        .await?;
    println!(
        "initial_tool_calls={}",
        state.tool_calls.load(Ordering::SeqCst)
    );

    let mut run_state = initial
        .durable_state()
        .cloned()
        .ok_or_else(|| AgentsError::message("interrupted MCP run did not include durable state"))?;
    for interruption in &initial.interruptions {
        println!(
            "approval_request tool={} namespace={} call_id={}",
            interruption.tool_name.as_deref().unwrap_or_default(),
            interruption.namespace.as_deref().unwrap_or_default(),
            interruption.call_id.as_deref().unwrap_or_default()
        );
        if matches!(interruption.kind, Some(RunInterruptionKind::ToolApproval)) {
            run_state.approve_for_tool(
                interruption.call_id.clone().unwrap_or_default(),
                interruption.tool_name.clone(),
                Some("approved by CLI operator".to_owned()),
            );
        }
    }

    let resumed = runner.resume_with_agent(&run_state, &agent).await?;
    println!("final_output={}", resumed.final_output.unwrap_or_default());
    println!(
        "final_tool_calls={}",
        state.tool_calls.load(Ordering::SeqCst)
    );
    Ok(())
}

fn tool_output_text(input: &[InputItem]) -> Option<String> {
    input.iter().find_map(|item| {
        let InputItem::Json { value } = item else {
            return None;
        };
        if value.get("type").and_then(Value::as_str) != Some("tool_call_output") {
            return None;
        }
        value
            .get("output")
            .and_then(|output| output.get("text"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    })
}
