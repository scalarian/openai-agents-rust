use openai_agents::AgentsError;
use openai_agents::realtime::{RealtimeAgent, RealtimeRunner};

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let runner = RealtimeRunner::new(RealtimeAgent::new("assistant"));
    let session = runner.run().await?;

    let events = session.send_text("hello from realtime").await?;
    for event in events {
        println!("{event:?}");
    }

    println!("transcript={}", session.transcript().await);
    session.close().await?;
    Ok(())
}
