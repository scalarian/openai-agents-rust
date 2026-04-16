---
name: rust-docs-release-worker
description: Implement and verify README, docs, examples, packaging, release-flow, and publish work for the Rust SDK.
---

# Rust Docs and Release Worker

NOTE: Startup and cleanup are handled by `worker-base`. This skill defines the WORK PROCEDURE.

## When to Use This Skill

Use this skill for features that update README/docs/reference/examples, docs maintenance scripts, packaging metadata, release instructions, crates.io publication, or post-publish verification.

## Required Skills

None.

## Work Procedure

1. Read `mission.md`, mission `AGENTS.md`, `.factory/library/parity.md`, `.factory/library/release.md`, `.factory/library/user-testing.md`, and the feature’s `fulfills` assertions before editing files.
2. Update docs/examples only to match shipped behavior. Do not promise capabilities that are not yet implemented.
3. When a feature changes a public claim, update all linked surfaces together: `README.md`, `docs/index.md`, topic docs, examples index, and any example file or reference page that the claim depends on.
4. For example changes, verify both inventory and command correctness:
   - build the affected examples
   - smoke-run the credential-free ones
   - mark credentialed or environment-dependent ones with prerequisites before the command block
5. For release or packaging work, validate manifests and dry runs in dependency order before attempting any real publish.
6. Real crates.io publish is only for explicit publish features. Before publishing, check local credentials exist, verify the target version is not already published, and avoid printing tokens or other secrets.
7. After publish, verify crates.io and docs.rs visibility instead of re-running publish commands.

## Example Handoff

```json
{
  "salientSummary": "Updated the Rust README/docs/example index for sandbox parity, fixed the docs maintenance path, then completed package dry runs and verified crates.io/docs.rs state for the new release.",
  "whatWasImplemented": "Aligned README, docs index, sandbox landing pages, and examples index with the shipped Rust sandbox surface; corrected the docs maintenance workflow so generated exports stay clean; updated manifests and per-crate release metadata; and executed dependency-ordered packaging and registry verification for all six crates.",
  "whatWasLeftUndone": "",
  "verification": {
    "commandsRun": [
      {
        "command": "cargo build --workspace --examples",
        "exitCode": 0,
        "observation": "All documented examples compile."
      },
      {
        "command": "docs/scripts/generate_llms_exports.sh && git diff --exit-code -- docs/llms.txt docs/llms-full.txt",
        "exitCode": 0,
        "observation": "Generated docs exports are up to date."
      },
      {
        "command": "cargo package --workspace --allow-dirty --no-verify",
        "exitCode": 0,
        "observation": "All workspace crates package successfully."
      }
    ],
    "interactiveChecks": [
      {
        "action": "Smoke-ran the credential-free README example commands and audited prerequisite notes for credentialed examples.",
        "observed": "Public docs commands are correct and every environment-dependent example is clearly marked."
      }
    ]
  },
  "tests": {
    "added": [
      {
        "file": "docs/examples.md",
        "cases": [
          {
            "name": "sandbox examples index entries",
            "verifies": "every shipped sandbox capability has a discoverable example or explicit note"
          }
        ]
      }
    ]
  },
  "discoveredIssues": []
}
```

## When to Return to Orchestrator

- Real publish is required but credentials or registry access are unavailable.
- A docs change depends on implementation work that is not yet shipped.
- Docker/provider prerequisites needed for a documented example cannot be satisfied in the current environment.
- A release-flow change would require changing mission boundaries or publishing a version that is already live without a coordinated version bump.
