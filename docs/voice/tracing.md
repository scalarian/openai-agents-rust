# Voice Tracing

Voice workflows use the shared agents runtime, so the agent turn inside `SingleAgentVoiceWorkflow` participates in the same tracing model described in [../tracing.md](../tracing.md).

## What Gets Traced

The shared runner can trace:

- the agent run launched by `SingleAgentVoiceWorkflow`
- tool calls and handoffs inside that run
- guardrail activity from the underlying agent workflow
- model request metadata allowed by the tracing configuration

`VoicePipeline` also emits live `VoiceStreamEvent` values for transcript, audio, lifecycle, and errors. Use those events for product telemetry that is specific to audio playback or capture.

## Current Rust Config Surface

`VoicePipelineConfig` currently exposes:

- `stream_audio`
- `split_sentences`
- `stt_settings`
- `tts_settings`

It does not currently expose Python-style voice-specific tracing fields such as audio-data redaction toggles or trace metadata. Use the shared tracing controls from [../tracing.md](../tracing.md) for agent-run tracing, and keep application-level audio telemetry outside the model-visible workflow when it contains sensitive data.

## Read Next

- [quickstart.md](quickstart.md)
- [pipeline.md](pipeline.md)
- [workflow.md](workflow.md)
- [../tracing.md](../tracing.md)
