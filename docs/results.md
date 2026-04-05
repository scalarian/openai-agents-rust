# Results

Use this page when you need to inspect what actually happened during a run, continue a conversation, or replay the output as the next input.

## The Two Result Shapes

| Type | When you see it |
| --- | --- |
| `RunResult` | after a completed run |
| `RunResultStreaming` | while a streamed run is still live |

## What `RunResult` Carries

- agent identity
- input and normalized input
- output items and new items
- raw model responses
- final output text
- guardrail results
- interruptions
- usage
- trace
- conversation and previous-response ids
- replayable run state

## Replay

Use:

- `to_input_list()` when you want full replay including preserved history
- `to_input_list_mode(ToInputListMode::Normalized)` when you want the normalized continuation shape

That replay machinery is the foundation for:

- follow-up turns
- resume flows
- session persistence
- debugging and tests

## Last Response Helpers

Both `RunResult` and `RunResultStreaming` expose last-response helpers so you can inspect the latest provider response without digging through internal item lists manually.

## Agent Tool Invocation

Nested agent-tool runs are captured as structured invocation metadata, not just free-form strings. That makes them suitable for:

- debugging
- operator review
- replayable workflows

## Read Next

- [sessions/README.md](sessions/README.md)
- [streaming.md](streaming.md)
- [usage.md](usage.md)
