# Examples

Use this page when you want runnable starting points instead of longer conceptual guides.

## Runnable Examples

All runnable examples live in `crates/openai-agents/examples`.

| Example | What it covers | File |
| --- | --- | --- |
| basic run | smallest end-to-end call | [basic_run.rs](../crates/openai-agents/examples/basic_run.rs) |
| memory session | session-backed follow-up turns | [memory_session.rs](../crates/openai-agents/examples/memory_session.rs) |
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
