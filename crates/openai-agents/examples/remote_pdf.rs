use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, Usage,
};
use serde_json::{Value, json};

const URL: &str = "https://www.berkshirehathaway.com/letters/2024ltr.pdf";

#[derive(Clone, Default)]
struct RemotePdfModel;

#[async_trait]
impl Model for RemotePdfModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let text = if has_file_url(&request.input, URL) {
            "The remote PDF input points to Berkshire Hathaway's 2024 shareholder letter."
                .to_owned()
        } else {
            "No remote PDF input found.".to_owned()
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text { text }],
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
struct RemotePdfProvider {
    model: Arc<RemotePdfModel>,
}

impl ModelProvider for RemotePdfProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("Assistant")
        .instructions("You are a helpful assistant.")
        .build();
    let input = vec![
        InputItem::Json {
            value: json!({
                "role": "user",
                "content": [{"type": "input_file", "file_url": URL}]
            }),
        },
        InputItem::Json {
            value: json!({
                "role": "user",
                "content": "Can you summarize the letter?"
            }),
        },
    ];

    let result = Runner::new()
        .with_model_provider(Arc::new(RemotePdfProvider::default()))
        .run_items(&agent, input)
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}

fn has_file_url(input: &[InputItem], expected_url: &str) -> bool {
    input.iter().any(|item| {
        let InputItem::Json { value } = item else {
            return false;
        };
        content_items(value).any(|content| {
            content.get("type").and_then(Value::as_str) == Some("input_file")
                && content.get("file_url").and_then(Value::as_str) == Some(expected_url)
        })
    })
}

fn content_items(value: &Value) -> impl Iterator<Item = &Value> {
    value
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}
