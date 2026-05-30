use std::env;
use std::sync::Arc;

use openai_agents::extensions::{AnyLLMApi, AnyLLMProvider};
use openai_agents::{
    Agent, AgentOutputSchema, AgentsError, Model, ModelProvider, ModelRequest, ModelSettings,
    MultiProvider, MultiProviderMap, OpenAIProvider, OutputSchemaDefinition, Runner, function_tool,
    set_tracing_disabled,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const MODEL: &str = "any-llm/openrouter/openai/gpt-5.4-mini";
const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";

#[derive(Debug, Deserialize, JsonSchema)]
struct WeatherArgs {
    city: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct ResultOutput {
    output_text: String,
    tool_results: Vec<String>,
}

fn openrouter_model_id(model: &str) -> &str {
    model.strip_prefix("openrouter/").unwrap_or(model)
}

#[derive(Clone)]
struct OpenRouterAnyLLMProvider {
    inner: AnyLLMProvider,
}

impl ModelProvider for OpenRouterAnyLLMProvider {
    fn resolve(&self, model: Option<&str>) -> Arc<dyn Model> {
        self.inner.resolve(model.map(openrouter_model_id))
    }

    fn prepare_request(&self, mut request: ModelRequest) -> ModelRequest {
        request.model = request
            .model
            .map(|model| openrouter_model_id(&model).to_owned());
        self.inner.prepare_request(request)
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    set_tracing_disabled(true);

    let api_key = env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| "dummy".to_owned());
    if api_key == "dummy" {
        println!("Skipping run because OPENROUTER_API_KEY is not set.");
        return Ok(());
    }

    let base_url =
        env::var("OPENROUTER_BASE_URL").unwrap_or_else(|_| OPENROUTER_BASE_URL.to_owned());
    let get_weather = function_tool(
        "get_weather",
        "Get the weather for a city.",
        |_ctx, args: WeatherArgs| async move {
            println!("[debug] getting weather for {}", args.city);
            Ok::<_, AgentsError>(format!("The weather in {} is sunny.", args.city))
        },
    )?;

    let output_schema = AgentOutputSchema::<ResultOutput>::new(true);
    let agent = Agent::builder("Assistant")
        .instructions("You only respond in haikus.")
        .model(MODEL)
        .model_settings(ModelSettings {
            tool_choice: Some("required".to_owned()),
            ..ModelSettings::default()
        })
        .output_schema(OutputSchemaDefinition::from_agent_output_schema(
            "Result",
            &output_schema,
        )?)
        .function_tool(get_weather)
        .build();

    let mut provider_map = MultiProviderMap::default();
    provider_map.add_provider(
        "any-llm",
        Arc::new(OpenRouterAnyLLMProvider {
            inner: AnyLLMProvider::new()
                .with_api_key(api_key)
                .with_base_url(base_url)
                .with_api(AnyLLMApi::ChatCompletions),
        }),
    );
    let provider =
        MultiProvider::new(Arc::new(OpenAIProvider::new())).with_provider_map(provider_map);
    let runner = Runner::new().with_model_provider(Arc::new(provider));
    let result = runner.run(&agent, "What's the weather in Tokyo?").await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
