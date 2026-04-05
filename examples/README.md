# Examples

Runnable examples live in `crates/openai-agents/examples`.

## Run An Example

From the repo root:

```bash
cargo run -p openai-agents --example basic_run
cargo run -p openai-agents --example memory_session
cargo run -p openai-agents --example streamed_run
cargo run -p openai-agents --example realtime_session
cargo run -p openai-agents --example voice_pipeline
```

## Example Map

| Example | What it shows | Docs |
| --- | --- | --- |
| `basic_run` | the smallest end-to-end facade call | [docs/quickstart.md](../docs/quickstart.md) |
| `memory_session` | persistent session state across turns | [docs/sessions/README.md](../docs/sessions/README.md) |
| `streamed_run` | live streamed execution with `run_streamed` | [docs/streaming.md](../docs/streaming.md) |
| `realtime_session` | a long-lived realtime session with live text interaction | [docs/realtime/README.md](../docs/realtime/README.md) |
| `voice_pipeline` | a voice workflow and buffered audio pipeline | [docs/voice/README.md](../docs/voice/README.md) |

## When To Prefer Docs

- Start in [docs/index.md](../docs/index.md) if you are new to the library.
- Open [docs/ref/README.md](../docs/ref/README.md) if you need the public API map.
- Use these examples when you want a concrete starting point you can run and edit immediately.
