use openai_agents::{Agent, AgentsError, Usage, function_tool, run};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize, JsonSchema)]
struct WeatherArgs {
    city: String,
}

fn print_usage(usage: Usage) {
    println!("input_tokens={}", usage.input_tokens);
    println!("output_tokens={}", usage.output_tokens);
    println!("total_tokens={}", usage.input_tokens + usage.output_tokens);
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let get_weather = function_tool(
        "get_weather",
        "Get the current weather information for a city.",
        |_ctx, args: WeatherArgs| async move {
            Ok::<_, AgentsError>(json!({
                "city": args.city,
                "temperature_range": "14-20C",
                "conditions": "Sunny with wind"
            }))
        },
    )?;

    let agent = Agent::builder("usage_demo")
        .instructions("You are a concise assistant. Use tools when useful.")
        .function_tool(get_weather)
        .build();

    let result = run(&agent, "What's the weather in Tokyo?").await?;

    println!("{}", result.final_output.unwrap_or_default());
    print_usage(result.usage);
    Ok(())
}
