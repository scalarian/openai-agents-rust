use openai_agents::{Agent, AgentsError, run};

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("assistant")
        .instructions("Be concise, practical, and structured.")
        .build();

    let result = run(&agent, "Give me three release checks.").await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
