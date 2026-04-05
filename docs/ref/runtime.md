# Runtime Reference

Use this page when you already know the concept you need and want the shortest path to the relevant runtime type or function.

## Core Entry Points

| Need | Type or function |
| --- | --- |
| one-shot async run | `run` |
| one-shot sync run | `run_sync` |
| configured reusable runner | `Runner` |
| live incremental run | `run_streamed`, `RunResultStreaming` |

## Agent Surfaces

- `Agent`
- `AgentBuilder`
- `AgentHooks`
- `AgentOutputSchema`
- `Agent::as_tool()`

## Run Surfaces

- `RunConfig`
- `RunOptions`
- `RunResult`
- `RunResultStreaming`
- `RunState`

## Tooling Surfaces

- `function_tool`
- `FunctionTool`
- `ToolOutput`
- `ToolErrorFormatter`
- shell and computer tool types
- approval and guardrail types

## Session Surfaces

- `Session`
- `MemorySession`
- `SQLiteSession`
- `SessionSettings`

## Tracing Surfaces

- `Trace`
- `Span`
- `TracingProcessor`
- `add_trace_processor`
- `flush_traces`
- `set_tracing_disabled`

## Good Companion Pages

- [../agents.md](../agents.md)
- [../running_agents.md](../running_agents.md)
- [../tools.md](../tools.md)
- [../sessions/README.md](../sessions/README.md)
