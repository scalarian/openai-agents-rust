# Tracing

Use this page when you need to understand what happened during a run and you want that story in spans instead of only in logs.

## What The Runtime Traces

- run start and end
- model calls
- tool execution
- handoffs
- guardrails
- session persistence
- speech and transcription flows

## Main Surfaces

- `Trace`
- `Span`
- `SpanData`
- tracing processors
- tracing setup helpers such as `add_trace_processor`, `flush_traces`, and `set_tracing_disabled`

## A Good Default Workflow

1. attach a tracing processor in application startup
2. add trace metadata on high-value workflows
3. flush on shutdown or in tests

## Why Tracing Beats Free-Form Logging

Tracing keeps causality intact across:

- nested tools
- handoffs
- streamed runs
- voice pipelines
- realtime sessions

## Read Next

- [running_agents.md](running_agents.md)
- [streaming.md](streaming.md)
- [voice/README.md](voice/README.md)
