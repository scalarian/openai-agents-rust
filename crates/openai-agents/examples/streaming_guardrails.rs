use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use openai_agents::{
    Agent, AgentsError, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, RunItem, Runner, StreamEvent, Usage,
};

#[derive(Clone, Default)]
struct LongAnswerModel;

#[async_trait]
impl Model for LongAnswerModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text {
                text: "A black hole is a region of spacetime where gravity is so strong that light cannot escape. Around it, matter can form a hot accretion disk, time can appear distorted to distant observers, and nearby objects follow paths shaped by extreme curvature.".to_owned(),
            }],
            usage: Usage {
                input_tokens: 10,
                output_tokens: 48,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct LongAnswerProvider {
    model: Arc<LongAnswerModel>,
}

impl ModelProvider for LongAnswerProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

fn check_readability_for_ten_year_old(text: &str) -> (bool, &'static str) {
    if text.len() > 120 && (text.contains("spacetime") || text.contains("accretion")) {
        (
            false,
            "The answer uses advanced terms before explaining them simply.",
        )
    } else {
        (true, "The answer is still short enough.")
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("Assistant")
        .instructions("You are a helpful assistant. You always write long, detailed responses.")
        .build();
    let runner = Runner::new().with_model_provider(Arc::new(LongAnswerProvider::default()));
    let result = runner
        .run_streamed(&agent, "What is a black hole, and how does it behave?")
        .await?;

    let mut current_text = String::new();
    let mut next_guardrail_check_len = 120usize;
    let mut events = result.stream_events();
    while let Some(event) = events.next().await {
        if let StreamEvent::RunItemEvent(event) = event
            && let RunItem::MessageOutput {
                content: OutputItem::Text { text },
            } = event.item
        {
            for chunk in text.as_bytes().chunks(60) {
                let chunk = String::from_utf8_lossy(chunk);
                print!("{chunk}");
                current_text.push_str(&chunk);
                if current_text.len() >= next_guardrail_check_len {
                    println!("\nRunning guardrail check");
                    let (allowed, reason) = check_readability_for_ten_year_old(&current_text);
                    if !allowed {
                        println!("\n================\n");
                        println!("Guardrail triggered. Reasoning:\n{reason}");
                        return Ok(());
                    }
                    next_guardrail_check_len += 120;
                }
            }
        }
    }

    let (allowed, reason) = check_readability_for_ten_year_old(&current_text);
    if !allowed {
        println!("\n================\n");
        println!("Guardrail triggered. Reasoning:\n{reason}");
    }
    Ok(())
}
