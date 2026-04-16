# User Testing

Validation surfaces, required tools, setup expectations, and concurrency guidance for this mission.

## Validation Surface

### Surface: Cargo runtime and package contract
- **Tooling:** shell + Cargo (`cargo test`, `cargo check`, `cargo build`, `cargo package`)
- **What gets exercised:** runtime/provider semantics, sessions, MCP, realtime, voice, sandbox implementation, packaging metadata
- **Setup:** Rust toolchain only; no browser required

### Surface: Example and temp-crate smoke checks
- **Tooling:** shell + Cargo (`cargo run`, `cargo check` in temp crates)
- **What gets exercised:** public facade imports, documented starter snippets, example commands, sandbox examples once added
- **Setup:** some examples are credential-free; others require documented prerequisites such as `OPENAI_API_KEY`, Docker, or audio input

### Surface: Docs and generated artifacts
- **Tooling:** shell scripts, markdown audits, `git diff --exit-code`
- **What gets exercised:** docs link integrity, generated `docs/llms*.txt` exports, release guide accuracy, parity mapping against upstream English docs
- **Setup:** current `docs/scripts/check_links.sh` depends on `rg`

### Surface: Registry and hosted documentation verification
- **Tooling:** shell + HTTP (`cargo info`, crates.io/docs.rs API or `curl`)
- **What gets exercised:** publish verification, version availability, docs.rs propagation
- **Setup:** publication must have happened already; repeated validation should verify instead of republishing

### Surface: Docker-backed sandbox validation
- **Tooling:** shell + Cargo + Docker daemon
- **What gets exercised:** Docker sandbox lifecycle, mounts, PTY, exposed ports, user config
- **Setup:** currently blocked because Docker is unavailable in this environment

### Surface: Hosted sandbox providers
- **Tooling:** shell + Cargo tests with mocked provider clients/state
- **What gets exercised:** code/package/config parity for hosted providers
- **Setup:** no live provider creds required by default; use mocked clients unless the user later opts into live provider validation

## Validation Concurrency

### Cargo-heavy validation
- **Examples:** `cargo test --workspace`, sandbox integration tests, `cargo build --workspace --examples`, packaging dry runs
- **Max concurrent validators:** 1
- **Why:** 10 CPU cores / 16 GiB RAM, with observed cargo-heavy runs peaking around ~0.8 GiB RSS and meaningful compile/test contention; safest path is one heavy cargo job at a time.

### Docs and shell audits
- **Examples:** markdown parity scans, manifest audits, generated-export checks, link checks
- **Max concurrent validators:** 2
- **Why:** these are lightweight compared with cargo-heavy jobs, but `rg` is missing and some scripts currently need remediation.

### HTTP verification
- **Examples:** crates.io version checks, docs.rs polling
- **Max concurrent validators:** 5
- **Why:** low CPU/RAM cost; bounded mainly by remote rate limits and propagation timing.

### Live OpenAI smoke validation
- **Examples:** selected credentialed examples or OpenAI-backed transport smokes
- **Max concurrent validators:** 1
- **Why:** network/API cost and the need to keep failures attributable to a single flow.

## Current Known Validation Blockers
- Docker daemon unavailable
- `rg` unavailable for `docs/scripts/check_links.sh`
- strict `cargo clippy --workspace --all-targets -- -D warnings` is not yet clean and should not be treated as a default hard gate until the mission fixes it
