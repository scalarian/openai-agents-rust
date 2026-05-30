use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::extensions::AsyncSQLiteSession;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, Session, Usage,
};

#[derive(Clone, Default)]
struct AsyncSQLiteSessionModel;

#[async_trait]
impl Model for AsyncSQLiteSessionModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let history = request
            .input
            .iter()
            .filter_map(InputItem::as_text)
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();
        let text = if history.contains("population") {
            "California has about 39 million people.".to_owned()
        } else if history.contains("what state") {
            "It is in California.".to_owned()
        } else if history.contains("golden gate bridge") {
            "San Francisco.".to_owned()
        } else {
            "I can answer from async SQLite session context.".to_owned()
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
struct AsyncSQLiteSessionProvider {
    model: Arc<AsyncSQLiteSessionModel>,
}

impl ModelProvider for AsyncSQLiteSessionProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("Assistant")
        .instructions("Reply very concisely.")
        .build();
    let session = AsyncSQLiteSession::open_in_memory("conversation_123").await?;
    let runner = Runner::new().with_model_provider(Arc::new(AsyncSQLiteSessionProvider::default()));

    println!("=== AsyncSQLiteSession Example ===");
    println!("database_url={}", session.database_url);

    let first = runner
        .run_with_session(&agent, "What city is the Golden Gate Bridge in?", &session)
        .await?;
    println!("assistant_1={}", first.final_output.unwrap_or_default());

    let second = runner
        .run_with_session(&agent, "What state is it in?", &session)
        .await?;
    println!("assistant_2={}", second.final_output.unwrap_or_default());

    let third = runner
        .run_with_session(&agent, "What's the population of that state?", &session)
        .await?;
    println!("assistant_3={}", third.final_output.unwrap_or_default());

    let latest_items = session.get_items_with_limit(Some(2)).await?;
    println!("latest_items={}", latest_items.len());

    let all_items = session.get_items().await?;
    println!("total_items={}", all_items.len());
    Ok(())
}
