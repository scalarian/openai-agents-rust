use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse,
    OpenAIResponsesCompactionArgs, OpenAIResponsesCompactionAwareSession,
    OpenAIResponsesCompactionMode, OpenAIResponsesCompactionSession, OutputItem,
    Result as AgentsResult, Runner, Session, Usage,
};

#[derive(Clone, Default)]
struct StatelessCompactionModel;

#[async_trait]
impl Model for StatelessCompactionModel {
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
            "Edmund Hillary and Tenzing Norgay were on the first confirmed summit team.".to_owned()
        } else {
            "I can answer from stateless compacted input.".to_owned()
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![
                OutputItem::Reasoning {
                    text: format!("stateless context item {}", request.input.len()),
                },
                OutputItem::Text { text },
            ],
            usage: Usage {
                input_tokens: 10,
                output_tokens: 8,
            },
            response_id: Some(format!("resp_stateless_{}", request.input.len())),
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct StatelessCompactionProvider {
    model: Arc<StatelessCompactionModel>,
}

impl ModelProvider for StatelessCompactionProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let session = OpenAIResponsesCompactionSession::new("demo-session")
        .with_model("gpt-4.1")?
        .with_mode(OpenAIResponsesCompactionMode::Auto)
        .with_compaction_threshold(3);
    let agent = Agent::builder("Assistant")
        .instructions("Reply concisely. Keep answers to 1-2 sentences.")
        .model_settings(openai_agents::ModelSettings {
            store: Some(false),
            ..openai_agents::ModelSettings::default()
        })
        .build();
    let runner =
        Runner::new().with_model_provider(Arc::new(StatelessCompactionProvider::default()));

    println!("=== Stateless Compaction Session Example ===\n");
    let prompts = [
        "What is the tallest mountain in the world?",
        "How tall is it in feet?",
        "When was it first climbed?",
        "Who was on that expedition?",
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
            ..OpenAIResponsesCompactionArgs::default()
        }))
        .await?;
    println!("Done\n");

    print_session_state("Final Session State", &session).await?;
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
