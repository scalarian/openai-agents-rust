use std::env;
use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, HostedMCPTool, Model, ModelProvider, ModelRequest, ModelResponse,
    OutputItem, Result as AgentsResult, Runner, Usage,
};
use serde_json::Value;

#[derive(Clone, Default)]
struct ConnectorModel;

#[async_trait]
impl Model for ConnectorModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let calendar_connector = request.tools.iter().find(|tool| {
            tool.name == "mcp"
                && tool
                    .hosted_tool_options
                    .get("connector_id")
                    .and_then(Value::as_str)
                    == Some("connector_googlecalendar")
        });

        let text = calendar_connector
            .map(|tool| {
                let label = tool
                    .hosted_tool_options
                    .get("server_label")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                let has_auth = tool
                    .hosted_tool_options
                    .get("authorization")
                    .and_then(Value::as_str)
                    .is_some_and(|value| value != "dummy");
                format!("Google Calendar connector configured: label={label}; authorization_present={has_auth}")
            })
            .unwrap_or_else(|| "Google Calendar connector was not configured.".to_owned());

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text { text }],
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
struct ConnectorProvider {
    model: Arc<ConnectorModel>,
}

impl ModelProvider for ConnectorProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let authorization =
        env::var("GOOGLE_CALENDAR_AUTHORIZATION").unwrap_or_else(|_| "dummy".to_owned());
    if authorization == "dummy" {
        println!("Using dummy GOOGLE_CALENDAR_AUTHORIZATION for configuration-only run.");
    }

    let hosted_mcp = HostedMCPTool::new("mcp", "Google Calendar hosted MCP connector")
        .with_hosted_tool_option("server_label", "google_calendar".into())
        .with_hosted_tool_option("connector_id", "connector_googlecalendar".into())
        .with_hosted_tool_option("authorization", authorization.into())
        .with_hosted_tool_option("require_approval", "never".into());

    let agent = Agent::builder("Assistant")
        .instructions("You are a helpful assistant that can help a user with their calendar.")
        .tool(hosted_mcp)
        .build();

    let result = Runner::new()
        .with_model_provider(Arc::new(ConnectorProvider::default()))
        .run(&agent, "What is my schedule for today?")
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
