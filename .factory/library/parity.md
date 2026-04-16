# Parity Plan

Mission-specific parity guidance for syncing Rust to `openai/openai-agents-python` `main`.

## Source of Truth
- Upstream repo: `openai/openai-agents-python`
- Branch baseline: current `main`
- Local clone used for comparison: `/tmp/openai-agents-python-upstream-nph6i2p0`

## Confirmed Scope
- Full current-`main` parity target
- English docs only
- Publish all Rust crates
- Hosted sandbox providers are required at the code/package/config parity layer, but live provider validation is out of scope unless extra credentials are later supplied

## Highest-Risk Parity Buckets
- Sandbox runtime surface (`SandboxAgent`, manifests, local sessions, resume/snapshots, capabilities, memory)
- Docker sandbox support and mount semantics
- Hosted sandbox provider feature gates and public surface
- README/docs/examples breadth and onboarding flow parity
- Release hardening and crates.io publication flow

## Milestone Map
1. runtime/provider parity
2. local sandbox MVP
3. sandbox resume/state
4. sandbox composition
5. Docker and mount parity
6. hosted sandbox provider parity
7. English docs/examples/release sweep

## Worker Guidance
- Prefer parity to upstream behavior and user-visible surface over literal file-by-file translation.
- Preserve the Rust workspace layering: core runtime in `agents-core`, OpenAI specifics in `agents-openai`, hosted/optional providers in `agents-extensions`, thin facade in `openai-agents`.
- When in doubt, validate parity against upstream tests/docs/examples for the same surface before changing public Rust behavior.
