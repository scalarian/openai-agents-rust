# Encrypted Sessions

`EncryptedSession` wraps any session implementation and stores encrypted envelopes in the underlying session. Reads decrypt valid envelopes back into normal input items before replay.

Use it when session data is persisted locally but should not be stored as plaintext.

## Quick Start

The runnable version lives in [encrypted_session.rs](../../crates/openai-agents/examples/encrypted_session.rs).

```rust,no_run
use openai_agents::extensions::EncryptedSession;
use openai_agents::{Agent, AgentsError, Runner, SQLiteSession};

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("Assistant").build();
    let underlying = SQLiteSession::open_in_memory("conversation_123").await?;
    let session = EncryptedSession::new(underlying, "my-secret-encryption-key")
        .with_ttl_seconds(3600);

    let result = Runner::default()
        .run_with_session(&agent, "Hello", &session)
        .await?;
    println!("{}", result.final_output_text().unwrap_or_default());
    Ok(())
}
```

## Behavior

- The wrapper implements the same `Session` trait as the underlying session.
- Stored items become `EncryptedEnvelope` JSON items in the inner session.
- `get_items` and `get_items_with_limit` return decrypted items.
- Expired items are skipped when `with_ttl_seconds` is configured.
- The encryption key can be any application-provided secret string.

## Underlying Sessions

`EncryptedSession<S>` is generic over the wrapped session. Common pairings are:

- `EncryptedSession<SQLiteSession>` for local SQLite persistence.
- `EncryptedSession<AdvancedSQLiteSession>` when you also need custom SQLite table names.
- `EncryptedSession<DatabaseSession>` when you want the database-session extension surface.

Content-based queries on the underlying session see encrypted envelopes, not plaintext messages. Decrypt through the wrapper when you need model-visible item history.

## Read Next

- [README.md](README.md)
- [memory.md](memory.md)
- [advanced_sqlite_session.md](advanced_sqlite_session.md)
- [sqlalchemy_session.md](sqlalchemy_session.md)
