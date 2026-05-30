# Examples

Use this page when you want runnable starting points instead of longer conceptual guides.

## Runnable Examples

All runnable examples live in `crates/openai-agents/examples`.

| Example | What it covers | File |
| --- | --- | --- |
| agents as tools | specialist agents exposed as callable tools | [agents_as_tools.rs](../crates/openai-agents/examples/agents_as_tools.rs) |
| basic run | smallest end-to-end call | [basic_run.rs](../crates/openai-agents/examples/basic_run.rs) |
| dynamic system prompt | per-run agent instructions | [dynamic_system_prompt.rs](../crates/openai-agents/examples/dynamic_system_prompt.rs) |
| function tools | typed local tools with JSON-schema inputs | [function_tools.rs](../crates/openai-agents/examples/function_tools.rs) |
| handoffs | control transfer between agents | [handoffs.rs](../crates/openai-agents/examples/handoffs.rs) |
| input guardrail | local tripwire handling before model execution | [input_guardrail.rs](../crates/openai-agents/examples/input_guardrail.rs) |
| lifecycle hooks | run and agent lifecycle callbacks | [lifecycle_hooks.rs](../crates/openai-agents/examples/lifecycle_hooks.rs) |
| memory session | session-backed follow-up turns | [memory_session.rs](../crates/openai-agents/examples/memory_session.rs) |
| output guardrail | final output tripwire handling | [output_guardrail.rs](../crates/openai-agents/examples/output_guardrail.rs) |
| previous response id | continue a Responses API conversation by id | [previous_response_id.rs](../crates/openai-agents/examples/previous_response_id.rs) |
| prompt template | static and dynamic reusable prompt config | [prompt_template.rs](../crates/openai-agents/examples/prompt_template.rs) |
| sandbox workspace | local sandbox workspace preparation and shell confinement | [sandbox_workspace.rs](../crates/openai-agents/examples/sandbox_workspace.rs) |
| streamed run | live events and completion | [streamed_run.rs](../crates/openai-agents/examples/streamed_run.rs) |
| tool guardrails | function-tool input and output guardrails | [tool_guardrails.rs](../crates/openai-agents/examples/tool_guardrails.rs) |
| usage tracking | token usage from a completed run | [usage_tracking.rs](../crates/openai-agents/examples/usage_tracking.rs) |
| realtime session | long-lived realtime interaction | [realtime_session.rs](../crates/openai-agents/examples/realtime_session.rs) |
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
