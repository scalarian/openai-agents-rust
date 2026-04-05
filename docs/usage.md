# Usage

Use this page when you care about turns, usage, replay growth, and the operational shape of your agent runs.

## What To Measure

- number of turns
- tool usage frequency
- guardrail trip frequency
- replay growth over time
- session size and compaction behavior
- trace coverage

## Runtime Surfaces That Help

- `Usage`
- `RunResult`
- `RunResultStreaming`
- tracing processors
- session settings and compaction-aware sessions

## Practical Production Guidance

- set `max_turns` intentionally
- inspect replay size when long-lived sessions are involved
- keep tool outputs structured and bounded
- trace high-value runs, not every experiment forever
- add compaction before prompts become unbounded

## Why This Page Exists

Agent runtimes get expensive and hard to debug when usage and replay are treated as invisible byproducts. In this runtime, they are first-class outputs.

## Read Next

- [results.md](results.md)
- [tracing.md](tracing.md)
- [sessions/openai.md](sessions/openai.md)
