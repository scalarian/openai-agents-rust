# Streaming

Use this page when you need visibility into the run before the final output is ready.

## What You Get

`RunResultStreaming` is a live run handle. It lets you:

- read events as they happen
- wait for the final `RunResult`
- inspect last-response and replay state
- preserve normalized and full replay input

## Minimal Example

```rust,no_run
use futures::StreamExt;
use openai_agents::{Agent, run_streamed};

#[tokio::main]
async fn main() -> Result<(), openai_agents::AgentsError> {
    let agent = Agent::builder("assistant").build();
    let streamed = run_streamed(&agent, "hello").await?;

    let events = streamed.stream_events().collect::<Vec<_>>().await;
    let result = streamed.wait_for_completion().await?;

    println!("events={}", events.len());
    println!("{:?}", result.final_output);
    Ok(())
}
```

Runnable version: [streamed_run.rs](../crates/openai-agents/examples/streamed_run.rs)

## Event Families

The stream can include:

- agent lifecycle events
- raw model events
- run item events
- tool lifecycle events
- handoff events
- interruption events

## Replay Matters

The streamed handle is not just an event feed. It also preserves the state needed to continue or replay the run after completion.

Use:

- `wait_for_completion()` for the durable result
- `to_input_list()` or `to_input_list_mode(...)` for replay

## Read Next

- [results.md](results.md)
- [running_agents.md](running_agents.md)
- [tracing.md](tracing.md)
