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
- `dynamic_instructions(...)`
- `prompt(...)`
- `dynamic_prompt(...)`
- `model_settings(...)`
- `function_tool(...)`
- `handoff(...)`
- `output_schema(...)`
- `reset_tool_choice(...)`
- `hooks(...)`

Use `dynamic_instructions(...)` when the system prompt should be derived from the run context or another runtime value instead of being fixed on the agent.
Use `prompt(...)` or `dynamic_prompt(...)` for reusable OpenAI Responses prompt configs.

## Structured Output

If you need a strict shape back from the model, attach an output schema to the agent instead of parsing untrusted text after the run.

Use structured outputs when:

- the result feeds another service
- you need stable keys and types
- you want retries or max-turn logic to stay in the runtime

Use `AgentOutputSchema<T>` or `OutputSchemaDefinition` directly when you need non-strict schema handling or a custom validation pass. See [non_strict_output_type.rs](../crates/openai-agents/examples/non_strict_output_type.rs).

For file input content, pass raw JSON `InputItem::Json` messages with `content` entries such as `{"type":"input_file","file_data":"data:application/pdf;base64,...","filename":"..."}` or `{"type":"input_file","file_url":"https://..."}`. See [local_file.rs](../crates/openai-agents/examples/local_file.rs) and [remote_pdf.rs](../crates/openai-agents/examples/remote_pdf.rs).

For image input content, use `{"type":"input_image","detail":"auto","image_url":"data:image/...;base64,..."}` for local images or an HTTPS image URL for remote images. See [local_image.rs](../crates/openai-agents/examples/local_image.rs) and [remote_image.rs](../crates/openai-agents/examples/remote_image.rs).

## Forcing Tool Use

Use `ModelSettings { tool_choice: Some(...) }` when the model should be forced to use a tool.
Valid choices include `auto`, `required`, `none`, or a specific callable tool name.
When using OpenAI Responses tool search, named choices cannot target a bare namespace name, a deferred-only function tool, or the hosted `tool_search_tool()` itself. Prefer `auto` or `required` for that flow; see [Hosted Tool Search](tools.md#hosted-tool-search).

```rust,no_run
use openai_agents::{Agent, AgentsError, ModelSettings, function_tool};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
struct WeatherArgs {
    city: String,
}

# fn build_agent() -> Result<Agent, AgentsError> {
let weather = function_tool(
    "get_weather",
    "Return weather for a city.",
    |_ctx, args: WeatherArgs| async move {
        Ok::<_, AgentsError>(format!("The weather in {} is sunny.", args.city))
    },
)?;

let agent = Agent::builder("weather")
    .function_tool(weather)
    .model_settings(ModelSettings {
        tool_choice: Some("get_weather".to_owned()),
        ..ModelSettings::default()
    })
    .build();
# Ok(agent)
# }
```

By default, the runner clears forced `tool_choice` after that agent uses a function, custom, or handoff tool. This matches the upstream loop-prevention behavior: tool results are sent back to the model, and leaving `tool_choice` forced can make the model call tools forever.

Use `.reset_tool_choice(false)` only when the agent must preserve a forced tool choice across tool-result turns.

## Agents As Tools

An agent can be wrapped as a tool and invoked from another agent. This is the main primitive for “router + specialist” setups.

Use `Agent::as_tool()` when:

- the specialist should keep its own instructions and state
- you want nested runs to show up as tool executions
- the parent should decide when to delegate

Use `Agent::as_tool::<TArgs>(...)` when the parent should pass typed structured arguments to the nested agent. See [agents_as_tools_structured.rs](../crates/openai-agents/examples/agents_as_tools_structured.rs).
Use `AgentAsToolOptions::is_enabled` when agent tools should be visible only for some run contexts. See [agents_as_tools_conditional.rs](../crates/openai-agents/examples/agents_as_tools_conditional.rs).

Read [multi_agent.md](multi_agent.md) and [handoffs.md](handoffs.md) before building large delegating systems.

## A Practical Rule

Start with one agent and one runner. Add extra agents only when a role needs genuinely different instructions, output constraints, or tools.

## Read Next

- [running_agents.md](running_agents.md)
- [tools.md](tools.md)
- [multi_agent.md](multi_agent.md)
- [ref/runtime.md](ref/runtime.md)
