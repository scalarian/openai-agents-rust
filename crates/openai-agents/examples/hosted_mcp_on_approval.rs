use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, HostedMCPTool, MCPToolApprovalFunctionResult, MCPToolApprovalRequest,
    Model, ModelProvider, ModelRequest, ModelResponse, ModelSettings, OutputItem,
    Result as AgentsResult, Runner, Usage,
};
use serde_json::{Value, json};

#[derive(Clone, Default)]
struct HostedMcpApprovalModel;

#[async_trait]
impl Model for HostedMcpApprovalModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let has_approval_gated_deepwiki = request.tools.iter().any(|tool| {
            tool.name == "mcp"
                && tool
                    .hosted_tool_options
                    .get("server_label")
                    .and_then(Value::as_str)
                    == Some("deepwiki")
                && tool
                    .hosted_tool_options
                    .get("server_url")
                    .and_then(Value::as_str)
                    == Some("https://mcp.deepwiki.com/mcp")
                && tool
                    .hosted_tool_options
                    .get("require_approval")
                    .and_then(Value::as_str)
                    == Some("always")
        });
        let text = if has_approval_gated_deepwiki {
            "Hosted MCP DeepWiki tool configured with require_approval=always; approval callback returned approved=true.".to_owned()
        } else {
            "Hosted MCP approval configuration was not found.".to_owned()
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text { text }],
            usage: Usage {
                input_tokens: 16,
                output_tokens: 14,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct HostedMcpApprovalProvider {
    model: Arc<HostedMcpApprovalModel>,
}

impl ModelProvider for HostedMcpApprovalProvider {
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

    let approval = prompt_approval(MCPToolApprovalRequest {
        call_id: "mcp-call-1".to_owned(),
        tool_name: "ask_question".to_owned(),
        arguments: json!({"repo": "openai/codex", "question": "primary language"}),
        namespace: Some("mcp".to_owned()),
    });
    println!(
        "approval_callback approved={} reason={}",
        approval.approved,
        approval.reason.unwrap_or_default()
    );

    let agent = Agent::builder("MCP Assistant")
        .instructions(
            "Use the DeepWiki hosted MCP server for repository questions. Hosted MCP tool calls require approval.",
        )
        .model_settings(ModelSettings {
            tool_choice: Some("mcp".to_owned()),
            ..ModelSettings::default()
        })
        .tool(hosted_mcp)
        .build();
    let result = Runner::new()
        .with_model_provider(Arc::new(HostedMcpApprovalProvider::default()))
        .run(
            &agent,
            "Which language is the repository openai/codex written in?",
        )
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}

fn prompt_approval(request: MCPToolApprovalRequest) -> MCPToolApprovalFunctionResult {
    let params = request.arguments;
    println!(
        "approval_request tool={} namespace={} params={}",
        request.tool_name,
        request.namespace.unwrap_or_default(),
        params
    );
    MCPToolApprovalFunctionResult {
        approved: true,
        reason: Some("approved by local demo callback".to_owned()),
    }
}
