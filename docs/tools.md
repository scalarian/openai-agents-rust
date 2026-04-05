# Tools

Use this page when you want the model to trigger typed behavior instead of only returning text.

## The Main Tool Families

| Tool family | When to use it |
| --- | --- |
| function tools | normal Rust functions with typed arguments |
| shell and computer tools | local or hosted execution environments |
| hosted OpenAI tools | code interpreter, file search, web search, image generation |
| MCP tools | external tools discovered from MCP servers |

## Function Tool Example

```rust,no_run
use schemars::JsonSchema;
use serde::Deserialize;
use openai_agents::{Agent, AgentsError, function_tool};

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchArgs {
    query: String,
}

let search = function_tool(
    "search",
    "Search internal docs",
    |_ctx, args: SearchArgs| async move {
        Ok::<_, AgentsError>(format!("search-result: {}", args.query))
    },
)?;

let agent = Agent::builder("assistant")
    .instructions("Use tools when helpful.")
    .function_tool(search)
    .build();
# Ok::<(), AgentsError>(())
```

## Tool Runtime Features

The runtime supports more than “call this function”:

- input guardrails
- output guardrails
- approval requirements
- timeout settings
- defer-loading behavior
- tool namespaces and identity
- error formatting

## A Good Tool Design Rule

Keep tools small and explicit. If a tool needs a whole page of hidden logic to be safe, split it into several tools or move the control flow into your application.

## Local Shell And Computer Tools

The core runtime exposes shell and computer abstractions for controlled execution. Use them when:

- the model needs bounded local actions
- you need approval-aware execution
- you want structured call and result objects instead of hand-rolled subprocess prompts

## Hosted And MCP Tools

- OpenAI-hosted tools live in the facade re-exports and the OpenAI crate
- MCP tools are discovered dynamically from configured servers

Read [mcp.md](mcp.md) for MCP-specific lifecycle behavior.

## Read Next

- [guardrails.md](guardrails.md)
- [human_in_the_loop.md](human_in_the_loop.md)
- [mcp.md](mcp.md)
- [ref/runtime.md](ref/runtime.md)
