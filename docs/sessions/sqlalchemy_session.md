# Database Sessions

The Rust extension equivalent of the upstream SQLAlchemy example is `DatabaseSession`. It exposes a database-URL-backed session interface over the shared `Session` trait.

The upstream-named [sqlalchemy_session_example.rs](../../crates/openai-agents/examples/sqlalchemy_session_example.rs) includes the Rust [database_session.rs](../../crates/openai-agents/examples/database_session.rs) example so users coming from Python can find the same workflow.

## Quick Start

```rust,no_run
use openai_agents::extensions::DatabaseSession;
use openai_agents::{Agent, AgentsError, Runner};

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("Assistant").build();
    let session = DatabaseSession::open("conversation_123", "sqlite::memory:").await?;

    let result = Runner::default()
        .run_with_session(&agent, "Hello", &session)
        .await?;
    println!("{}", result.final_output_text().unwrap_or_default());
    Ok(())
}
```

## When To Use It

Use `DatabaseSession` when you want the extension session abstraction and a database URL instead of constructing the core SQLite session directly. It is useful for parity with upstream database-session examples and for code that wants to keep session construction behind a URL-based factory.

For direct SQLite control, use `SQLiteSession` or `AdvancedSQLiteSession`.

## API Shape

`DatabaseSession` implements the shared `Session` trait:

- `get_items`
- `get_items_with_limit`
- `add_items`
- `pop_item`
- `clear_session`

The current Rust implementation uses the SQLite-backed storage path. It does not expose Python SQLAlchemy engines or driver-specific pooling.

## Read Next

- [README.md](README.md)
- [memory.md](memory.md)
- [advanced_sqlite_session.md](advanced_sqlite_session.md)
- [encrypted_session.md](encrypted_session.md)
