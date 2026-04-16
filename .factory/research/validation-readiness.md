# Validation Readiness

## Dry Run Results
- `cargo fmt --all --check` passes
- `cargo test --workspace` passes
- `cargo build --workspace --examples` passes
- `docs/scripts/generate_llms_exports.sh` rewrites tracked exports on this repo today
- `docs/scripts/check_links.sh` depends on `rg`, which is currently unavailable in this environment
- `cargo package --workspace --allow-dirty --no-verify` passes
- `cargo clippy --workspace --all-targets -- -D warnings` is not yet clean

## External Requirements
- `OPENAI_API_KEY` for live OpenAI example/integration validation
- `CARGO_REGISTRY_TOKEN` for crates.io publication
- Docker daemon for Docker-backed sandbox validation

## Resource Profile
- Machine: 10 CPU cores, 16 GiB RAM
- Safe concurrency for cargo-heavy validation: 1
- Lightweight HTTP verification can run in parallel after heavy jobs finish

## Current Blockers
- Docker unavailable
- `rg` unavailable
- generated docs exports are currently dirty after regeneration
