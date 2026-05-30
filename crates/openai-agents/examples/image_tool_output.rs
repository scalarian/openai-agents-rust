use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, ToolOutput, ToolOutputImage, Usage, function_tool,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

const IMAGE_URL: &str =
    "https://images.unsplash.com/photo-1505761671935-60b3a7427bad?auto=format&fit=crop&w=400&q=80";

#[derive(Debug, Deserialize, JsonSchema)]
struct NoArgs {}

#[derive(Clone, Default)]
struct ImageDemoModel;

#[async_trait]
impl Model for ImageDemoModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if let Some((image_url, detail)) = image_tool_output(&request.input) {
            vec![OutputItem::Text {
                text: format!(
                    "The image tool returned image_url={image_url} with detail={detail}."
                ),
            }]
        } else {
            vec![OutputItem::ToolCall {
                call_id: "call-fetch-image".to_owned(),
                tool_name: "fetch_random_image".to_owned(),
                arguments: json!({}),
                namespace: None,
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 7,
                output_tokens: 9,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct ImageDemoProvider {
    model: Arc<ImageDemoModel>,
}

impl ModelProvider for ImageDemoProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

fn image_tool_output(input: &[InputItem]) -> Option<(String, String)> {
    input.iter().find_map(|item| {
        let InputItem::Json { value } = item else {
            return None;
        };
        let image = value.get("output")?.get("value")?;
        let image_url = image.get("image_url").and_then(Value::as_str)?.to_owned();
        let detail = image
            .get("detail")
            .and_then(Value::as_str)
            .unwrap_or("auto")
            .to_owned();
        Some((image_url, detail))
    })
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let fetch_random_image = function_tool(
        "fetch_random_image",
        "Fetch a random image.",
        |_ctx, _args: NoArgs| async move {
            println!("image_tool_called=true");
            Ok::<_, AgentsError>(ToolOutput::Image(ToolOutputImage {
                image_url: Some(IMAGE_URL.to_owned()),
                file_id: None,
                detail: Some("auto".to_owned()),
            }))
        },
    )?;

    let agent = Agent::builder("Assistant")
        .instructions("Use the image tool, then describe the returned image.")
        .function_tool(fetch_random_image)
        .build();

    let runner = Runner::new().with_model_provider(Arc::new(ImageDemoProvider::default()));
    let result = runner
        .run(
            &agent,
            "Fetch an image using the image tool, then describe it.",
        )
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
