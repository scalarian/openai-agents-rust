# Sandbox Memory

Sandbox memory is workspace state that survives local sandbox resume. It is separate from conversational sessions such as `SQLiteSession` and `OpenAIConversationsSession`.

In this Rust runtime, sandbox memory is exposed as explicit memory notes on `LocalSandboxSession`:

- `write_memory_note(key, value)` stores a note in the serialized sandbox session state.
- `read_memory_note(key)` reads a note after live reuse or state restore.
- memory notes travel with `serialize_session_state`.

Use this when follow-up sandbox runs should remember short operational facts, user preferences, or prior fixes without replaying the whole conversation.

## Minimal Flow

The runnable version lives in [sandbox_memory.rs](../../crates/openai-agents/examples/sandbox_memory.rs).

```rust,no_run
use openai_agents::{
    AgentsError, LocalSandboxSession, RunConfig, SandboxAgent, SandboxRunConfig,
    prepare_sandbox_run,
};

fn main() -> Result<(), AgentsError> {
    let agent = SandboxAgent::builder("memory demo")
        .instructions("Use sandbox session memory when follow-up work depends on prior fixes.")
        .build();

    let first = prepare_sandbox_run(
        &agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig::default()),
            ..RunConfig::default()
        },
    )?;
    first
        .session
        .write_memory_note("last_fix", "Invoice total must multiply by 1 + tax_rate.")?;

    let encoded = first.session.serialize_session_state()?;
    let restored_state = LocalSandboxSession::deserialize_session_state(encoded)?;
    let resumed = prepare_sandbox_run(
        &agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig {
                session_state: Some(restored_state),
                ..SandboxRunConfig::default()
            }),
            ..RunConfig::default()
        },
    )?;

    assert_eq!(
        resumed.session.read_memory_note("last_fix")?,
        Some("Invoice total must multiply by 1 + tax_rate.".to_owned())
    );

    resumed.session.cleanup()?;
    Ok(())
}
```

## What To Store

Store compact facts that are useful without the full transcript:

- a bug fix or verification command that matters for follow-up work
- a known data-source location inside the workspace
- a user preference that should shape the next sandbox run
- a handoff note between isolated sandbox agents

Do not use memory notes as a database replacement. Large artifacts belong in workspace files, and conversational history belongs in a `Session`.

## Multi-Agent Layouts

Memory notes use string keys, so applications can isolate layouts by prefix convention.

```rust,no_run
session.write_memory_note("gtm:last_analysis", "Segment enterprise buyers first.")?;
session.write_memory_note("engineering:last_fix", "Add regression coverage for invoice totals.")?;
```

See [sandbox_memory_multi_agent.rs](../../crates/openai-agents/examples/sandbox_memory_multi_agent.rs) for a runnable example that keeps GTM and engineering notes separate while sharing sandbox resume mechanics.

## Persistent Workspace Layouts

If the memory should also be visible as workspace files, write files under a stable directory such as `persistent/memories/` and include that directory in the serialized or restored sandbox state.

[sandbox_memory_s3.rs](../../crates/openai-agents/examples/sandbox_memory_s3.rs) demonstrates a persistent-memory-style layout using local sandbox files and memory notes. Hosted remote storage can use the provider mount payload types described in [clients.md](clients.md), but live provider validation depends on credentials and provider setup.

## Combine With Conversation Sessions

For multi-turn chats, use both layers:

- a conversational `Session` stores model-visible message history
- the sandbox session stores workspace files and memory notes

Pass the same live sandbox session or restored `session_state` when you want workspace continuity. Pass the same conversational session when you want chat continuity.

## Read Next

- [guide.md](guide.md)
- [clients.md](clients.md)
- [../sessions/README.md](../sessions/README.md)
- [../examples.md](../examples.md)
