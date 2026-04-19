# Workspace Analysis

## Surface
- Workspace contains six publishable crates: core, openai, realtime, voice, extensions, and the facade crate.
- `openai-agents-rs` is the main public import surface and re-exports most runtime/openai APIs plus `realtime`, `voice`, and `extensions` namespaces.
- Current docs/examples surface is materially smaller than upstream Python `v0.14.2`.

## Validation Commands Observed
- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo build --workspace --examples`
- `docs/scripts/generate_llms_exports.sh`
- `docs/scripts/check_links.sh`
- `git diff --exit-code -- docs/llms.txt docs/llms-full.txt`
- `cargo package --workspace --allow-dirty --no-verify`

## Docs and Examples Snapshot
- Rust docs: ~40 markdown docs plus curated ref pages
- Rust runnable examples: 5 current examples under `crates/openai-agents/examples`
- Tests are concentrated in crate-level tests and facade semantics tests

## Risks
- Sandbox parity is the largest code gap.
- Docs/examples parity will require broad editorial and example work.
- Publication is currently manual and dependency-order sensitive.
