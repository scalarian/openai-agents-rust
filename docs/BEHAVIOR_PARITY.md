# Behavior Parity

Behavior parity is tracked against the pinned Python test families in
`reference/openai-agents-python/tests` and the JS package layout in
`reference/openai-agents-js/packages`.

Allowed statuses:

- `covered`: there is Rust coverage for the family and the runtime surface is materially present
- `partial`: the runtime surface exists, but important cases are still missing
- `omitted-with-rationale`: intentionally not ported because it is Python- or environment-specific

## Family Ledger

| Python family | Status | Rust coverage | Notes |
| --- | --- | --- | --- |
| `test_agent_runner` | `covered` | `crates/agents-core/src/run.rs`, `crates/openai-agents/tests/runner_semantics.rs` | Core non-streamed runner, nested tools, resume, and default-runner behavior are covered. |
| `test_agent_runner_streamed` | `covered` | `crates/agents-core/src/run.rs`, `crates/openai-agents/tests/runner_semantics.rs` | Live streamed runs, event order, and streamed completion are covered. |
| `test_agent_runner_sync` | `covered` | `crates/agents-core/src/run.rs`, `crates/openai-agents/tests/runner_semantics.rs` | Runtime rejection and sync bridging are covered. |
| `test_max_turns` | `covered` | `crates/agents-core/src/run.rs` | Max-turn termination and handler behavior are covered in core tests. |
| `test_openai_conversations_session` | `partial` | `crates/agents-openai/src/memory.rs`, `crates/openai-agents/tests/openai_session_semantics.rs` | Runner/session continuity is covered; full remote conversations API semantics are still thinner than Python. |
| `memory/test_openai_responses_compaction_session` | `partial` | `crates/agents-openai/src/memory.rs` | Candidate selection, model-name detection, forced sanitizing compaction, and previous-response-id compaction are covered; richer remote compaction policy and replay semantics still need work. |
| `test_openai_responses` | `partial` | `crates/agents-openai/src/models.rs`, `crates/openai-agents/tests/openai_session_semantics.rs` | Payload shaping and session continuity are covered; full Responses runtime parity is still incomplete. |
| `test_openai_chatcompletions` | `partial` | `crates/agents-openai/src/models.rs` | Request shaping and tool-call parsing are covered; deeper streaming/runtime behavior still needs more coverage. |
| `mcp/test_runner_calls_mcp` | `covered` | `crates/openai-agents/tests/mcp_semantics.rs` | Non-streamed and streamed MCP tool execution through the runner are covered. |
| `mcp/test_mcp_server_manager` | `partial` | `crates/agents-core/src/mcp/manager.rs`, `crates/openai-agents/tests/mcp_semantics.rs` | Connect, reconnect, and cleanup basics are covered; richer lifecycle/resource behavior is still thinner than Python. |
| `mcp/test_mcp_resources` | `covered` | `crates/agents-core/src/mcp/server.rs`, `crates/openai-agents/tests/mcp_semantics.rs` | Connection-gated resource listing, resource template listing, and resource reads are covered for streamable HTTP MCP servers. |
| `realtime/test_runner` | `partial` | `crates/agents-realtime/src/runner.rs`, `crates/openai-agents/tests/realtime_semantics.rs` | Session creation and session commands are covered; richer model-config passthrough and tool/handoff orchestration remain thinner. |
| `realtime/test_session` | `partial` | `crates/agents-realtime/src/session.rs`, `crates/openai-agents/tests/realtime_semantics.rs` | Live event streaming and lifecycle methods are covered; full async session protocol parity remains incomplete. |
| `realtime/test_openai_realtime` | `partial` | `crates/agents-realtime/src/openai_realtime.rs` | Transport-backed placeholder events exist; real protocol parity remains incomplete. |
| `voice/test_pipeline` | `partial` | `crates/agents-voice/src/pipeline.rs`, `crates/openai-agents/tests/voice_semantics.rs` | Live streamed audio results and pipeline lifecycle are covered; richer chunking and transform behavior from Python are still missing. |
| `voice/test_workflow` | `partial` | `crates/agents-voice/src/workflow.rs`, `crates/openai-agents/tests/voice_semantics.rs` | Single-agent streamed workflow behavior is covered; full workflow state parity still needs deeper coverage. |
