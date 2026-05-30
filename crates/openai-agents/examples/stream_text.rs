use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use openai_agents::{
    Agent, AgentsError, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, RunItem, Runner, StreamEvent, Usage,
};

#[derive(Clone, Default)]
struct TextStreamModel;

#[async_trait]
impl Model for TextStreamModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text {
                text: "Here are five concise jokes in one streamed message.".to_owned(),
            }],
            usage: Usage {
                input_tokens: 5,
                output_tokens: 9,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct TextStreamProvider {
    model: Arc<TextStreamModel>,
}

impl ModelProvider for TextStreamProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("Joker")
        .instructions("You are a helpful assistant.")
        .build();

    let runner = Runner::new().with_model_provider(Arc::new(TextStreamProvider::default()));
    let streamed = runner
        .run_streamed(&agent, "Please tell me 5 jokes.")
        .await?;

    let mut events = streamed.stream_events();
    while let Some(event) = events.next().await {
        if let StreamEvent::RunItemEvent(event) = event
            && let RunItem::MessageOutput { content } = event.item
            && let Some(text) = content.as_text()
        {
            print!("{text}");
        }
    }
    println!();

    streamed.wait_for_completion().await?;
    Ok(())
}
