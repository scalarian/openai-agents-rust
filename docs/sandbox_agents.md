# Sandbox Agents

Sandbox agents give an agent a prepared workspace with filesystem tools, optional shell access, patching, and resumable local state. Use them when the task is grounded in files instead of only chat history.

## Minimal Local Workspace

This example prepares a local sandbox without making a model call. The runnable version lives in [sandbox_workspace.rs](../crates/openai-agents/examples/sandbox_workspace.rs).

```rust,no_run
use openai_agents::{
    AgentsError, File, Manifest, RunConfig, SandboxAgent, SandboxRunConfig, prepare_sandbox_run,
};

fn main() -> Result<(), AgentsError> {
    let sandbox_agent = SandboxAgent::builder("sandbox engineer")
        .instructions("Use the sandbox workspace for file inspection and edits.")
        .default_manifest(
            Manifest::default().with_entry("notes.txt", File::from_text("hello from sandbox\n")),
        )
        .build();

    let run_config = RunConfig {
        sandbox: Some(SandboxRunConfig::default()),
        ..RunConfig::default()
    };
    let prepared = prepare_sandbox_run(&sandbox_agent, &run_config)?;

    println!("{}", prepared.session.read_file("/workspace/notes.txt")?);
    prepared.session.cleanup()?;
    Ok(())
}
```

## Runtime Model

- `SandboxAgent` wraps a normal `Agent` with sandbox defaults.
- `Manifest` describes files staged into the workspace.
- `File`, `Dir`, and `LocalDir` are the manifest entry types.
- `SandboxRunConfig` carries the manifest override, session state, or live session for a run.
- `LocalSandboxSession` exposes direct workspace operations when your application needs to inspect or persist state outside an agent run.

## Capabilities

Sandbox agents default to filesystem, shell, and patch capabilities. You can narrow that set with `SandboxAgentBuilder::capabilities`.

| Capability | Attached tools |
| --- | --- |
| `SandboxCapability::Filesystem` | `sandbox_list_files`, `sandbox_read_file` |
| `SandboxCapability::Shell` | `sandbox_run_shell` |
| `SandboxCapability::ApplyPatch` | `sandbox_apply_patch` |

## Filesystem Safety

The logical workspace root is `/workspace`; host paths are resolved through the local session root. Reads and writes reject paths that escape the workspace, including symlink escapes.

For shell commands, the runtime validates obvious path escapes before execution and applies platform confinement during execution:

- Linux uses Landlock to allow file mutation only in the workspace and writable extra grants.
- macOS uses `sandbox-exec` when available.

Read-only extra grants can expose host paths to the model without allowing writes through `LocalSandboxSession` APIs.

## Read Next

- [sandbox/guide.md](sandbox/guide.md)
- [sandbox/clients.md](sandbox/clients.md)
- [sandbox/memory.md](sandbox/memory.md)
- [examples.md](examples.md)
- [tools.md](tools.md)
- [human_in_the_loop.md](human_in_the_loop.md)
- [ref/runtime.md](ref/runtime.md)
