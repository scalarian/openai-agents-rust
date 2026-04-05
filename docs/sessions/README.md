# Sessions

Use this section when you want state to survive across turns, runs, or process boundaries.

## In This Section

- [memory.md](memory.md): in-memory and SQLite-backed session basics
- [openai.md](openai.md): OpenAI conversation-aware sessions and compaction

## What Sessions Are For

Sessions own durable conversation state such as:

- prior input items
- continuation history
- conversation identifiers
- provider-specific continuation metadata

## What Sessions Are Not For

Do not use sessions as a grab bag for arbitrary application state. Put application state in your own context object and keep sessions focused on runtime conversation state.

## Read Next

- [memory.md](memory.md)
- [openai.md](openai.md)
- [../results.md](../results.md)
