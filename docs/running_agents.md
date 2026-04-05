# Running Agents

Use this page when you want to choose the right execution entry point and understand how the runtime moves through turns.

## The Three Main Shapes

| Shape | Use it when | Entry point |
| --- | --- | --- |
| one-shot async | your app is already async | `run` |
| reusable configured runner | you want shared config and sessions | `Runner` |
| live event stream | you need events before the run completes | `run_streamed` |

## One-Shot Run

```rust
use openai_agents::{Agent, run};

# async fn demo() -> Result<(), openai_agents::AgentsError> {
let agent = Agent::builder("assistant").build();
let result = run(&agent, "hello").await?;
assert_eq!(result.agent_name, "assistant");
assert_eq!(result.final_output.as_deref(), Some("hello"));
# Ok(())
# }
```

## Reusable Runner

```rust
use openai_agents::{Agent, RunConfig, Runner};

# async fn demo() -> Result<(), openai_agents::AgentsError> {
let agent = Agent::builder("assistant").build();
let runner = Runner::new().with_config(RunConfig {
    max_turns: 5,
    ..RunConfig::default()
});

let result = runner.run(&agent, "hello").await?;
assert_eq!(result.agent_name, "assistant");
# Ok(())
# }
```

## Streamed Run

```rust,no_run
use futures::StreamExt;
use openai_agents::{Agent, run_streamed};

#[tokio::main]
async fn main() -> Result<(), openai_agents::AgentsError> {
    let agent = Agent::builder("assistant").build();
    let streamed = run_streamed(&agent, "hello").await?;

    while let Some(event) = streamed.stream_events().next().await {
        println!("{event:?}");
    }

    let result = streamed.wait_for_completion().await?;
    println!("{:?}", result.final_output);
    Ok(())
}
```

## What The Runner Owns

The runner is the execution coordinator. It is the place where:

- model provider selection happens
- sessions are loaded and persisted
- guardrails run
- tools are invoked
- handoffs occur
- traces and usage are recorded

## What A Run Produces

Every completed run gives you a `RunResult`. Every streamed run gives you a `RunResultStreaming`, which can produce a final `RunResult`.

Read [results.md](results.md) to understand replay and continuation.

## Resume And Continuation

Continuation data lives in the result state and session state, not in ad-hoc application bookkeeping.

Use the built-in result/session machinery when you want:

- conversation continuity
- replay input generation
- persisted `previous_response_id`
- persisted `conversation_id`

## Read Next

- [config.md](config.md)
- [streaming.md](streaming.md)
- [results.md](results.md)
- [sessions/README.md](sessions/README.md)
