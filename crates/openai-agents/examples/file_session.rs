use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::extensions::FileSession;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, Session, Usage,
};

#[derive(Clone, Default)]
struct FileSessionModel;

#[async_trait]
impl Model for FileSessionModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let history = request
            .input
            .iter()
            .filter_map(InputItem::as_text)
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();
        let text = if history.contains("what state") {
            "It is in California.".to_owned()
        } else if history.contains("golden gate bridge") {
            "San Francisco.".to_owned()
        } else {
            "I can answer from file-backed session context.".to_owned()
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text { text }],
            usage: Usage {
                input_tokens: 8,
                output_tokens: 5,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct FileSessionProvider {
    model: Arc<FileSessionModel>,
}

impl ModelProvider for FileSessionProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let dir = std::env::temp_dir().join("openai-agents-file-session-example");
    let session = FileSession::with_session_id(&dir, "conversation_123");
    session.clear_session().await?;

    let agent = Agent::builder("Assistant")
        .instructions("Reply very concisely.")
        .build();
    let runner = Runner::new().with_model_provider(Arc::new(FileSessionProvider::default()));

    println!("=== File Session Example ===");
    println!("items_path={}", session.items_path().display());

    let first = runner
        .run_with_session(&agent, "What city is the Golden Gate Bridge in?", &session)
        .await?;
    println!("assistant_1={}", first.final_output.unwrap_or_default());

    let rehydrated = FileSession::with_session_id(&dir, "conversation_123");
    let second = runner
        .run_with_session(&agent, "What state is it in?", &rehydrated)
        .await?;
    println!("assistant_2={}", second.final_output.unwrap_or_default());

    let all_items = rehydrated.get_items().await?;
    println!("total_items={}", all_items.len());

    rehydrated
        .save_state_json(r#"{"example":"saved alongside the session"}"#)
        .await?;
    println!(
        "state_saved={}",
        rehydrated.load_state_json().await?.is_some()
    );

    rehydrated.clear_session().await?;
    println!("session_cleared=true");
    Ok(())
}
