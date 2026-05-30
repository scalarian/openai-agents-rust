use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, HostedMCPTool, Model, ModelProvider, ModelRequest, ModelResponse,
    ModelSettings, OutputItem, Result as AgentsResult, Runner, Usage,
};
use serde_json::Value;

#[derive(Clone, Default)]
struct HostedMcpModel;

#[async_trait]
impl Model for HostedMcpModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let has_deepwiki = request.tools.iter().any(|tool| {
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
                    == Some("never")
        });
        let text = if has_deepwiki {
            "The DeepWiki hosted MCP tool is configured for repository inspection.".to_owned()
        } else {
            "No hosted MCP tool was configured.".to_owned()
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text { text }],
            usage: Usage {
                input_tokens: 9,
                output_tokens: 8,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct HostedMcpProvider {
    model: Arc<HostedMcpModel>,
}

impl ModelProvider for HostedMcpProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let hosted_mcp = HostedMCPTool::new("mcp", "DeepWiki hosted MCP server")
        .with_hosted_tool_option("server_label", "deepwiki".into())
        .with_hosted_tool_option("server_url", "https://mcp.deepwiki.com/mcp".into())
        .with_hosted_tool_option("require_approval", "never".into());

    let agent = Agent::builder("Assistant")
        .instructions("Use the DeepWiki hosted MCP server to inspect repositories.")
        .model_settings(ModelSettings {
            tool_choice: Some("mcp".to_owned()),
            ..ModelSettings::default()
        })
        .tool(hosted_mcp)
        .build();

    let result = Runner::new()
        .with_model_provider(Arc::new(HostedMcpProvider::default()))
        .run(
            &agent,
            "Which language is the repository openai/openai-agents-python written in?",
        )
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
