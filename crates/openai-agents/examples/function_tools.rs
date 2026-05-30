use openai_agents::{Agent, AgentsError, function_tool, run};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize, JsonSchema)]
struct WeatherArgs {
    city: String,
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let get_weather = function_tool(
        "get_weather",
        "Get the current weather information for a city.",
        |_ctx, args: WeatherArgs| async move {
            println!("[debug] get_weather called for {}", args.city);
            Ok::<_, AgentsError>(json!({
                "city": args.city,
                "temperature_range": "14-20C",
                "conditions": "Sunny with wind"
            }))
        },
    )?;

    let agent = Agent::builder("Hello world")
        .instructions("You are a helpful agent. Use tools when they answer the question.")
        .function_tool(get_weather)
        .build();

    let result = run(&agent, "What's the weather in Tokyo?").await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
