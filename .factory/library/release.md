# Release

Release-specific facts and expectations for this mission.

## Publish Order
1. `openai-agents-core-rs`
2. `openai-agents-openai-rs`
3. `openai-agents-realtime-rs`
4. `openai-agents-voice-rs`
5. `openai-agents-extensions-rs`
6. `openai-agents-rs`

## Required Pre-Publish Checks
- formatting and workspace tests
- example compilation
- docs maintenance and generated export cleanliness
- packaging dry runs for all crates
- version and inter-crate dependency alignment across manifests

## Publish/Rerun Guidance
- Real publish is only for the explicit publish feature.
- Validation and reruns should verify crates.io/docs.rs state instead of attempting to republish versions that already exist.
- Do not expose registry tokens in logs, docs, commits, or handoffs.

## Current Release Caveats
- `docs/scripts/check_links.sh` currently depends on `rg`
- Docker-backed validation is currently blocked in this environment
- strict clippy is not yet a default gate
