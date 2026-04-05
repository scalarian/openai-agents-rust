# Agents

Use this page when you want to understand what an `Agent` is, how to build one, and which pieces belong on the agent versus on the runner.

## Smallest Agent

```rust
use openai_agents::Agent;

let agent = Agent::builder("assistant")
    .instructions("Be concise, practical, and structured.")
    .build();
```

An agent is the runtime definition of:

- its name and instructions
- its tools
- its output schema
- its handoffs
- its hooks and optional runtime helpers

## What Usually Belongs On The Agent

| Put it on the agent when... | Examples |
| --- | --- |
| it defines stable behavior for that role | instructions, tools, handoffs |
| it should travel with the agent anywhere it runs | output schema, hooks |
| it is part of the agent identity | handoff description, tool set |

## What Usually Belongs On The Runner

| Put it on the runner or run config when... | Examples |
| --- | --- |
| it is request-specific | `max_turns`, `conversation_id`, `trace_metadata` |
| it depends on the caller environment | model provider, sessions, tracing processors |
| it changes from run to run | run hooks, session callbacks, runtime overrides |

## Common Agent Builder Features

- `instructions(...)`
- `function_tool(...)`
- `handoff(...)`
- `output_schema(...)`
- `hooks(...)`

## Structured Output

If you need a strict shape back from the model, attach an output schema to the agent instead of parsing untrusted text after the run.

Use structured outputs when:

- the result feeds another service
- you need stable keys and types
- you want retries or max-turn logic to stay in the runtime

## Agents As Tools

An agent can be wrapped as a tool and invoked from another agent. This is the main primitive for “router + specialist” setups.

Use `Agent::as_tool()` when:

- the specialist should keep its own instructions and state
- you want nested runs to show up as tool executions
- the parent should decide when to delegate

Read [multi_agent.md](multi_agent.md) and [handoffs.md](handoffs.md) before building large delegating systems.

## A Practical Rule

Start with one agent and one runner. Add extra agents only when a role needs genuinely different instructions, output constraints, or tools.

## Read Next

- [running_agents.md](running_agents.md)
- [tools.md](tools.md)
- [multi_agent.md](multi_agent.md)
- [ref/runtime.md](ref/runtime.md)
