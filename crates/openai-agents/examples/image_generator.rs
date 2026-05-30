use std::env;
use std::fs;
use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose;
use openai_agents::{
    Agent, AgentsError, ImageGenerationToolOptions, Model, ModelProvider, ModelRequest,
    ModelResponse, OutputItem, Result as AgentsResult, RunItem, Runner, Usage,
    image_generation_tool_with_options,
};
use serde_json::{Value, json};

const GENERATED_IMAGE: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII=";

#[derive(Clone, Default)]
struct ImageGeneratorModel;

#[async_trait]
impl Model for ImageGeneratorModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let has_low_quality_tool = request.tools.iter().any(|tool| {
            tool.name == "image_generation"
                && tool
                    .hosted_tool_options
                    .get("quality")
                    .and_then(Value::as_str)
                    == Some("low")
        });
        let output = if has_low_quality_tool {
            vec![
                OutputItem::Json {
                    value: json!({
                        "type": "image_generation_call",
                        "id": "ig_frog_pizza",
                        "result": GENERATED_IMAGE
                    }),
                },
                OutputItem::Text {
                    text:
                        "Generated a low-quality preview image for a comic-book frog eating pizza."
                            .to_owned(),
                },
            ]
        } else {
            vec![OutputItem::Text {
                text: "No image generation tool was configured.".to_owned(),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
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
struct ImageGeneratorProvider {
    model: Arc<ImageGeneratorModel>,
}

impl ModelProvider for ImageGeneratorProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("Image generator")
        .instructions("Always use the image generation tool when the user asks for a new image.")
        .tool(image_generation_tool_with_options(
            ImageGenerationToolOptions {
                quality: Some("low".to_owned()),
            },
        ))
        .build();

    println!("Generating image, this may take a while...");
    let result = Runner::new()
        .with_model_provider(Arc::new(ImageGeneratorProvider::default()))
        .run(
            &agent,
            "Create an image of a frog eating a pizza, comic book style.",
        )
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    if let Some(encoded) = generated_image_result(&result.new_items) {
        let path = env::temp_dir().join("openai_agents_image_generation_example.png");
        let bytes = general_purpose::STANDARD
            .decode(encoded)
            .map_err(|error| AgentsError::message(error.to_string()))?;
        fs::write(&path, bytes).map_err(|error| AgentsError::message(error.to_string()))?;
        println!("Saved generated image to: {}", path.display());
    } else {
        println!("No image_generation_call item was returned.");
    }
    Ok(())
}

fn generated_image_result(items: &[RunItem]) -> Option<&str> {
    items.iter().find_map(|item| {
        let RunItem::MessageOutput {
            content: OutputItem::Json { value },
        } = item
        else {
            return None;
        };
        if value.get("type").and_then(Value::as_str) != Some("image_generation_call") {
            return None;
        }
        value.get("result").and_then(Value::as_str)
    })
}
