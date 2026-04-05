use futures::StreamExt;
use openai_agents::{Agent, AgentsError, run_streamed};

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("assistant")
        .instructions("Be concise, practical, and structured.")
        .build();

    let streamed = run_streamed(&agent, "Stream a release checklist.").await?;
    let events = streamed.stream_events().collect::<Vec<_>>().await;
    let result = streamed.wait_for_completion().await?;

    println!("events={}", events.len());
    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
