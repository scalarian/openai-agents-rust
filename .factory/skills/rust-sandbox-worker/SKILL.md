---
name: rust-sandbox-worker
description: Implement and verify local, Docker, and hosted-provider sandbox parity in the Rust workspace.
---

# Rust Sandbox Worker

NOTE: Startup and cleanup are handled by `worker-base`. This skill defines the WORK PROCEDURE.

## When to Use This Skill

Use this skill for features that add or change sandbox agents, manifests, entries, local sandbox execution, sandbox resume/state, sandbox memory, Docker backends, mounts, or hosted sandbox provider integrations.

## Required Skills

None.

## Work Procedure

1. Read `mission.md`, mission `AGENTS.md`, `.factory/library/architecture.md`, `.factory/library/environment.md`, `.factory/library/parity.md`, and the feature’s sandbox-related assertions before touching code.
2. Preserve the agreed crate ownership:
   - provider-neutral sandbox runtime and local/Docker mechanics belong in core
   - hosted providers belong in extensions
   - facade crate stays a re-export layer
3. Write failing tests first. Prefer deterministic local tests with fake/scripted models. For Docker/provider work, start with mocked or request-shape tests unless the feature explicitly requires live backend verification.
   - For compile-surface or feature-gated export work, it is acceptable to first sketch the minimal gated symbol wiring needed for a temp-crate or compile-matrix test to exist, then immediately finalize the failing compile-surface test before broader implementation.
4. When a feature claims workspace safety, `LocalDir` safety, or shell/PTY confinement, include adversarial regressions for the escape vectors that matter to that surface (for example: symlink ancestry, TOCTOU source swaps, shell expansion, nested interpreters, or other host-write escape attempts).
5. When a feature claims durable sandbox resume or snapshot behavior, prove it across a real teardown boundary: serialize state, drop the original runner-owned workspace/session, then resume from the durable payload. Include symlink-only drift/restore coverage when snapshots or fingerprints are involved, and never serialize caller-owned injected sessions into runner-managed durable state.
6. Implement the minimal runtime changes to make the tests pass while preserving workspace-root safety, runner-owned vs caller-owned session rules, and `RunState` as the public resume boundary.
7. Re-run targeted sandbox tests after each change. Add example or temp-program smoke coverage whenever the feature changes public sandbox imports or docs-visible examples.
8. Use live backend checks only when the environment supports them:
   - if Docker is unavailable and the feature requires real Docker validation, stop and return to the orchestrator
   - hosted providers default to mocked/code-parity validation unless the feature explicitly says live creds are available
9. Before handoff, prove cleanup happened for any temp dirs, temp crates, or sandbox scratch state you created.

## Example Handoff

```json
{
  "salientSummary": "Added Unix-local sandbox session resume support with manifest staging and approval-safe RunState restore, then verified the workspace stays rooted during file and shell operations.",
  "whatWasImplemented": "Created failing sandbox tests first for manifest materialization, workspace-root path rejection, RunState resume after approval interruption, and caller-owned live session reuse. Implemented the core sandbox session/state changes in agents-core and re-exported the finished public surface from the facade without placing runtime ownership there.",
  "whatWasLeftUndone": "",
  "verification": {
    "commandsRun": [
      {
        "command": "cargo test -p openai-agents-rs --test sandbox_semantics sandbox_runstate_resume_restores_workspace_after_approval -- --exact",
        "exitCode": 0,
        "observation": "Resumed run recovered the pre-interruption file and approval flow."
      },
      {
        "command": "cargo test -p openai-agents-rs --test sandbox_semantics sandbox_paths_reject_workspace_escape -- --exact",
        "exitCode": 0,
        "observation": "Filesystem helpers reject outside-root paths."
      },
      {
        "command": "cargo build -p openai-agents-rs --example sandbox_resume",
        "exitCode": 0,
        "observation": "Public sandbox example still compiles against the facade."
      }
    ],
    "interactiveChecks": [
      {
        "action": "Ran a temporary local sandbox smoke that wrote a file, paused, resumed, and read the same file back.",
        "observed": "Workspace contents persisted across resume without leaking outside the sandbox root."
      }
    ]
  },
  "tests": {
    "added": [
      {
        "file": "crates/openai-agents/tests/sandbox_semantics.rs",
        "cases": [
          {
            "name": "sandbox_runstate_resume_restores_workspace_after_approval",
            "verifies": "RunState interruption preserves enough sandbox state to resume workspace and approvals"
          },
          {
            "name": "sandbox_paths_reject_workspace_escape",
            "verifies": "sandbox file and patch operations stay rooted in the workspace"
          }
        ]
      }
    ]
  },
  "discoveredIssues": []
}
```

## When to Return to Orchestrator

- Real Docker validation is required but Docker is unavailable.
- A hosted provider feature needs live provider credentials or infrastructure that the mission does not currently have.
- The feature requires a new public import shape or run-state boundary that conflicts with `.factory/library/architecture.md`.
- Any sandbox operation appears able to escape the workspace root or leak secrets, and the issue cannot be fully fixed within the current feature.
