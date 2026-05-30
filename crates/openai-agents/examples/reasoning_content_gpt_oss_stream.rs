use std::env;
use std::sync::Arc;

use futures::StreamExt;
use openai_agents::{
    Agent, AgentsError, Model, ModelProvider, ModelRequest, ModelSettings,
    OpenAIChatCompletionsModel, OpenAIClientOptions, OutputItem, ReasoningSettings, RunItem,
    Runner, StreamEvent, set_tracing_disabled,
};

const DEFAULT_MODEL: &str = "openai/gpt-oss-20b";
const DEFAULT_BASE_URL: &str = "https://openrouter.ai/api/v1";

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

    if env::var("RUN_GPT_OSS_REASONING_STREAM").ok().as_deref() != Some("1") {
        println!("Skipping run because RUN_GPT_OSS_REASONING_STREAM=1 was not provided.");
        return Ok(());
    }

    let api_key = env::var("OPENROUTER_API_KEY")
        .or_else(|_| env::var("GPT_OSS_API_KEY"))
        .unwrap_or_else(|_| "dummy".to_owned());
    if api_key == "dummy" {
        println!("Skipping run because OPENROUTER_API_KEY or GPT_OSS_API_KEY is not set.");
        return Ok(());
    }

    let model_name = env::var("GPT_OSS_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_owned());
    let base_url = env::var("GPT_OSS_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_owned());
    let model = OpenAIChatCompletionsModel::new(
        model_name.clone(),
        OpenAIClientOptions::new(Some(api_key)).with_base_url(base_url),
    );
    let provider = DirectModelProvider {
        model: Arc::new(model),
    };

    let agent = Agent::builder("Assistant")
        .instructions("Provide a concise answer and include available reasoning content.")
        .model(model_name)
        .model_settings(ModelSettings {
            reasoning: Some(ReasoningSettings {
                effort: Some("high".to_owned()),
                summary: Some("detailed".to_owned()),
            }),
            ..ModelSettings::default()
        })
        .build();

    let result = Runner::new()
        .with_model_provider(Arc::new(provider))
        .run_streamed(&agent, "Tell me about recursion in programming.")
        .await?;

    println!("=== Run starting ===");
    let mut events = result.stream_events();
    while let Some(event) = events.next().await {
        match event {
            StreamEvent::RunItemEvent(event) => match event.item {
                RunItem::Reasoning { text } => println!("reasoning_delta={text}"),
                RunItem::MessageOutput {
                    content: OutputItem::Text { text },
                } => println!("output_delta={text}"),
                _ => {}
            },
            StreamEvent::RawResponseEvent(raw) => {
                if raw.type_name.contains("reasoning") || raw.type_name.contains("output_text") {
                    println!("raw_event={}", raw.type_name);
                }
            }
            StreamEvent::AgentUpdated(_) | StreamEvent::Lifecycle(_) => {}
        }
    }
    println!("=== Run complete ===");
    Ok(())
}
