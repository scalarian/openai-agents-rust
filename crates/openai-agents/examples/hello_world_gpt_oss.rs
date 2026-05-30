use std::env;
use std::sync::Arc;

use openai_agents::{
    Agent, AgentsError, Model, ModelProvider, ModelRequest, OpenAIChatCompletionsModel,
    OpenAIClientOptions, Runner, set_tracing_disabled,
};

const DEFAULT_MODEL: &str = "gpt-oss:20b";
const DEFAULT_BASE_URL: &str = "http://localhost:11434/v1";

#[derive(Clone)]
struct DirectModelProvider {
    model: Arc<OpenAIChatCompletionsModel>,
}

impl ModelProvider for DirectModelProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }

    fn prepare_request(&self, mut request: ModelRequest) -> ModelRequest {
        request
            .model
            .get_or_insert_with(|| DEFAULT_MODEL.to_owned());
        request
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    set_tracing_disabled(true);

    if env::var("RUN_GPT_OSS_LOCAL").ok().as_deref() != Some("1") {
        println!("Skipping run because RUN_GPT_OSS_LOCAL=1 was not provided.");
        return Ok(());
    }

    let model_name = env::var("GPT_OSS_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_owned());
    let base_url = env::var("GPT_OSS_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_owned());
    let api_key = env::var("GPT_OSS_API_KEY").unwrap_or_else(|_| "ollama".to_owned());
    let model = OpenAIChatCompletionsModel::new(
        model_name.clone(),
        OpenAIClientOptions::new(Some(api_key)).with_base_url(base_url),
    );
    let provider = DirectModelProvider {
        model: Arc::new(model),
    };

    let agent = Agent::builder("Assistant")
        .instructions(
            "You're a helpful assistant. You provide a concise answer to the user's question.",
        )
        .model(model_name)
        .build();

    let result = Runner::new()
        .with_model_provider(Arc::new(provider))
        .run(&agent, "Tell me about recursion in programming.")
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
