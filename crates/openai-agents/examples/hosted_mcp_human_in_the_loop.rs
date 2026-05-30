use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, HostedMCPTool, InputItem, Model, ModelProvider, ModelRequest,
    ModelResponse, ModelSettings, OutputItem, Result as AgentsResult, RunInterruptionKind, Runner,
    Usage, function_tool,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize, JsonSchema)]
struct HostedMcpApprovalArgs {
    server_label: String,
    tool_name: String,
    repo: String,
}

#[derive(Clone, Default)]
struct HostedMcpHitlModel {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl Model for HostedMcpHitlModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let _call = self.calls.fetch_add(1, Ordering::SeqCst);
        let output = if tool_output_text(&request.input, "approve_hosted_mcp_call").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "approve-deepwiki-call".to_owned(),
                tool_name: "approve_hosted_mcp_call".to_owned(),
                arguments: json!({
                    "server_label": "deepwiki",
                    "tool_name": "ask_question",
                    "repo": "openai/codex"
                }),
                namespace: None,
            }]
        } else if hosted_mcp_requires_approval(&request) {
            let approval =
                tool_output_text(&request.input, "approve_hosted_mcp_call").unwrap_or_default();
            vec![OutputItem::Text {
                text: format!(
                    "Manual approval completed for hosted MCP DeepWiki. {approval} The configured hosted MCP call may now answer repository questions."
                ),
            }]
        } else {
            vec![OutputItem::Text {
                text: "Hosted MCP tool was not configured with require_approval=always.".to_owned(),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 24,
                output_tokens: 16,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct HostedMcpHitlProvider {
    model: Arc<HostedMcpHitlModel>,
}

impl ModelProvider for HostedMcpHitlProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let hosted_mcp = HostedMCPTool::new("mcp", "DeepWiki hosted MCP server")
        .with_hosted_tool_option("server_label", "deepwiki".into())
        .with_hosted_tool_option("server_url", "https://mcp.deepwiki.com/mcp".into())
        .with_hosted_tool_option("require_approval", "always".into());
    let approval_gate = function_tool(
        "approve_hosted_mcp_call",
        "Approve a hosted MCP tool call before the hosted server receives it.",
        |_ctx, args: HostedMcpApprovalArgs| async move {
            Ok::<_, AgentsError>(format!(
                "approved server={} tool={} repo={}",
                args.server_label, args.tool_name, args.repo
            ))
        },
    )?
    .with_needs_approval(true);

    let agent = Agent::builder("MCP Assistant")
        .instructions(
            "Use the DeepWiki hosted MCP server for repository questions and pause for human approval before hosted MCP calls.",
        )
        .model_settings(ModelSettings {
            tool_choice: Some("approve_hosted_mcp_call".to_owned()),
            ..ModelSettings::default()
        })
        .tool(hosted_mcp)
        .function_tool(approval_gate)
        .build();
    let runner = Runner::new().with_model_provider(Arc::new(HostedMcpHitlProvider::default()));

    let initial = runner
        .run(
            &agent,
            "Which language is the repository openai/codex written in?",
        )
        .await?;
    let mut run_state = initial
        .durable_state()
        .cloned()
        .ok_or_else(|| AgentsError::message("interrupted hosted MCP run did not include state"))?;
    for interruption in &initial.interruptions {
        println!(
            "approval_request tool={} call_id={}",
            interruption.tool_name.as_deref().unwrap_or_default(),
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
    Ok(())
}

fn hosted_mcp_requires_approval(request: &ModelRequest) -> bool {
    request.tools.iter().any(|tool| {
        tool.name == "mcp"
            && tool
                .hosted_tool_options
                .get("server_label")
                .and_then(Value::as_str)
                == Some("deepwiki")
            && tool
                .hosted_tool_options
                .get("require_approval")
                .and_then(Value::as_str)
                == Some("always")
    })
}

fn tool_output_text(input: &[InputItem], tool_name: &str) -> Option<String> {
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
