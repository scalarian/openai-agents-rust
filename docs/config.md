# Configuration

Use this page when you need to understand what is configured on `RunConfig`, what is configured on `RunOptions`, and how overrides should be layered.

## The Short Version

- `RunConfig` describes the runtime behavior for a run
- `RunOptions` is the call-time wrapper that can carry context, sessions, hooks, and config overrides together
- `Runner` is where you usually keep defaults you want to reuse

## `RunConfig`

`RunConfig` carries the runtime knobs that shape behavior:

- model selection
- max turns
- tracing settings and trace metadata
- previous response and conversation identifiers
- model settings
- session settings
- handoff filters and history mapping
- input and output guardrails
- session input callbacks
- tool error formatting
- run hooks and error handlers

## `RunOptions`

`RunOptions` is the outer execution wrapper. Use it when you need to pass:

- `context`
- `hooks`
- `error_handlers`
- `run_config`
- `session`
- `previous_response_id`
- `conversation_id`
- `model_provider`

## Recommended Layering

| Layer | What belongs there |
| --- | --- |
| agent | stable role definition |
| runner defaults | shared application defaults |
| run config | request-specific runtime behavior |
| run options | concrete call-time wrapper state |

## Practical Precedence

The safe working assumption is:

1. start from runner defaults
2. apply agent-specific behavior
3. overlay the provided `RunConfig`
4. finish with `RunOptions` call-time state such as session, context, and explicit identifiers

## A Good Default Pattern

```rust,no_run
use openai_agents::{RunConfig, RunOptions, Runner};

let runner = Runner::new().with_config(RunConfig {
    max_turns: 8,
    ..RunConfig::default()
});

let _options = RunOptions {
    run_config: Some(RunConfig {
        workflow_name: "checkout-support".to_owned(),
        ..RunConfig::default()
    }),
    ..RunOptions::default()
};
```

## When Config Gets Too Large

If your `RunConfig` starts carrying many request-specific closures or callbacks, move that assembly into your own application builder instead of inlining it at the call site.

## Read Next

- [context.md](context.md)
- [sessions/README.md](sessions/README.md)
- [tracing.md](tracing.md)
- [ref/runtime.md](ref/runtime.md)
