# Advanced SQLite Sessions

`AdvancedSQLiteSession` is an extension session for SQLite-backed local conversation state with configurable database URLs and table names.

Use it when the built-in `SQLiteSession` behavior is right, but your application needs explicit table names or wants the extension wrapper used by the upstream-named examples.

## Quick Start

The runnable version lives in [advanced_sqlite_session.rs](../../crates/openai-agents/examples/advanced_sqlite_session.rs).

```rust,no_run
use openai_agents::extensions::AdvancedSQLiteSession;
use openai_agents::{Agent, AgentsError, Runner, SessionSettings};

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("Assistant")
        .instructions("Reply very concisely.")
        .build();

    let session = AdvancedSQLiteSession::open_with_options(
        "conversation_123",
        "sqlite::memory:",
        "agent_sessions",
        "agent_messages",
        Some(SessionSettings::default()),
    )
    .await?;

    let result = Runner::default()
        .run_with_session(&agent, "What city is the Golden Gate Bridge in?", &session)
        .await?;
    println!("{}", result.final_output_text().unwrap_or_default());
    Ok(())
}
```

## Configuration

| Option | Purpose |
| --- | --- |
| `session_id` | Stable conversation identifier. |
| `database_url` | SQLx-style SQLite URL such as `sqlite::memory:` or a file-backed SQLite URL. |
| `sessions_table` | Table for session metadata. |
| `messages_table` | Table for persisted input and output items. |
| `SessionSettings` | Optional bounds such as latest-item replay limits. |

`AdvancedSQLiteSession` implements the shared `Session` trait, so it works with `Runner::run_with_session`, `Runner::resume_with_agent_and_session`, `get_items`, `get_items_with_limit`, `pop_item`, and `clear_session`.

## What It Does Not Do

The Rust extension is intentionally narrower than the Python `AdvancedSQLiteSession`. It does not currently expose conversation branching or usage analytics tables. Track token usage from `RunResult::usage` and store application analytics in your own tables when you need that data.

## Read Next

- [README.md](README.md)
- [memory.md](memory.md)
- [encrypted_session.md](encrypted_session.md)
- [sqlalchemy_session.md](sqlalchemy_session.md)
