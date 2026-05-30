# Examples

Runnable examples live in `crates/openai-agents/examples`.

## Run An Example

From the repo root:

```bash
cargo run -p openai-agents-rs --example agents_as_tools
cargo run -p openai-agents-rs --example agents_as_tools_conditional
cargo run -p openai-agents-rs --example agents_as_tools_streaming
cargo run -p openai-agents-rs --example agents_as_tools_structured
cargo run -p openai-agents-rs --example basic_run
cargo run -p openai-agents-rs --example custom_agent_model
cargo run -p openai-agents-rs --example custom_model_provider
cargo run -p openai-agents-rs --example default_model_provider
cargo run -p openai-agents-rs --example deterministic_flow
cargo run -p openai-agents-rs --example dynamic_system_prompt
cargo run -p openai-agents-rs --example forcing_tool_use
cargo run -p openai-agents-rs --example function_tools
cargo run -p openai-agents-rs --example handoffs
cargo run -p openai-agents-rs --example image_tool_output
cargo run -p openai-agents-rs --example input_guardrail
cargo run -p openai-agents-rs --example lifecycle_hooks
cargo run -p openai-agents-rs --example llm_as_a_judge
cargo run -p openai-agents-rs --example memory_session
cargo run -p openai-agents-rs --example non_strict_output_type
cargo run -p openai-agents-rs --example output_guardrail
cargo run -p openai-agents-rs --example parallelization
cargo run -p openai-agents-rs --example previous_response_id
cargo run -p openai-agents-rs --example prompt_template
cargo run -p openai-agents-rs --example prompt_template -- --dynamic
cargo run -p openai-agents-rs --example sandbox_workspace
cargo run -p openai-agents-rs --example sqlite_session
cargo run -p openai-agents-rs --example stream_function_call_args
cargo run -p openai-agents-rs --example stream_items
cargo run -p openai-agents-rs --example stream_text
cargo run -p openai-agents-rs --example streamed_run
cargo run -p openai-agents-rs --example tool_guardrails
cargo run -p openai-agents-rs --example usage_tracking
cargo run -p openai-agents-rs --example realtime_session
cargo run -p openai-agents-rs --example retry
cargo run -p openai-agents-rs --example routing
cargo run -p openai-agents-rs --example voice_pipeline
```

## Example Map

| Example | What it shows | Docs |
| --- | --- | --- |
| `agents_as_tools` | specialist agents exposed as callable tools | [docs/agents.md](../docs/agents.md) |
| `agents_as_tools_conditional` | dynamically enabled agent tools | [docs/agents.md](../docs/agents.md) |
| `agents_as_tools_streaming` | stream events emitted by a nested agent tool | [docs/streaming.md](../docs/streaming.md) |
| `agents_as_tools_structured` | structured input for agent-as-tool calls | [docs/agents.md](../docs/agents.md) |
| `basic_run` | the smallest end-to-end facade call | [docs/quickstart.md](../docs/quickstart.md) |
| `custom_agent_model` | per-agent model name resolved by a custom provider | [docs/models/providers.md](../docs/models/providers.md) |
| `custom_model_provider` | per-run custom model provider selection | [docs/models/providers.md](../docs/models/providers.md) |
| `default_model_provider` | global default runner model provider | [docs/models/providers.md](../docs/models/providers.md) |
| `deterministic_flow` | multi-step agent workflow with an explicit gate | [docs/multi_agent.md](../docs/multi_agent.md) |
| `dynamic_system_prompt` | per-run agent instructions | [docs/agents.md](../docs/agents.md) |
| `forcing_tool_use` | required tool choice with stop-on-tool output | [docs/models/settings.md](../docs/models/settings.md) |
| `function_tools` | typed local function tools with structured arguments | [docs/tools.md](../docs/tools.md) |
| `handoffs` | control transfer between agents | [docs/handoffs.md](../docs/handoffs.md) |
| `image_tool_output` | function tool returning an image output item | [docs/tools.md](../docs/tools.md) |
| `input_guardrail` | tripwire handling before model execution | [docs/guardrails.md](../docs/guardrails.md) |
| `lifecycle_hooks` | run and agent lifecycle callbacks | [docs/running_agents.md](../docs/running_agents.md) |
| `llm_as_a_judge` | iterative generator and evaluator agent loop | [docs/multi_agent.md](../docs/multi_agent.md) |
| `memory_session` | persistent session state across turns | [docs/sessions/README.md](../docs/sessions/README.md) |
| `non_strict_output_type` | structured output validation with non-strict and custom schemas | [docs/agents.md](../docs/agents.md) |
| `output_guardrail` | tripwire handling for final model output | [docs/guardrails.md](../docs/guardrails.md) |
| `parallelization` | concurrent agent calls followed by a picker agent | [docs/multi_agent.md](../docs/multi_agent.md) |
| `previous_response_id` | Responses API conversation continuation by id | [docs/running_agents.md](../docs/running_agents.md) |
| `prompt_template` | static and dynamic reusable prompt config | [docs/agents.md](../docs/agents.md) |
| `sandbox_workspace` | local sandbox manifest staging and shell confinement | [docs/sandbox_agents.md](../docs/sandbox_agents.md) |
| `sqlite_session` | SQLite-backed conversation memory | [docs/sessions/memory.md](../docs/sessions/memory.md) |
| `stream_function_call_args` | streamed tool-call argument items | [docs/streaming.md](../docs/streaming.md) |
| `stream_items` | streamed run-item events for tool calls and messages | [docs/streaming.md](../docs/streaming.md) |
| `stream_text` | streamed message text output | [docs/streaming.md](../docs/streaming.md) |
| `streamed_run` | live streamed execution with `run_streamed` | [docs/streaming.md](../docs/streaming.md) |
| `tool_guardrails` | input and output guardrails around function tools | [docs/guardrails.md](../docs/guardrails.md) |
| `usage_tracking` | token usage from a completed run | [docs/usage.md](../docs/usage.md) |
| `realtime_session` | a long-lived realtime session with live text interaction | [docs/realtime/README.md](../docs/realtime/README.md) |
| `retry` | runner-managed model retry settings | [docs/models/settings.md](../docs/models/settings.md) |
| `routing` | streamed triage handoff to a specialist agent | [docs/handoffs.md](../docs/handoffs.md) |
| `voice_pipeline` | a voice workflow and buffered audio pipeline | [docs/voice/README.md](../docs/voice/README.md) |

## When To Prefer Docs

- Start in [docs/index.md](../docs/index.md) if you are new to the library.
- Open [docs/ref/README.md](../docs/ref/README.md) if you need the public API map.
- Use these examples when you want a concrete starting point you can run and edit immediately.
