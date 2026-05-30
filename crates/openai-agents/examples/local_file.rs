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

const FILEPATH: &str = "crates/openai-agents/examples/media/partial_o3-and-o4-mini-system-card.pdf";

#[derive(Clone, Default)]
struct FileQuestionModel;

#[async_trait]
impl Model for FileQuestionModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let answer = local_file_sentence(&request.input)
            .unwrap_or_else(|| "No file input found.".to_owned());

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text { text: answer }],
            usage: Usage {
                input_tokens: 16,
                output_tokens: 8,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct FileQuestionProvider {
    model: Arc<FileQuestionModel>,
}

impl ModelProvider for FileQuestionProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("Assistant")
        .instructions("You are a helpful assistant.")
        .build();
    let b64_file = file_to_base64(FILEPATH)?;
    let input = vec![
        InputItem::Json {
            value: json!({
                "role": "user",
                "content": [{
                    "type": "input_file",
                    "file_data": format!("data:application/pdf;base64,{b64_file}"),
                    "filename": "partial_o3-and-o4-mini-system-card.pdf"
                }]
            }),
        },
        InputItem::Json {
            value: json!({
                "role": "user",
                "content": "What is the first sentence of the introduction?"
            }),
        },
    ];

    let result = Runner::new()
        .with_model_provider(Arc::new(FileQuestionProvider::default()))
        .run_items(&agent, input)
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}

fn file_to_base64(path: &str) -> Result<String, AgentsError> {
    let bytes = fs::read(path).map_err(|error| AgentsError::message(error.to_string()))?;
    Ok(general_purpose::STANDARD.encode(bytes))
}

fn local_file_sentence(input: &[InputItem]) -> Option<String> {
    let data_url = input.iter().find_map(input_file_data)?;
    let encoded = data_url.rsplit_once(',').map(|(_, data)| data)?;
    let bytes = general_purpose::STANDARD.decode(encoded).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let introduction = text.split_once("Introduction")?.1.trim();
    introduction
        .split_once('.')
        .map(|(sentence, _)| format!("{}.", sentence.trim()))
}

fn input_file_data(item: &InputItem) -> Option<&str> {
    let InputItem::Json { value } = item else {
        return None;
    };
    content_items(value).find_map(|content| {
        if content.get("type").and_then(Value::as_str) == Some("input_file") {
            content.get("file_data").and_then(Value::as_str)
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
