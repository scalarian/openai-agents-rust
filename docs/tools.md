# Tools

Use this page when you want the model to trigger typed behavior instead of only returning text.

## The Main Tool Families

| Tool family | When to use it |
| --- | --- |
| function tools | normal Rust functions with typed arguments |
| custom tools | raw string inputs for Responses custom-tool calls |
| shell and computer tools | local or hosted execution environments |
| hosted OpenAI tools | code interpreter, file search, web search, image generation |
| MCP tools | external tools discovered from MCP servers |

Function tools can return text, JSON, image, or file outputs. See [image_tool_output.rs](../crates/openai-agents/examples/image_tool_output.rs) for an image-returning function tool.

Custom tools use one raw string input instead of JSON arguments. Use `custom_tool(...)` when the model should send free-form text such as patches, document edits, or mini-language commands. See [custom_tool.rs](../crates/openai-agents/examples/custom_tool.rs) for a deterministic raw-input example. Custom tools also support approval interruptions and `with_on_approval(...)` callbacks for automatic approval or rejection.

Hosted tools are added as static tools on the agent. For example, [code_interpreter.rs](../crates/openai-agents/examples/code_interpreter.rs) configures an auto code interpreter container, [file_search.rs](../crates/openai-agents/examples/file_search.rs) configures vector store search with included results, [image_generator.rs](../crates/openai-agents/examples/image_generator.rs) decodes an image generation result, [tool_search.rs](../crates/openai-agents/examples/tool_search.rs) shows deferred namespace loading, [web_search.rs](../crates/openai-agents/examples/web_search.rs) configures `web_search_tool_with_options(...)` with an approximate user location, and [web_search_filters.rs](../crates/openai-agents/examples/web_search_filters.rs) shows domain filters plus source includes.

## Hosted Tool Search

Tool search lets OpenAI Responses models defer large tool surfaces until runtime, so the model loads only the subset it needs for the current turn. Use it when you have many function tools or namespace groups and want to reduce tool-schema tokens without exposing every function up front.

Start with hosted tool search when the candidate tools are already known as agent tools. If an application needs to decide what to load dynamically, the Responses API also supports client-executed tool search, but the standard `Runner` does not auto-execute that mode.

```rust,no_run
use openai_agents::{Agent, AgentsError, function_tool, tool_search_tool};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
struct TrackingArgs {
    tracking_number: String,
}

# fn shipping_eta_tool() -> Result<openai_agents::FunctionTool, AgentsError> {
function_tool(
    "get_shipping_eta",
    "Look up a shipment ETA by tracking number.",
    |_ctx, args: TrackingArgs| async move {
        Ok::<_, AgentsError>(format!("ETA for {}", args.tracking_number))
    },
)
# }
#
# fn build_agent() -> Result<Agent, AgentsError> {
let mut shipping_eta = shipping_eta_tool()?.with_defer_loading(true);
shipping_eta.definition.namespace = Some("shipping".to_owned());

let agent = Agent::builder("shipping")
    .model("gpt-5.5")
    .instructions("Load the shipping namespace before using shipping tools.")
    .function_tool(shipping_eta)
    .tool(tool_search_tool())
    .build();
# Ok(agent)
# }
```

What to know:

- Hosted tool search is available only with OpenAI Responses models.
- Add exactly one `tool_search_tool()` when you configure deferred-loading function tools on an agent.
- Use `FunctionTool::with_defer_loading(true)` for deferred top-level tools.
- Set `tool.definition.namespace = Some("name".to_owned())` to group related function tools under a namespace. Namespaces are usually a better search surface than many individually deferred functions.
- Namespaces can mix immediate and deferred tools. Tools without `defer_loading` remain callable immediately, while deferred tools in the same namespace are loaded through tool search.
- Named `tool_choice` cannot target bare namespace names, deferred-only tools, or the hosted `tool_search_tool()` itself. Prefer `auto`, `required`, or a real top-level callable tool name.
- Tool search activity appears in [RunResult.new_items](results.md#what-runresult-carries) and in [RunItemStreamEvent](streaming.md#event-families) with dedicated item and event types.
- See [tool_search.rs](../crates/openai-agents/examples/tool_search.rs) for a complete runnable example covering both namespaced loading and top-level deferred tools.

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
