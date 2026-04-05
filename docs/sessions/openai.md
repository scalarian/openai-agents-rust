# OpenAI Sessions

Use this page when you want sessions that preserve OpenAI conversation behavior rather than only storing local item history.

## Main Types

- `OpenAIConversationsSession`
- `OpenAIResponsesCompactionSession`

## What They Add

- conversation identifiers
- previous-response identifiers
- provider-aware continuation behavior
- compaction decisions tied to Responses-style history management

## When To Use Them

Use an OpenAI-specific session when:

- you want OpenAI-managed conversation continuity
- you want compaction-aware long-lived sessions
- you need runtime continuity across normal runs, streamed runs, and resumes

## Compaction

Compaction exists to keep long-lived conversations usable. Treat it as part of the runtime state model, not as ad-hoc string trimming.

## Read Next

- [../models/openai.md](../models/openai.md)
- [../results.md](../results.md)
- [../usage.md](../usage.md)
