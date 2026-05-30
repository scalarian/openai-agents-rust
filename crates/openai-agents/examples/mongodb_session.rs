use std::env;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use mongodb::{Client, options::ClientOptions};
use openai_agents::extensions::MongoDBSession;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, Session, Usage,
};

const DEFAULT_MONGO_URI: &str = "mongodb://localhost:27017";
const DEFAULT_DATABASE: &str = "agents_example";
const DEFAULT_SERVER_SELECTION_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone, Default)]
struct MongoDBSessionModel;

#[async_trait]
impl Model for MongoDBSessionModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let history = request
            .input
            .iter()
            .filter_map(InputItem::as_text)
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();
        let text = if history.contains("capital of france") {
            "Paris.".to_owned()
        } else if history.contains("population") {
            "California has about 39 million people.".to_owned()
        } else if history.contains("what state") {
            "It is in California.".to_owned()
        } else if history.contains("golden gate bridge") {
            "San Francisco.".to_owned()
        } else {
            "I can answer from MongoDB-backed session context.".to_owned()
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
struct MongoDBSessionProvider {
    model: Arc<MongoDBSessionModel>,
}

impl ModelProvider for MongoDBSessionProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let mongo_uri = env::var("MONGO_URI").unwrap_or_else(|_| DEFAULT_MONGO_URI.to_owned());
    let database = env::var("MONGO_DATABASE").unwrap_or_else(|_| DEFAULT_DATABASE.to_owned());

    println!("=== MongoDB Session Example ===");
    println!("mongo_uri={mongo_uri}");
    println!("database={database}");

    let mut options = ClientOptions::parse(&mongo_uri)
        .await
        .map_err(|error| AgentsError::message(error.to_string()))?;
    options
        .server_selection_timeout
        .get_or_insert(DEFAULT_SERVER_SELECTION_TIMEOUT);
    let client =
        Client::with_options(options).map_err(|error| AgentsError::message(error.to_string()))?;
    let session_a = MongoDBSession::new("conversation_a", client.clone(), database.clone());
    let session_b = MongoDBSession::new("conversation_b", client, database);

    if !session_a.ping().await {
        println!("MongoDB is not available at {mongo_uri}");
        println!("Start it with: docker run -d -p 27017:27017 mongo");
        return Ok(());
    }

    session_a.clear_session().await?;
    session_b.clear_session().await?;

    let agent = Agent::builder("Assistant")
        .instructions("Reply very concisely.")
        .build();
    let runner = Runner::new().with_model_provider(Arc::new(MongoDBSessionProvider::default()));

    println!("=== Session A ===");
    let first = runner
        .run_with_session(
            &agent,
            "What city is the Golden Gate Bridge in?",
            &session_a,
        )
        .await?;
    println!("turn_1={}", first.final_output.unwrap_or_default());

    let second = runner
        .run_with_session(&agent, "What state is it in?", &session_a)
        .await?;
    println!("turn_2={}", second.final_output.unwrap_or_default());

    let third = runner
        .run_with_session(&agent, "What's the population of that state?", &session_a)
        .await?;
    println!("turn_3={}", third.final_output.unwrap_or_default());

    println!("=== Session B ===");
    let isolated = runner
        .run_with_session(&agent, "What is the capital of France?", &session_b)
        .await?;
    println!("turn_1={}", isolated.final_output.unwrap_or_default());

    let a_items = session_a.get_items().await?;
    let b_items = session_b.get_items().await?;
    println!("session_a_items={}", a_items.len());
    println!("session_b_items={}", b_items.len());

    session_a.clear_session().await?;
    session_b.clear_session().await?;
    println!("sessions_cleared=true");
    Ok(())
}
