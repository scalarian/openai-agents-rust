# OpenAI Models

Use this page when you want the OpenAI-specific runtime behavior: default provider resolution, hosted tools, Responses, Chat Completions, and conversation-aware sessions.

## The Two Main Paths

| Path | Use it when |
| --- | --- |
| Responses | you want the conversation-aware modern path |
| Chat Completions | you need compatibility with that API shape |

## What The OpenAI Layer Adds

- default provider behavior
- hosted tool constructors
- conversation-aware session helpers
- compaction-aware sessions
- Responses websocket support
- request shaping helpers

## Practical Guidance

- prefer Responses for new work
- use Chat Completions when you have a real compatibility reason
- keep OpenAI-specific request shaping in config, not in app-level string prompts

## Read Next

- [../sessions/openai.md](../sessions/openai.md)
- [../tools.md](../tools.md)
- [../ref/openai.md](../ref/openai.md)
