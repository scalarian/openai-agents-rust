# Realtime

Use this section when you need a long-lived session that can send and receive incremental events instead of waiting for one final result.

## Main Types

- `RealtimeRunner`
- `RealtimeSession`
- `RealtimeAgent`
- `RealtimeEvent`

## Minimal Example

```rust
use openai_agents::realtime::{RealtimeAgent, RealtimeRunner};

#[tokio::main]
async fn main() -> Result<(), openai_agents::AgentsError> {
    let runner = RealtimeRunner::new(RealtimeAgent::new("assistant"));
    let session = runner.run().await?;

    let events = session.send_text("hello from realtime").await?;
    for event in events {
        println!("{event:?}");
    }

    session.close().await?;
    Ok(())
}
```

Runnable version: [realtime_session.rs](../../crates/openai-agents/examples/realtime_session.rs)

## In This Section

- [events.md](events.md)
- [audio.md](audio.md)
- [transports.md](transports.md)

## Read Next

- [events.md](events.md)
- [audio.md](audio.md)
- [../voice/README.md](../voice/README.md)
