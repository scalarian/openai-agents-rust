# Context

Use this page when your runs or tools need application state such as user identity, tenancy, feature flags, or service handles.

## Why Context Exists

The runtime separates:

- the agent definition
- the execution configuration
- the per-call context

That lets you reuse the same agent safely across requests while still passing request-local state into tools, hooks, and runtime callbacks.

## The Main Types

- `RunContext`
- `RunContextWrapper`
- `ToolContext`
- `RunOptions<TContext>`

## The Pattern

```rust,no_run
use openai_agents::{RunOptions, Runner};

#[derive(Clone)]
struct AppContext {
    user_id: String,
}

let options = RunOptions {
    context: Some(AppContext {
        user_id: "user_123".to_owned(),
    }),
    ..RunOptions::default()
};

let _runner = Runner::new();
```

## Where Context Is Visible

- tool handlers
- run hooks
- tracing helpers
- handoff and approval flows that depend on caller state

## What Not To Put In Context

- durable conversational history
- replay state
- conversation identifiers
- anything already modeled by sessions or results

That data belongs in sessions or in the run result, not in your app context object.

## Read Next

- [tools.md](tools.md)
- [human_in_the_loop.md](human_in_the_loop.md)
- [results.md](results.md)
