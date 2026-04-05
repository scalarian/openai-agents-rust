# Memory And SQLite Sessions

Use this page when you need a local session backend without OpenAI-specific conversation semantics.

## Main Types

- `MemorySession`
- `SQLiteSession`
- `SessionSettings`

## When To Use Which

| Session type | Good for |
| --- | --- |
| `MemorySession` | tests, demos, short-lived app state |
| `SQLiteSession` | local persistence, desktop apps, prototypes that need durability |

## Session Settings

Use `SessionSettings` when you need bounds such as history limits. That is the right layer for controlling replay growth without rewriting session storage code.

## Read Next

- [openai.md](openai.md)
- [../quickstart.md](../quickstart.md)
- [../results.md](../results.md)
