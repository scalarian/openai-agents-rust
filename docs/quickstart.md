# Quickstart

Use this page when you want the fastest path from an empty Rust project to a working agent run.

## Install

Add the facade crate and Tokio:

```toml
[dependencies]
openai-agents = { git = "https://github.com/scalarian/openai-agents-rust", package = "openai-agents" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## First Run

```rust
use openai_agents::{run, Agent};

#[tokio::main]
async fn main() -> Result<(), openai_agents::AgentsError> {
    let agent = Agent::builder("assistant")
        .instructions("Be concise, practical, and structured.")
        .build();

    let result = run(&agent, "Give me three release checks.").await?;
    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
```

Runnable version: [basic_run.rs](../crates/openai-agents/examples/basic_run.rs)

## When To Use Which Entry Point

| Need | Entry point |
| --- | --- |
| one-shot async call | `run` |
| one-shot sync call | `run_sync` |
| reusable configured runner | `Runner` |
| session-backed conversations | `Runner::run_with_session` |
| live streamed events | `run_streamed` or `Runner::run_streamed` |

## Add A Session

```rust
use openai_agents::{Agent, MemorySession, Runner};

#[tokio::main]
async fn main() -> Result<(), openai_agents::AgentsError> {
    let agent = Agent::builder("assistant")
        .instructions("Track the conversation and answer briefly.")
        .build();

    let session = MemorySession::new("demo");
    let runner = Runner::new();

    runner.run_with_session(&agent, "My name is Ada.", &session).await?;
    let result = runner
        .run_with_session(&agent, "What is my name?", &session)
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
```

Runnable version: [memory_session.rs](../crates/openai-agents/examples/memory_session.rs)

## Add Streaming

```rust,no_run
use futures::StreamExt;
use openai_agents::{Agent, run_streamed};

#[tokio::main]
async fn main() -> Result<(), openai_agents::AgentsError> {
    let agent = Agent::builder("assistant")
        .instructions("Be concise.")
        .build();

    let streamed = run_streamed(&agent, "Stream a deployment checklist.").await?;
    let events = streamed.stream_events().collect::<Vec<_>>().await;
    let result = streamed.wait_for_completion().await?;

    println!("events={}", events.len());
    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
```

Runnable version: [streamed_run.rs](../crates/openai-agents/examples/streamed_run.rs)

## What You Should Read Next

- [agents.md](agents.md): how to shape an agent beyond name and instructions
- [running_agents.md](running_agents.md): how runners, options, and resume paths work
- [tools.md](tools.md): how to attach runtime behavior
- [sessions/README.md](sessions/README.md): how state and continuation work
