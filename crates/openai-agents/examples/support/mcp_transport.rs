use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, MCPServer, MCPTool, Model, ModelProvider, ModelRequest,
    ModelResponse, OutputItem, Result as AgentsResult, Runner, ToolOutput, Usage,
};
use serde_json::{Value, json};

pub fn demo_tools() -> Vec<MCPTool> {
    vec![
        MCPTool {
            name: "add".to_owned(),
            description: Some("Add two numbers.".to_owned()),
            input_schema: Some(json!({
                "type": "object",
                "properties": {
                    "a": { "type": "integer" },
                    "b": { "type": "integer" }
                },
                "required": ["a", "b"],
                "additionalProperties": false
            })),
            ..MCPTool::default()
        },
        MCPTool {
            name: "get_current_weather".to_owned(),
            description: Some("Get the current weather for a city.".to_owned()),
            input_schema: Some(json!({
                "type": "object",
                "properties": {
                    "city": { "type": "string" }
                },
                "required": ["city"],
                "additionalProperties": false
            })),
            ..MCPTool::default()
        },
        MCPTool {
            name: "get_secret_word".to_owned(),
            description: Some("Return the demo secret word.".to_owned()),
            input_schema: Some(json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            })),
            ..MCPTool::default()
        },
    ]
}

pub fn demo_tool_outputs() -> HashMap<String, ToolOutput> {
    HashMap::from([
        ("add".to_owned(), ToolOutput::from("29")),
        (
            "get_current_weather".to_owned(),
            ToolOutput::from("The weather in Tokyo is sunny with a light breeze and 20 C."),
        ),
        ("get_secret_word".to_owned(), ToolOutput::from("apple")),
    ])
}

pub async fn run_demo_questions(server: Arc<dyn MCPServer>) -> Result<(), AgentsError> {
    let agent = Agent::builder("Assistant")
        .instructions("Use the MCP tools to answer the questions.")
        .mcp_server(server)
        .build();
    let runner = Runner::new().with_model_provider(Arc::new(TransportDemoProvider::default()));

    for message in [
        "Add these numbers: 7 and 22.",
        "What's the weather in Tokyo?",
        "What's the secret word?",
    ] {
        println!("Running: {message}");
        let result = runner.run(&agent, message).await?;
        println!("{}", result.final_output.unwrap_or_default());
        println!();
    }

    Ok(())
}

#[derive(Clone, Default)]
struct TransportDemoModel;

#[async_trait]
impl Model for TransportDemoModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if let Some(sum) = latest_tool_output(&request.input, "add") {
            vec![OutputItem::Text {
                text: format!("7 + 22 = {sum}."),
            }]
        } else if let Some(weather) = latest_tool_output(&request.input, "get_current_weather") {
            vec![OutputItem::Text { text: weather }]
        } else if let Some(secret) = latest_tool_output(&request.input, "get_secret_word") {
            vec![OutputItem::Text {
                text: format!("The secret word is {secret}."),
            }]
        } else if input_mentions(&request.input, "weather") {
            vec![OutputItem::ToolCall {
                call_id: "call-weather".to_owned(),
                tool_name: "get_current_weather".to_owned(),
                arguments: json!({ "city": "Tokyo" }),
                namespace: None,
            }]
        } else if input_mentions(&request.input, "secret word") {
            vec![OutputItem::ToolCall {
                call_id: "call-secret".to_owned(),
                tool_name: "get_secret_word".to_owned(),
                arguments: json!({}),
                namespace: None,
            }]
        } else if request.tools.iter().any(|tool| tool.name == "add") {
            vec![OutputItem::ToolCall {
                call_id: "call-add".to_owned(),
                tool_name: "add".to_owned(),
                arguments: json!({ "a": 7, "b": 22 }),
                namespace: None,
            }]
        } else {
            vec![OutputItem::Text {
                text: "No MCP tools are available.".to_owned(),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 16,
                output_tokens: 12,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct TransportDemoProvider {
    model: Arc<TransportDemoModel>,
}

impl ModelProvider for TransportDemoProvider {
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
            .map(ToOwned::to_owned)
            .or_else(|| {
                value
                    .get("output")
                    .map(Value::to_string)
                    .map(|text| text.trim_matches('"').to_owned())
            })
    })
}

fn input_mentions(input: &[InputItem], needle: &str) -> bool {
    let needle = needle.to_ascii_lowercase();
    input.iter().any(|item| match item {
        InputItem::Text { text } => text.to_ascii_lowercase().contains(&needle),
        InputItem::Json { value } => value.to_string().to_ascii_lowercase().contains(&needle),
    })
}
