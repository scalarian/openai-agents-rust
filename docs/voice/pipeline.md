# Voice Pipeline

Use this page when you need the full STT -> workflow -> TTS loop.

## What The Pipeline Does

1. accepts buffered or streamed audio input
2. transcribes it
3. runs the workflow
4. emits transcript and audio events
5. exposes completion state through `StreamedAudioResult`

## Why `StreamedAudioResult` Matters

The result is live, not just a static summary. It owns:

- transcript events
- audio events
- lifecycle events
- completion and error state

## Read Next

- [README.md](README.md)
- [../streaming.md](../streaming.md)
- [../tracing.md](../tracing.md)
