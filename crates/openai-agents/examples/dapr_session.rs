use std::env;
use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::extensions::DaprSession;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, Session, Usage,
};

const DEFAULT_DAPR_HTTP_ENDPOINT: &str = "http://localhost:3500";
const DEFAULT_STATE_STORE: &str = "statestore";

#[derive(Clone, Default)]
struct DaprSessionModel;

#[async_trait]
impl Model for DaprSessionModel {
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
            "I can answer from Dapr-backed session context.".to_owned()
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
struct DaprSessionProvider {
    model: Arc<DaprSessionModel>,
}

impl ModelProvider for DaprSessionProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let address =
        env::var("DAPR_HTTP_ENDPOINT").unwrap_or_else(|_| DEFAULT_DAPR_HTTP_ENDPOINT.to_owned());
    let state_store =
        env::var("DAPR_STATE_STORE").unwrap_or_else(|_| DEFAULT_STATE_STORE.to_owned());

    println!("=== Dapr Session Example ===");
    println!("dapr_http_endpoint={address}");
    println!("state_store={state_store}");
    println!(
        "start_command=dapr run --app-id myapp --dapr-http-port 3500 --resources-path ./components"
    );

    let session = DaprSession::from_address("dapr_conversation_123", state_store.clone(), &address);
    if let Err(error) = session.clear_session().await {
        println!("Dapr sidecar or state store is not available: {error}");
        println!("Start Dapr locally or set DAPR_HTTP_ENDPOINT and DAPR_STATE_STORE.");
        return Ok(());
    }

    println!("Connected to Dapr successfully.");
    let agent = Agent::builder("Assistant")
        .instructions("Reply very concisely.")
        .build();
    let runner = Runner::new().with_model_provider(Arc::new(DaprSessionProvider::default()));

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

    let latest_items = session.get_items_with_limit(Some(2)).await?;
    println!("latest_items={}", latest_items.len());

    let isolated_session =
        DaprSession::from_address("different_conversation_456", state_store.clone(), &address);
    isolated_session.clear_session().await?;
    let isolated = runner
        .run_with_session(
            &agent,
            "Hello, this is a new conversation!",
            &isolated_session,
        )
        .await?;
    println!(
        "isolated_assistant={}",
        isolated.final_output.unwrap_or_default()
    );

    let original_items = session.get_items().await?;
    let isolated_items = isolated_session.get_items().await?;
    println!("original_items={}", original_items.len());
    println!("isolated_items={}", isolated_items.len());

    let ttl_session = DaprSession::from_address("ttl_demo_session", state_store, &address)
        .with_key_prefix("agents-session-demo-")
        .with_ttl_seconds(3600);
    println!(
        "ttl_seconds={}",
        ttl_session.ttl_seconds.unwrap_or_default()
    );
    println!("key_prefix={}", ttl_session.key_prefix);

    session.clear_session().await?;
    isolated_session.clear_session().await?;
    ttl_session.clear_session().await?;
    println!("sessions_cleared=true");
    Ok(())
}
