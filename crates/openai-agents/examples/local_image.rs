use std::fs;
use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, Usage,
};
use serde_json::{Value, json};

const FILEPATH: &str = "crates/openai-agents/examples/media/image_bison.svg";

#[derive(Clone, Default)]
struct ImageQuestionModel;

#[async_trait]
impl Model for ImageQuestionModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let text = if local_image_mentions_bison(&request.input) {
            "The image shows a stylized bison standing on grass.".to_owned()
        } else {
            "No local image input found.".to_owned()
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text { text }],
            usage: Usage {
                input_tokens: 14,
                output_tokens: 8,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct ImageQuestionProvider {
    model: Arc<ImageQuestionModel>,
}

impl ModelProvider for ImageQuestionProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let b64_image = image_to_base64(FILEPATH)?;
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
                    "image_url": format!("data:image/svg+xml;base64,{b64_image}")
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
        .with_model_provider(Arc::new(ImageQuestionProvider::default()))
        .run_items(&agent, input)
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}

fn image_to_base64(path: &str) -> Result<String, AgentsError> {
    let bytes = fs::read(path).map_err(|error| AgentsError::message(error.to_string()))?;
    Ok(general_purpose::STANDARD.encode(bytes))
}

fn local_image_mentions_bison(input: &[InputItem]) -> bool {
    input
        .iter()
        .find_map(input_image_data)
        .and_then(|data_url| data_url.rsplit_once(',').map(|(_, data)| data.to_owned()))
        .and_then(|encoded| general_purpose::STANDARD.decode(encoded).ok())
        .map(|bytes| String::from_utf8_lossy(&bytes).contains("bison"))
        .unwrap_or(false)
}

fn input_image_data(item: &InputItem) -> Option<&str> {
    let InputItem::Json { value } = item else {
        return None;
    };
    content_items(value).find_map(|content| {
        if content.get("type").and_then(Value::as_str) == Some("input_image") {
            content.get("image_url").and_then(Value::as_str)
        } else {
            None
        }
    })
}

fn content_items(value: &Value) -> impl Iterator<Item = &Value> {
    value
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}
