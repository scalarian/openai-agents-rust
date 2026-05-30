use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, RunOptions, Runner, Usage,
};

#[derive(Clone, Default)]
struct PreviousResponseModel;

#[async_trait]
impl Model for PreviousResponseModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let prompt = request
            .input
            .iter()
            .filter_map(|item| item.as_text())
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();

        let previous_response_id = request.previous_response_id.clone();
        let (text, response_id) = if prompt.contains("capital")
            && previous_response_id.as_deref() == Some("resp-brazil")
        {
            (
                "Brasilia (continued from resp-brazil)".to_owned(),
                "resp-brasilia".to_owned(),
            )
        } else {
            ("Brazil".to_owned(), "resp-brazil".to_owned())
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text { text }],
            usage: Usage {
                input_tokens: 4,
                output_tokens: 2,
            },
            response_id: Some(response_id),
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct PreviousResponseProvider {
    model: Arc<PreviousResponseModel>,
}

impl ModelProvider for PreviousResponseProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("Assistant")
        .instructions("Answer in one short sentence.")
        .build();
    let runner = Runner::new().with_model_provider(Arc::new(PreviousResponseProvider::default()));

    let first = runner
        .run(&agent, "What is the largest country in South America?")
        .await?;
    println!("{}", first.final_output_text().unwrap_or_default());
    println!(
        "first_response_id={}",
        first.last_response_id().unwrap_or("none")
    );

    let second = runner
        .run_with_options(
            &agent,
            vec!["What is the capital of that country?".into()],
            RunOptions {
                previous_response_id: first.last_response_id().map(str::to_owned),
                ..RunOptions::default()
            },
        )
        .await?;

    println!("{}", second.final_output_text().unwrap_or_default());
    println!(
        "second_response_id={}",
        second.last_response_id().unwrap_or("none")
    );

    Ok(())
}
