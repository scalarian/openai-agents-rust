use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse,
    OpenAIConversationsSession, OutputItem, Result as AgentsResult, Runner, Session, Usage,
};

#[derive(Clone, Default)]
struct OpenAISessionModel;

#[async_trait]
impl Model for OpenAISessionModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let latest = request
            .input
            .iter()
            .rev()
            .find_map(InputItem::as_text)
            .unwrap_or_default()
            .to_lowercase();
        let text = if latest.contains("population") {
            "California has about 39 million people.".to_owned()
        } else if latest.contains("what state") {
            "It is in California.".to_owned()
        } else if latest.contains("golden gate bridge") {
            "San Francisco.".to_owned()
        } else {
            "I can answer using the OpenAI conversation session.".to_owned()
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text { text }],
            usage: Usage {
                input_tokens: 8,
                output_tokens: 5,
            },
            response_id: Some(format!(
                "resp_{}",
                request.previous_response_id.as_deref().unwrap_or("start")
            )),
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct OpenAISessionProvider {
    model: Arc<OpenAISessionModel>,
}

impl ModelProvider for OpenAISessionProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("Assistant")
        .instructions("Reply very concisely.")
        .build();
    let session = OpenAIConversationsSession::new("conversation_123");
    let runner = Runner::new().with_model_provider(Arc::new(OpenAISessionProvider::default()));

    println!("=== OpenAI Conversation Session Example ===");
    println!("conversation_id={}", session.conversation_id().await);

    let first = runner
        .run_with_session(&agent, "What city is the Golden Gate Bridge in?", &session)
        .await?;
    println!("assistant_1={}", first.final_output.unwrap_or_default());
    println!(
        "last_response_id={}",
        session.last_response_id().await.unwrap_or_default()
    );

    let second = runner
        .run_with_session(&agent, "What state is it in?", &session)
        .await?;
    println!("assistant_2={}", second.final_output.unwrap_or_default());
    println!(
        "last_response_id={}",
        session.last_response_id().await.unwrap_or_default()
    );

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
