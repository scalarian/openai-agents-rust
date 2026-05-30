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

Runnable examples: [memory_session.rs](../../crates/openai-agents/examples/memory_session.rs) for in-memory state, [sqlite_session.rs](../../crates/openai-agents/examples/sqlite_session.rs) for SQLite-backed state, [advanced_sqlite_session.rs](../../crates/openai-agents/examples/advanced_sqlite_session.rs) for the extension SQLite session with custom table names and persisted tool history, [async_sqlite_session.rs](../../crates/openai-agents/examples/async_sqlite_session.rs) for async-friendly SQLite session storage, [database_session.rs](../../crates/openai-agents/examples/database_session.rs) for database URL-backed sessions, [encrypted_session.rs](../../crates/openai-agents/examples/encrypted_session.rs) for encrypted-at-rest session storage, [mongodb_session.rs](../../crates/openai-agents/examples/mongodb_session.rs) for MongoDB-backed storage, [redis_session.rs](../../crates/openai-agents/examples/redis_session.rs) for Redis-backed storage, and [dapr_session.rs](../../crates/openai-agents/examples/dapr_session.rs) for Dapr state-store backed storage.

## Session Settings

Use `SessionSettings` when you need bounds such as history limits. That is the right layer for controlling replay growth without rewriting session storage code.

## Read Next

- [openai.md](openai.md)
- [../quickstart.md](../quickstart.md)
- [../results.md](../results.md)
