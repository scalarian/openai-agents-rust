# Voice Workflow

Use this page when you want to understand how voice uses the shared runner rather than bypassing it.

## Main Idea

`SingleAgentVoiceWorkflow` turns streamed core run output into text chunks that the voice pipeline can speak or further process.

That means voice stays aligned with:

- the shared runner
- shared sessions
- shared tracing
- shared tool and handoff behavior

## Why This Matters

A separate “voice-only agent system” would drift. Keeping voice on top of the main runner means text, tools, sessions, and traces stay compatible.

## Read Next

- [pipeline.md](pipeline.md)
- [../streaming.md](../streaming.md)
