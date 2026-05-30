# Examples

Use this page when you want runnable starting points instead of longer conceptual guides.

## Runnable Examples

All runnable examples live in `crates/openai-agents/examples`.

| Example | What it covers | File |
| --- | --- | --- |
| agents as tools | specialist agents exposed as callable tools | [agents_as_tools.rs](../crates/openai-agents/examples/agents_as_tools.rs) |
| basic run | smallest end-to-end call | [basic_run.rs](../crates/openai-agents/examples/basic_run.rs) |
| function tools | typed local tools with JSON-schema inputs | [function_tools.rs](../crates/openai-agents/examples/function_tools.rs) |
| input guardrail | local tripwire handling before model execution | [input_guardrail.rs](../crates/openai-agents/examples/input_guardrail.rs) |
| memory session | session-backed follow-up turns | [memory_session.rs](../crates/openai-agents/examples/memory_session.rs) |
| sandbox workspace | local sandbox workspace preparation and shell confinement | [sandbox_workspace.rs](../crates/openai-agents/examples/sandbox_workspace.rs) |
| streamed run | live events and completion | [streamed_run.rs](../crates/openai-agents/examples/streamed_run.rs) |
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
