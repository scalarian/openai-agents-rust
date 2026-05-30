use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, MCPServerStreamableHttp, MCPServerStreamableHttpParams, MCPTool,
    Model, ModelProvider, ModelRequest, ModelResponse, OutputItem, Result as AgentsResult, Runner,
    ToolOutput, Usage,
};
use serde_json::{Value, json};

#[derive(Clone, Default)]
struct DeepWikiModel;

#[async_trait]
impl Model for DeepWikiModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if let Some(answer) = latest_tool_output(&request.input, "ask_deepwiki") {
            vec![OutputItem::Text {
                text: format!("DeepWiki reports: {answer}"),
            }]
        } else if request.tools.iter().any(|tool| tool.name == "ask_deepwiki") {
            vec![OutputItem::ToolCall {
                call_id: "call-deepwiki".to_owned(),
                tool_name: "ask_deepwiki".to_owned(),
                arguments: json!({
                    "repository": "openai/codex",
                    "question": "What is the primary programming language?"
                }),
                namespace: None,
            }]
        } else {
            vec![OutputItem::Text {
                text: "No DeepWiki MCP tools are available.".to_owned(),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 18,
                output_tokens: 10,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct DeepWikiProvider {
    model: Arc<DeepWikiModel>,
}

impl ModelProvider for DeepWikiProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let url = env::var("MCP_STREAMABLE_HTTP_REMOTE_URL")
        .unwrap_or_else(|_| "https://mcp.deepwiki.com/mcp".to_owned());
    let server = MCPServerStreamableHttp::new(
        "DeepWiki MCP Streamable HTTP Server",
        MCPServerStreamableHttpParams {
            url,
            timeout_seconds: Some(15),
            sse_read_timeout_seconds: Some(300),
            ..MCPServerStreamableHttpParams::default()
        },
    )
    .with_tools(vec![MCPTool {
        name: "ask_deepwiki".to_owned(),
        description: Some("Ask DeepWiki a question about a GitHub repository.".to_owned()),
        input_schema: Some(json!({
            "type": "object",
            "properties": {
                "repository": { "type": "string" },
                "question": { "type": "string" }
            },
            "required": ["repository", "question"],
            "additionalProperties": false
        })),
        ..MCPTool::default()
    }])
    .with_tool_outputs(HashMap::from([(
        "ask_deepwiki".to_owned(),
        ToolOutput::from("The primary programming language for openai/codex is Rust."),
    )]));

    let agent = Agent::builder("DeepWiki Assistant")
        .instructions("Use the remote MCP tools to respond to repository questions.")
        .mcp_server(Arc::new(server))
        .build();

    let result = Runner::new()
        .with_model_provider(Arc::new(DeepWikiProvider::default()))
        .run(
            &agent,
            "For the repository openai/codex, tell me the primary programming language.",
        )
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
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
            .map(ToOwned::to_owned)
    })
}
