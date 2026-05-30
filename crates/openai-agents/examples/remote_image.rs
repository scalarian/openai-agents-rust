use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, Usage,
};
use serde_json::{Value, json};

const URL: &str =
    "https://images.unsplash.com/photo-1505761671935-60b3a7427bad?auto=format&fit=crop&w=400&q=80";

#[derive(Clone, Default)]
struct RemoteImageModel;

#[async_trait]
impl Model for RemoteImageModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let text = if has_image_url(&request.input, URL) {
            "The remote image URL points to a city street scene.".to_owned()
        } else {
            "No remote image input found.".to_owned()
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
struct RemoteImageProvider {
    model: Arc<RemoteImageModel>,
}

impl ModelProvider for RemoteImageProvider {
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
                "content": [{
                    "type": "input_image",
                    "detail": "auto",
                    "image_url": URL
                }]
            }),
        },
        InputItem::Json {
            value: json!({
                "role": "user",
                "content": "What do you see in this image?"
            }),
        },
    ];

    let result = Runner::new()
        .with_model_provider(Arc::new(RemoteImageProvider::default()))
        .run_items(&agent, input)
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}

fn has_image_url(input: &[InputItem], expected_url: &str) -> bool {
    input.iter().any(|item| {
        let InputItem::Json { value } = item else {
            return false;
        };
        content_items(value).any(|content| {
            content.get("type").and_then(Value::as_str) == Some("input_image")
                && content.get("image_url").and_then(Value::as_str) == Some(expected_url)
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
