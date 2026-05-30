use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, Usage,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct OutlineCheck {
    good_quality: bool,
    is_scifi: bool,
}

#[derive(Clone, Default)]
struct StoryFlowModel;

#[async_trait]
impl Model for StoryFlowModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let instructions = request.instructions.unwrap_or_default().to_lowercase();
        let text = if instructions.contains("judge") {
            r#"{"good_quality":true,"is_scifi":true}"#.to_owned()
        } else if instructions.contains("write a short story") {
            "Under the ice, the signal taught the cartographer how to draw a map home.".to_owned()
        } else if instructions.contains("outline") {
            "A lunar cartographer finds an ancient signal under Europa's ice.".to_owned()
        } else {
            "Ready.".to_owned()
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text { text }],
            usage: Usage {
                input_tokens: 4,
                output_tokens: 8,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct StoryFlowProvider {
    model: Arc<StoryFlowModel>,
}

impl ModelProvider for StoryFlowProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let outline_agent = Agent::builder("story_outline_agent")
        .instructions("Generate a very short story outline based on the user's input.")
        .build();
    let checker_agent = Agent::builder("outline_checker_agent")
        .instructions(
            "Read the given story outline, judge the quality, and determine if it is scifi.",
        )
        .build();
    let story_agent = Agent::builder("story_agent")
        .instructions("Write a short story based on the given outline.")
        .build();

    let runner = Runner::new().with_model_provider(Arc::new(StoryFlowProvider::default()));

    let outline = runner
        .run(&outline_agent, "Write a short sci-fi story.")
        .await?;
    let outline_text = outline.final_output_text().unwrap_or_default();
    println!("outline={outline_text}");

    let check = runner.run(&checker_agent, outline_text).await?;
    let check: OutlineCheck = serde_json::from_str(check.final_output_text().unwrap_or_default())
        .map_err(|error| AgentsError::message(error.to_string()))?;

    if !check.good_quality || !check.is_scifi {
        println!("stopped=true");
        return Ok(());
    }

    let story = runner.run(&story_agent, outline_text).await?;
    println!("story={}", story.final_output_text().unwrap_or_default());
    Ok(())
}
