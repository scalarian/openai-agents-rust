use std::env;
use std::sync::Arc;

use openai_agents::extensions::{AnyLLMApi, AnyLLMProvider};
use openai_agents::{Agent, AgentsError, Runner, function_tool, set_tracing_disabled};
use schemars::JsonSchema;
use serde::Deserialize;

const DEFAULT_MODEL: &str = "openrouter/openai/gpt-5.4-mini";
const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";

#[derive(Debug, Deserialize, JsonSchema)]
struct WeatherArgs {
    city: String,
}

fn arg_value(flag: &str) -> Option<String> {
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == flag {
            return args.next();
        }

        if let Some(value) = arg.strip_prefix(&format!("{flag}=")) {
            return Some(value.to_owned());
        }
    }
    None
}

fn model_for_base_url<'a>(model: &'a str, base_url: &str) -> &'a str {
    if base_url.contains("openrouter.ai") {
        model.strip_prefix("openrouter/").unwrap_or(model)
    } else {
        model
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    set_tracing_disabled(true);

    let model = arg_value("--model")
        .or_else(|| env::var("ANY_LLM_MODEL").ok())
        .unwrap_or_else(|| DEFAULT_MODEL.to_owned());
    let api_key = arg_value("--api-key")
        .or_else(|| env::var("OPENROUTER_API_KEY").ok())
        .unwrap_or_else(|| "dummy".to_owned());
    let base_url = arg_value("--base-url")
        .or_else(|| env::var("OPENROUTER_BASE_URL").ok())
        .unwrap_or_else(|| OPENROUTER_BASE_URL.to_owned());

    println!("Using default model: {model}");
    if api_key == "dummy" {
        println!("Skipping run because no valid OPENROUTER_API_KEY was provided.");
        return Ok(());
    }

    let get_weather = function_tool(
        "get_weather",
        "Get the weather for a city.",
        |_ctx, args: WeatherArgs| async move {
            println!("[debug] getting weather for {}", args.city);
            Ok::<_, AgentsError>(format!("The weather in {} is sunny.", args.city))
        },
    )?;

    let agent = Agent::builder("Assistant")
        .instructions("You only respond in haikus.")
        .model(model_for_base_url(&model, &base_url))
        .function_tool(get_weather)
        .build();

    let provider = AnyLLMProvider::new()
        .with_api_key(api_key)
        .with_base_url(base_url)
        .with_api(AnyLLMApi::ChatCompletions);
    let runner = Runner::new().with_model_provider(Arc::new(provider));
    let result = runner.run(&agent, "What's the weather in Tokyo?").await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
