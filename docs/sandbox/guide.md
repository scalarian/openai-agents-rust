# Sandbox Guide

Sandbox agents pair a normal agent with a prepared workspace. Use them when the task is grounded in files, generated artifacts, shell commands, or workspace state instead of only conversation history.

The Rust sandbox surface is local-first:

- `SandboxAgent` carries the normal agent configuration plus sandbox defaults.
- `Manifest` declares the files, directories, and extra path grants for fresh workspaces.
- `SandboxRunConfig` chooses the per-run sandbox input: a manifest override, a live local session, or serialized local session state.
- `LocalSandboxSession` owns filesystem, shell, patch, PTY, snapshot, and memory-note operations.
- `prepare_sandbox_run` materializes the workspace, attaches capability tools, and returns the prepared `Agent` plus session.

## Minimal Flow

The runnable version lives in [sandbox_workspace.rs](../../crates/openai-agents/examples/sandbox_workspace.rs).

```rust,no_run
use openai_agents::{
    AgentsError, File, Manifest, RunConfig, SandboxAgent, SandboxRunConfig, prepare_sandbox_run,
};

fn main() -> Result<(), AgentsError> {
    let sandbox_agent = SandboxAgent::builder("Workspace assistant")
        .instructions("Inspect workspace files before answering.")
        .default_manifest(
            Manifest::default().with_entry("README.md", File::from_text("# Demo\n")),
        )
        .build();

    let prepared = prepare_sandbox_run(
        &sandbox_agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig::default()),
            ..RunConfig::default()
        },
    )?;

    println!("{}", prepared.session.read_file("/workspace/README.md")?);
    prepared.session.cleanup()?;
    Ok(())
}
```

After preparation, run `prepared.agent` with the normal `Runner` APIs. The sandbox tools are ordinary function tools bound to the prepared session.

## When To Use Sandbox Agents

Use a sandbox agent when the workflow needs one of these properties:

- a known starting filesystem layout
- workspace-safe file reads, writes, and patches
- shell commands that run from a controlled workspace root
- resumable local workspace state
- isolated workspaces for sub-agents or reviewers
- generated artifacts that your application can inspect after the run

If the workflow only needs one occasional shell command and no persistent workspace, a normal agent with a hosted or local shell tool is usually simpler.

## Core Pieces

| Piece | Role |
| --- | --- |
| `SandboxAgent` | Agent definition plus sandbox defaults such as `default_manifest`, `base_instructions`, and capabilities. |
| `Manifest` | Fresh-session workspace contract built from `File`, `Dir`, `LocalDir`, and extra path grants. |
| `SandboxCapability` | Controls which sandbox tools are attached. Defaults are filesystem, shell, and patch. |
| `SandboxRunConfig` | Per-run sandbox source: manifest override, live session, or serialized session state. |
| `LocalSandboxSession` | Live workspace API for direct file, shell, patch, PTY, snapshot, and memory-note operations. |

## Preparation Model

`prepare_sandbox_run` does the sandbox-specific work before the normal runner loop starts:

1. Resolve the effective manifest from the run config or agent default.
2. Create, reuse, or restore a local sandbox session.
3. Materialize manifest entries under the logical `/workspace` root.
4. Build sandbox instructions from the base prompt, agent instructions, capabilities, and workspace tree.
5. Attach capability tools to the prepared agent.

The runner still owns turns, model calls, approvals, tracing, handoffs, and durable run state. The sandbox session owns workspace operations and local confinement.

## Capabilities

Sandbox agents default to these capabilities:

| Capability | Tools |
| --- | --- |
| `SandboxCapability::Filesystem` | `sandbox_list_files`, `sandbox_read_file` |
| `SandboxCapability::Shell` | `sandbox_run_shell` |
| `SandboxCapability::ApplyPatch` | `sandbox_apply_patch` |

If you pass `SandboxAgentBuilder::capabilities`, the list replaces the defaults. Include every capability the run should expose.

## Safety Boundaries

The logical workspace root is `/workspace`. Sandbox APIs reject paths that escape that root, including symlink escapes. Shell commands run from the session workspace and are checked for obvious path escapes before execution.

On Linux, the local shell path uses Landlock where available to restrict writes. On macOS, the runtime uses `sandbox-exec` when available. Extra path grants can expose host paths intentionally; keep them read-only unless the workflow truly needs write access.

## Read Next

- [../sandbox_agents.md](../sandbox_agents.md)
- [../examples.md](../examples.md)
- [../tools.md](../tools.md)
- [../human_in_the_loop.md](../human_in_the_loop.md)
