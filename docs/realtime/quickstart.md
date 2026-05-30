# Realtime Quickstart

Realtime agents keep a live session open so your application can send input and receive incremental events without waiting for one final text-only result.

## Minimal Session

The runnable version lives in [realtime_session.rs](../../crates/openai-agents/examples/realtime_session.rs).

```rust,no_run
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

## Add Tools And Handoffs

`RealtimeAgent` is the realtime-specific agent surface. Use it for realtime instructions, tools, and handoffs; use `RealtimeRunner` to create the live session.

```rust,no_run
use openai_agents::realtime::{RealtimeAgent, RealtimeRunner};

#[tokio::main]
async fn main() -> Result<(), openai_agents::AgentsError> {
    let agent = RealtimeAgent::new("support")
        .with_instructions("Keep replies short and ask one question at a time.");
    let session = RealtimeRunner::new(agent).run().await?;

    session.send_text("I need help with my reservation").await?;
    session.close().await?;
    Ok(())
}
```

For a larger agent configuration, see [realtime_app_agent.rs](../../crates/openai-agents/examples/realtime_app_agent.rs).

## What To Read Next

- [README.md](README.md): section overview and minimal type map
- [guide.md](guide.md): lifecycle, events, tools, and session state
- [transport.md](transport.md): WebSocket, SIP, Twilio, and Cloudflare transport choices
- [events.md](events.md): event families emitted by a session
- [audio.md](audio.md): audio formats and playback state
