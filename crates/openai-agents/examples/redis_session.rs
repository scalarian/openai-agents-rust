use std::env;
use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::extensions::RedisSession;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, Session, Usage,
};

const DEFAULT_REDIS_URL: &str = "redis://localhost:6379/0";

#[derive(Clone, Default)]
struct RedisSessionModel;

#[async_trait]
impl Model for RedisSessionModel {
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
            "I can answer from Redis-backed session context.".to_owned()
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text { text }],
            usage: Usage {
                input_tokens: 9,
                output_tokens: 5,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct RedisSessionProvider {
    model: Arc<RedisSessionModel>,
}

impl ModelProvider for RedisSessionProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_owned());
    println!("=== Redis Session Example ===");
    println!("redis_url={redis_url}");

    let session = RedisSession::from_url("redis_conversation_123", redis_url.clone())?;
    if let Err(error) = session.clear_session().await {
        println!("Redis server is not available: {error}");
        println!("Start Redis locally or set REDIS_URL to run the full example.");
        return Ok(());
    }

    println!("Connected to Redis successfully.");
    let agent = Agent::builder("Assistant")
        .instructions("Reply very concisely.")
        .build();
    let runner = Runner::new().with_model_provider(Arc::new(RedisSessionProvider::default()));

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

    let all_items = session.get_items().await?;
    println!("total_items={}", all_items.len());
    session.clear_session().await?;
    println!("session_cleared=true");
    Ok(())
}
