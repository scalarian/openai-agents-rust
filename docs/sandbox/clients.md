# Sandbox Clients

Use this page to choose where sandbox work should run and which Rust feature flags expose optional hosted-provider client types.

The core Rust runtime currently provides a local sandbox session through `prepare_sandbox_run` and `LocalSandboxSession`. Optional hosted-provider clients live in `openai_agents::extensions::sandbox` behind crate features. Docker-backed sandbox parity is intentionally deferred for this workspace.

## Decision Guide

| Goal | Start with | Why |
| --- | --- | --- |
| local development on macOS or Linux | `SandboxRunConfig::default()` | No provider credentials or extra feature flags. |
| resume local workspace state | `SandboxRunConfig { session_state: Some(state), .. }` | Restores a serialized `LocalSandboxSessionState`. |
| reuse a live local workspace | `SandboxRunConfig { session: Some(session), .. }` | Keeps one live `LocalSandboxSession` across runs. |
| inspect hosted-provider configuration | `openai_agents::extensions::sandbox::*` | Enables provider-specific option, state, auth, and mount payload types. |

## Local Client Path

Most applications do not instantiate a separate client for local sandbox work. They pass `SandboxRunConfig` into `prepare_sandbox_run`.

```rust,no_run
use openai_agents::{AgentsError, RunConfig, SandboxAgent, SandboxRunConfig, prepare_sandbox_run};

fn main() -> Result<(), AgentsError> {
    let agent = SandboxAgent::builder("local sandbox").build();
    let prepared = prepare_sandbox_run(
        &agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig::default()),
            ..RunConfig::default()
        },
    )?;

    println!("{}", prepared.session.workspace_root().display());
    prepared.session.cleanup()?;
    Ok(())
}
```

The returned `LocalSandboxSession` exposes direct operations:

- `read_file`, `write_file`, `list_files`, and `apply_patch`
- `run_shell`
- `open_pty`
- `serialize_session_state` and `deserialize_session_state`
- `write_memory_note` and `read_memory_note`

## Resume Choices

Use a live session when multiple runs happen in one process and you want immediate workspace continuity. Use serialized session state when the application needs to persist or transfer the workspace state.

```rust,no_run
use openai_agents::{
    AgentsError, LocalSandboxSession, RunConfig, SandboxAgent, SandboxRunConfig,
    prepare_sandbox_run,
};

fn main() -> Result<(), AgentsError> {
    let agent = SandboxAgent::builder("resumable sandbox").build();
    let first = prepare_sandbox_run(
        &agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig::default()),
            ..RunConfig::default()
        },
    )?;
    first.session.write_file("/workspace/status.txt", "ready\n")?;

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

    assert_eq!(resumed.session.read_file("/workspace/status.txt")?, "ready\n");
    resumed.session.cleanup()?;
    Ok(())
}
```

See [sandbox_remote_snapshot.rs](../../crates/openai-agents/examples/sandbox_remote_snapshot.rs) and [sandbox_tutorial_resume.rs](../../crates/openai-agents/examples/sandbox_tutorial_resume.rs) for runnable resume flows.

## Hosted Provider Types

Hosted provider support is exposed through feature flags on `openai-agents-rs`. Enable only the providers you need.

| Feature | Client | Auth environment variable | PTY support |
| --- | --- | --- | --- |
| `e2b` | `E2BSandboxClient` | `E2B_API_KEY` | yes |
| `modal` | `ModalSandboxClient` | `MODAL_TOKEN_ID` | no |
| `daytona` | `DaytonaSandboxClient` | `DAYTONA_API_KEY` | yes |
| `blaxel` | `BlaxelSandboxClient` | `BL_API_KEY` | yes |
| `cloudflare` | `CloudflareSandboxClient` | `CLOUDFLARE_SANDBOX_API_KEY` | yes |
| `runloop` | `RunloopSandboxClient` | `RUNLOOP_API_KEY` | no |
| `vercel` | `VercelSandboxClient` | `VERCEL_TOKEN` | no |

Example:

```bash
cargo run -p openai-agents-rs --features e2b --example sandbox_e2b_extension
```

Provider examples are indexed in [../examples.md](../examples.md). They cover option construction, auth precedence, session state serialization, resume, exposed ports, idle timeout, and provider-specific workspace-root constraints.

## Hosted Mount Payloads

The extension crate also exposes hosted mount entry and strategy types for provider payload parity:

| Storage | Entry type |
| --- | --- |
| S3 | `HostedS3Mount` |
| Cloudflare R2 | `HostedR2Mount` |
| Google Cloud Storage | `HostedGcsMount` |

Use `HostedMountStrategy` to resolve provider payloads for E2B, Modal, Daytona, Blaxel, Cloudflare, and Runloop. Vercel currently has no hosted-specific mount strategy in this Rust surface.

## Read Next

- [guide.md](guide.md)
- [memory.md](memory.md)
- [../examples.md](../examples.md)
- [../sandbox_agents.md](../sandbox_agents.md)
