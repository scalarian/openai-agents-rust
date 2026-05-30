# Sessions

Use this section when you want state to survive across turns, runs, or process boundaries.

## In This Section

- [index.md](index.md): upstream-compatible entry point for this section
- [memory.md](memory.md): in-memory and SQLite-backed session basics
- [openai.md](openai.md): OpenAI conversation-aware sessions and compaction
- [advanced_sqlite_session.md](advanced_sqlite_session.md): extension SQLite sessions with custom database URLs and table names
- [encrypted_session.md](encrypted_session.md): encryption wrapper for persisted session items
- [sqlalchemy_session.md](sqlalchemy_session.md): Rust database-session equivalent for upstream SQLAlchemy examples

The runnable [compaction_session.rs](../../crates/openai-agents/examples/compaction_session.rs) and [compaction_session_stateless.rs](../../crates/openai-agents/examples/compaction_session_stateless.rs) examples show automatic, manual, and `store=false` OpenAI Responses compaction.

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
- [advanced_sqlite_session.md](advanced_sqlite_session.md)
- [encrypted_session.md](encrypted_session.md)
- [sqlalchemy_session.md](sqlalchemy_session.md)
- [../results.md](../results.md)
