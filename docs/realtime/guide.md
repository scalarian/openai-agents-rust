# Realtime Guide

Realtime sessions are long-lived runtime objects. A session can accept user text or audio, emit events, execute realtime tools, and move between realtime agents through handoffs.

## Runtime Pieces

| Piece | Role |
| --- | --- |
| `RealtimeAgent` | Realtime instructions, tools, and handoffs for one specialist. |
| `RealtimeRunner` | Factory that starts a live `RealtimeSession`. |
| `RealtimeSession` | Sends input, streams events, tracks active state, and closes the live run. |
| `RealtimeEvent` | Event values emitted by the session. |

## Lifecycle

1. Build a `RealtimeAgent`.
2. Create a `RealtimeRunner`.
3. Call `run().await` to get a `RealtimeSession`.
4. Send text or audio.
5. Consume returned events or subscribe to live session streams.
6. Close the session when the interaction ends.

```rust,no_run
use openai_agents::realtime::{RealtimeAgent, RealtimeRunner};

#[tokio::main]
async fn main() -> Result<(), openai_agents::AgentsError> {
    let agent = RealtimeAgent::new("assistant")
        .with_instructions("Answer in one or two short sentences.");
    let session = RealtimeRunner::new(agent).run().await?;

    for event in session.send_text("Summarize the session lifecycle").await? {
        println!("{event:?}");
    }

    session.close().await?;
    Ok(())
}
```

## Events And History

Use [events.md](events.md) when you need the event-family map. Treat realtime events as the application contract for transcripts, raw model events, tool lifecycle updates, handoffs, interruptions, and close events.

For examples that exercise the facade and semantic tests around event behavior, see:

- [realtime_session.rs](../../crates/openai-agents/examples/realtime_session.rs)
- [realtime_app_agent.rs](../../crates/openai-agents/examples/realtime_app_agent.rs)
- [realtime_twilio_media_stream.rs](../../crates/openai-agents/examples/realtime_twilio_media_stream.rs)
- [realtime_twilio_sip.rs](../../crates/openai-agents/examples/realtime_twilio_sip.rs)

## Audio

Use [audio.md](audio.md) for audio formats and playback tracking. Realtime audio flows should keep capture/playback concerns in the application boundary and use the runtime session for model-facing events, commands, and state.

## Transport

Use [transport.md](transport.md) to choose between the OpenAI realtime WebSocket path, SIP-oriented flows, and extension transports such as Twilio and Cloudflare adapters.

## Read Next

- [quickstart.md](quickstart.md)
- [transport.md](transport.md)
- [events.md](events.md)
- [audio.md](audio.md)
