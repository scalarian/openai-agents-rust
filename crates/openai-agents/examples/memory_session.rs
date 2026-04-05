use openai_agents::{Agent, AgentsError, MemorySession, Runner};

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("assistant")
        .instructions("Track the conversation and answer briefly.")
        .build();

    let session = MemorySession::new("demo");
    let runner = Runner::new();

    runner
        .run_with_session(&agent, "My name is Ada.", &session)
        .await?;
    let result = runner
        .run_with_session(&agent, "What is my name?", &session)
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
