# Python Release Parity Research

## Current Surface
- Upstream Python `v0.14.2` foregrounds sandbox agents as a core SDK concept.
- Public surface includes core agents/runtime, tools, handoffs, sessions, tracing, realtime, voice, provider integrations, and a large sandbox ecosystem.

## Recent Changes
- Sandbox Agents landed in the recent upstream release train with new docs, examples, and API surface.
- README shifted to present sandbox agents as a first-run concept.
- `main` is only slightly ahead of `v0.14.2`, so the mission should track the tagged release rather than unreleased branch-only changes.

## Release Signals
- Upstream cut `v0.14.0`, `v0.14.1`, and `v0.14.2` around and after the sandbox release.
- Mission parity should target the latest tagged release, `v0.14.2`.

## Recommended Buckets
- runtime/provider parity
- sandbox local/runtime parity
- Docker sandbox and mounts
- hosted sandbox providers
- README/docs/examples parity
- release/publish hardening
