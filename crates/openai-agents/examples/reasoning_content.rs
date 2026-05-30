use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse,
    ModelSettings, OutputItem, ReasoningSettings, Result as AgentsResult, RunItem, Runner,
    StreamEvent, Usage,
};

#[derive(Clone, Default)]
struct ReasoningContentModel;

#[async_trait]
impl Model for ReasoningContentModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let prompt = request
            .input
            .iter()
            .filter_map(InputItem::as_text)
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();
        let summary = request
            .settings
            .reasoning
            .as_ref()
            .and_then(|reasoning| reasoning.summary.as_deref())
            .unwrap_or("auto");
        let (reasoning, answer) = if prompt.contains("15 x 27") {
            (
                "Break 27 into 20 and 7, then compute 15*20 + 15*7.",
                "15 x 27 = 405.",
            )
        } else {
            (
                "29 squared equals 841, so the square root is 29.",
                "The square root of 841 is 29.",
            )
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![
                OutputItem::Reasoning {
                    text: format!("{reasoning} summary={summary}"),
                },
                OutputItem::Text {
                    text: answer.to_owned(),
                },
            ],
            usage: Usage {
                input_tokens: 11,
                output_tokens: 8,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct ReasoningContentProvider {
    model: Arc<ReasoningContentModel>,
}

impl ModelProvider for ReasoningContentProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("Reasoning Agent")
        .instructions("Explain your reasoning briefly before answering.")
        .model_settings(ModelSettings {
            reasoning: Some(ReasoningSettings {
                effort: Some("medium".to_owned()),
                summary: Some("detailed".to_owned()),
            }),
            ..ModelSettings::default()
        })
        .build();
    let runner = Runner::new().with_model_provider(Arc::new(ReasoningContentProvider::default()));

    println!("=== Reasoning Content: Non-streaming ===");
    let result = runner
        .run(
            &agent,
            "What is the square root of 841? Explain your reasoning.",
        )
        .await?;
    for item in &result.new_items {
        if let RunItem::Reasoning { text } = item {
            println!("reasoning={text}");
        }
    }
    println!("final_output={}", result.final_output.unwrap_or_default());

    println!("=== Reasoning Content: Streaming ===");
    let streamed = runner
        .run_streamed(&agent, "What is 15 x 27? Explain your reasoning.")
        .await?;
    let mut events = streamed.stream_events();
    while let Some(event) = events.next().await {
        match event {
            StreamEvent::RunItemEvent(event) => match event.item {
                RunItem::Reasoning { text } => println!("stream_reasoning={text}"),
                RunItem::MessageOutput {
                    content: OutputItem::Text { text },
                } => println!("stream_output={text}"),
                _ => {}
            },
            StreamEvent::AgentUpdated(_)
            | StreamEvent::RawResponseEvent(_)
            | StreamEvent::Lifecycle(_) => {}
        }
    }
    Ok(())
}
