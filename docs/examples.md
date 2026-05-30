# Examples

Use this page when you want runnable starting points instead of longer conceptual guides.

## Runnable Examples

All runnable examples live in `crates/openai-agents/examples`.

| Example | What it covers | File |
| --- | --- | --- |
| agents as tools | specialist agents exposed as callable tools | [agents_as_tools.rs](../crates/openai-agents/examples/agents_as_tools.rs) |
| agents as tools conditional | dynamically enabled agent tools | [agents_as_tools_conditional.rs](../crates/openai-agents/examples/agents_as_tools_conditional.rs) |
| agents as tools streaming | stream events emitted by a nested agent tool | [agents_as_tools_streaming.rs](../crates/openai-agents/examples/agents_as_tools_streaming.rs) |
| agents as tools structured | structured input for agent-as-tool calls | [agents_as_tools_structured.rs](../crates/openai-agents/examples/agents_as_tools_structured.rs) |
| basic run | smallest end-to-end call | [basic_run.rs](../crates/openai-agents/examples/basic_run.rs) |
| custom agent model | per-agent model name resolved by a custom provider | [custom_agent_model.rs](../crates/openai-agents/examples/custom_agent_model.rs) |
| custom model provider | per-run custom model provider selection | [custom_model_provider.rs](../crates/openai-agents/examples/custom_model_provider.rs) |
| default model provider | global default runner model provider | [default_model_provider.rs](../crates/openai-agents/examples/default_model_provider.rs) |
| deterministic flow | multi-step agent workflow with an explicit gate | [deterministic_flow.rs](../crates/openai-agents/examples/deterministic_flow.rs) |
| dynamic system prompt | per-run agent instructions | [dynamic_system_prompt.rs](../crates/openai-agents/examples/dynamic_system_prompt.rs) |
| function tools | typed local tools with JSON-schema inputs | [function_tools.rs](../crates/openai-agents/examples/function_tools.rs) |
| forcing tool use | required tool choice and stop-on-tool behavior | [forcing_tool_use.rs](../crates/openai-agents/examples/forcing_tool_use.rs) |
| handoffs | control transfer between agents | [handoffs.rs](../crates/openai-agents/examples/handoffs.rs) |
| image tool output | function tool returning an image output item | [image_tool_output.rs](../crates/openai-agents/examples/image_tool_output.rs) |
| input guardrail | local tripwire handling before model execution | [input_guardrail.rs](../crates/openai-agents/examples/input_guardrail.rs) |
| lifecycle hooks | run and agent lifecycle callbacks | [lifecycle_hooks.rs](../crates/openai-agents/examples/lifecycle_hooks.rs) |
| llm as a judge | iterative generator and evaluator agent loop | [llm_as_a_judge.rs](../crates/openai-agents/examples/llm_as_a_judge.rs) |
| memory session | session-backed follow-up turns | [memory_session.rs](../crates/openai-agents/examples/memory_session.rs) |
| non-strict output type | structured output validation with non-strict and custom schemas | [non_strict_output_type.rs](../crates/openai-agents/examples/non_strict_output_type.rs) |
| output guardrail | final output tripwire handling | [output_guardrail.rs](../crates/openai-agents/examples/output_guardrail.rs) |
| parallelization | run multiple agent calls concurrently and pick the best result | [parallelization.rs](../crates/openai-agents/examples/parallelization.rs) |
| previous response id | continue a Responses API conversation by id | [previous_response_id.rs](../crates/openai-agents/examples/previous_response_id.rs) |
| prompt template | static and dynamic reusable prompt config | [prompt_template.rs](../crates/openai-agents/examples/prompt_template.rs) |
| sandbox workspace | local sandbox workspace preparation and shell confinement | [sandbox_workspace.rs](../crates/openai-agents/examples/sandbox_workspace.rs) |
| sqlite session | SQLite-backed conversation memory | [sqlite_session.rs](../crates/openai-agents/examples/sqlite_session.rs) |
| stream function call args | streamed tool-call argument items | [stream_function_call_args.rs](../crates/openai-agents/examples/stream_function_call_args.rs) |
| stream items | streamed run-item events for tool calls and messages | [stream_items.rs](../crates/openai-agents/examples/stream_items.rs) |
| stream text | streamed message text output | [stream_text.rs](../crates/openai-agents/examples/stream_text.rs) |
| streamed run | live events and completion | [streamed_run.rs](../crates/openai-agents/examples/streamed_run.rs) |
| tool guardrails | function-tool input and output guardrails | [tool_guardrails.rs](../crates/openai-agents/examples/tool_guardrails.rs) |
| usage tracking | token usage from a completed run | [usage_tracking.rs](../crates/openai-agents/examples/usage_tracking.rs) |
| realtime session | long-lived realtime interaction | [realtime_session.rs](../crates/openai-agents/examples/realtime_session.rs) |
| retry | runner-managed model retry settings | [retry.rs](../crates/openai-agents/examples/retry.rs) |
| routing | streamed triage handoff to a specialist agent | [routing.rs](../crates/openai-agents/examples/routing.rs) |
| voice pipeline | STT -> workflow -> TTS flow | [voice_pipeline.rs](../crates/openai-agents/examples/voice_pipeline.rs) |

## Read By Goal

- first program: [quickstart.md](quickstart.md)
- tools: [tools.md](tools.md)
- sessions: [sessions/README.md](sessions/README.md)
- streaming: [streaming.md](streaming.md)
- realtime: [realtime/README.md](realtime/README.md)
- voice: [voice/README.md](voice/README.md)

## Example Design Rules

Good examples in this repo should:

- compile
- stay short
- focus on one capability
- link back to the canonical docs page

If an example needs five unrelated setup steps, it should probably become a guide instead.
