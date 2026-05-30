use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse,
    OpenAIResponsesCompactionArgs, OpenAIResponsesCompactionAwareSession,
    OpenAIResponsesCompactionMode, OpenAIResponsesCompactionSession, OutputItem,
    Result as AgentsResult, Runner, Session, Usage,
};

#[derive(Clone, Default)]
struct CompactionModel;

#[async_trait]
impl Model for CompactionModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let latest = request
            .input
            .iter()
            .rev()
            .find_map(InputItem::as_text)
            .unwrap_or_default()
            .to_lowercase();
        let text = if latest.contains("tallest mountain") {
            "Mount Everest is the tallest mountain above sea level.".to_owned()
        } else if latest.contains("how tall") {
            "It is about 29,032 feet tall.".to_owned()
        } else if latest.contains("first climbed") {
            "It was first confirmed climbed in 1953.".to_owned()
        } else if latest.contains("expedition") {
            "Edmund Hillary and Tenzing Norgay reached the summit on that expedition.".to_owned()
        } else if latest.contains("what country") {
            "It sits on the border between Nepal and China.".to_owned()
        } else {
            "I can answer from the compacted session context.".to_owned()
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![
                OutputItem::Reasoning {
                    text: format!("tracked context item {}", request.input.len()),
                },
                OutputItem::Text { text },
            ],
            usage: Usage {
                input_tokens: 12,
                output_tokens: 9,
            },
            response_id: Some(format!("resp_{}", request.input.len())),
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct CompactionProvider {
    model: Arc<CompactionModel>,
}

impl ModelProvider for CompactionProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let session = OpenAIResponsesCompactionSession::new("demo-session")
        .with_model("gpt-4.1")?
        .with_mode(OpenAIResponsesCompactionMode::Input)
        .with_compaction_threshold(4);
    let agent = Agent::builder("Assistant")
        .instructions("Reply concisely. Keep answers to 1-2 sentences.")
        .build();
    let runner = Runner::new().with_model_provider(Arc::new(CompactionProvider::default()));

    println!("=== Compaction Session Example ===\n");
    let prompts = [
        "What is the tallest mountain in the world?",
        "How tall is it in feet?",
        "When was it first climbed?",
        "Who was on that expedition?",
        "What country is the mountain in?",
    ];

    for (index, prompt) in prompts.iter().enumerate() {
        println!("Turn {}:", index + 1);
        println!("User: {prompt}");
        let result = runner.run_with_session(&agent, *prompt, &session).await?;
        println!("Assistant: {}\n", result.final_output.unwrap_or_default());
    }

    print_session_state("Session State (Auto Compaction)", &session).await?;

    println!("=== Manual Compaction ===");
    session
        .run_compaction(Some(OpenAIResponsesCompactionArgs {
            force: Some(true),
            compaction_mode: Some("previous_response_id".to_owned()),
            response_id: Some("resp_manual".to_owned()),
            ..OpenAIResponsesCompactionArgs::default()
        }))
        .await?;
    println!("Done\n");

    print_session_state("Session State (Manual Compaction)", &session).await?;
    Ok(())
}

async fn print_session_state(
    label: &str,
    session: &OpenAIResponsesCompactionSession,
) -> Result<(), AgentsError> {
    let items = session.get_items().await?;
    println!("=== {label} ===");
    println!("Total items: {}", items.len());
    println!(
        "Compaction candidates: {}",
        session.compaction_candidate_count().await?
    );
    for item in items {
        match item {
            InputItem::Text { .. } => println!("  - message"),
            InputItem::Json { value } => {
                let item_type = value
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("json");
                println!("  - {item_type}");
            }
        }
    }
    println!();
    Ok(())
}
