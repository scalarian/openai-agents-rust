# Examples

Use this page when you want runnable starting points instead of longer conceptual guides.

## Runnable Examples

All runnable examples live in `crates/openai-agents/examples`.

| Example | What it covers | File |
| --- | --- | --- |
| agents as tools | specialist agents exposed as callable tools | [agents_as_tools.rs](../crates/openai-agents/examples/agents_as_tools.rs) |
| agents as tools conditional | dynamically enabled agent tools | [agents_as_tools_conditional.rs](../crates/openai-agents/examples/agents_as_tools_conditional.rs) |
| agents as tools streaming | stream events emitted by a nested agent tool | [agents_as_tools_streaming.rs](../crates/openai-agents/examples/agents_as_tools_streaming.rs) |
| agents as tools structured | structured input for agent-as-tool calls | [agents_as_tools_structured.rs](../crates/openai-agents/examples/agents_as_tools_structured.rs) |
| agent lifecycle | per-agent hooks around tools, handoffs, and final output | [agent_lifecycle.rs](../crates/openai-agents/examples/agent_lifecycle.rs) |
| agent lifecycle example | upstream-named agent lifecycle entry point | [agent_lifecycle_example.rs](../crates/openai-agents/examples/agent_lifecycle_example.rs) |
| advanced sqlite session example | upstream-named advanced SQLite session entry point | [advanced_sqlite_session_example.rs](../crates/openai-agents/examples/advanced_sqlite_session_example.rs) |
| advanced sqlite session | extension SQLite session with custom table names and tool history | [advanced_sqlite_session.rs](../crates/openai-agents/examples/advanced_sqlite_session.rs) |
| async sqlite session | async-friendly extension wrapper over SQLite-backed session memory | [async_sqlite_session.rs](../crates/openai-agents/examples/async_sqlite_session.rs) |
| AnyLLM auto | upstream-named multi-provider AnyLLM routing entry point | [any_llm_auto.rs](../crates/openai-agents/examples/any_llm_auto.rs) |
| AnyLLM provider | upstream-named AnyLLM provider entry point | [any_llm_provider.rs](../crates/openai-agents/examples/any_llm_provider.rs) |
| apply patch | approval-gated local patch tool workflow | [apply_patch.rs](../crates/openai-agents/examples/apply_patch.rs) |
| auto mode | deterministic fallback inputs and confirmations for automated example runs | [auto_mode.rs](../crates/openai-agents/examples/auto_mode.rs) |
| basic run | smallest end-to-end call | [basic_run.rs](../crates/openai-agents/examples/basic_run.rs) |
| code interpreter | hosted code interpreter tool configuration and streamed call items | [code_interpreter.rs](../crates/openai-agents/examples/code_interpreter.rs) |
| codex | experimental Codex tool configuration for read-only workspace inspection | [codex.rs](../crates/openai-agents/examples/codex.rs) |
| compaction session example | upstream-named compaction session entry point | [compaction_session_example.rs](../crates/openai-agents/examples/compaction_session_example.rs) |
| compaction session stateless example | upstream-named stateless compaction entry point | [compaction_session_stateless_example.rs](../crates/openai-agents/examples/compaction_session_stateless_example.rs) |
| compaction session | automatic and manual OpenAI Responses session compaction | [compaction_session.rs](../crates/openai-agents/examples/compaction_session.rs) |
| compaction session stateless | auto compaction for `store=false` Responses runs | [compaction_session_stateless.rs](../crates/openai-agents/examples/compaction_session_stateless.rs) |
| custom example agent | upstream-named per-agent custom model entry point | [custom_example_agent.rs](../crates/openai-agents/examples/custom_example_agent.rs) |
| custom example global | upstream-named global custom provider entry point | [custom_example_global.rs](../crates/openai-agents/examples/custom_example_global.rs) |
| custom example provider | upstream-named custom model provider entry point | [custom_example_provider.rs](../crates/openai-agents/examples/custom_example_provider.rs) |
| custom agent model | per-agent model name resolved by a custom provider | [custom_agent_model.rs](../crates/openai-agents/examples/custom_agent_model.rs) |
| custom model provider | per-run custom model provider selection | [custom_model_provider.rs](../crates/openai-agents/examples/custom_model_provider.rs) |
| database session | database-session extension using an in-memory SQLite URL | [database_session.rs](../crates/openai-agents/examples/database_session.rs) |
| Dapr session example | upstream-named Dapr session entry point | [dapr_session_example.rs](../crates/openai-agents/examples/dapr_session_example.rs) |
| dapr session | Dapr state-store backed extension session with graceful availability check | [dapr_session.rs](../crates/openai-agents/examples/dapr_session.rs) |
| default model provider | global default runner model provider | [default_model_provider.rs](../crates/openai-agents/examples/default_model_provider.rs) |
| deterministic | upstream-named deterministic workflow entry point | [deterministic.rs](../crates/openai-agents/examples/deterministic.rs) |
| deterministic flow | multi-step agent workflow with an explicit gate | [deterministic_flow.rs](../crates/openai-agents/examples/deterministic_flow.rs) |
| dynamic system prompt | per-run agent instructions | [dynamic_system_prompt.rs](../crates/openai-agents/examples/dynamic_system_prompt.rs) |
| encrypted session example | upstream-named encrypted session entry point | [encrypted_session_example.rs](../crates/openai-agents/examples/encrypted_session_example.rs) |
| encrypted session | encrypted session wrapper over SQLite storage | [encrypted_session.rs](../crates/openai-agents/examples/encrypted_session.rs) |
| file session | file-backed extension session rehydrated by session id | [file_session.rs](../crates/openai-agents/examples/file_session.rs) |
| file search | hosted file search with vector store options and included results | [file_search.rs](../crates/openai-agents/examples/file_search.rs) |
| file HITL example | upstream-named file session HITL entry point | [file_hitl_example.rs](../crates/openai-agents/examples/file_hitl_example.rs) |
| function tools | typed local tools with JSON-schema inputs | [function_tools.rs](../crates/openai-agents/examples/function_tools.rs) |
| forcing tool use | required tool choice and stop-on-tool behavior | [forcing_tool_use.rs](../crates/openai-agents/examples/forcing_tool_use.rs) |
| handoffs | control transfer between agents | [handoffs.rs](../crates/openai-agents/examples/handoffs.rs) |
| hello world | upstream-style haiku response example | [hello_world.rs](../crates/openai-agents/examples/hello_world.rs) |
| hello world GPT-5 | GPT-5 reasoning and verbosity model settings | [hello_world_gpt_5.rs](../crates/openai-agents/examples/hello_world_gpt_5.rs) |
| hello world GPT-OSS | optional local GPT-OSS Chat Completions provider example | [hello_world_gpt_oss.rs](../crates/openai-agents/examples/hello_world_gpt_oss.rs) |
| hosted MCP connectors | hosted MCP connector payload configuration | [hosted_mcp_connectors.rs](../crates/openai-agents/examples/hosted_mcp_connectors.rs) |
| hosted MCP human in the loop | approval interruption and resume for hosted MCP calls | [hosted_mcp_human_in_the_loop.rs](../crates/openai-agents/examples/hosted_mcp_human_in_the_loop.rs) |
| hosted MCP on approval | hosted MCP approval callback flow | [hosted_mcp_on_approval.rs](../crates/openai-agents/examples/hosted_mcp_on_approval.rs) |
| hosted MCP simple | upstream-named hosted MCP simple entry point | [hosted_mcp_simple.rs](../crates/openai-agents/examples/hosted_mcp_simple.rs) |
| HITL session scenario | approval resume across memory, file, and OpenAI sessions | [hitl_session_scenario.rs](../crates/openai-agents/examples/hitl_session_scenario.rs) |
| hosted mcp | hosted MCP tool payload configuration for Responses models | [hosted_mcp.rs](../crates/openai-agents/examples/hosted_mcp.rs) |
| human in the loop | approval-required tool calls with saved run state | [human_in_the_loop.rs](../crates/openai-agents/examples/human_in_the_loop.rs) |
| human in the loop custom rejection | custom model-visible rejection text for denied tool calls | [human_in_the_loop_custom_rejection.rs](../crates/openai-agents/examples/human_in_the_loop_custom_rejection.rs) |
| human in the loop stream | streamed approval interruption and streamed resume | [human_in_the_loop_stream.rs](../crates/openai-agents/examples/human_in_the_loop_stream.rs) |
| image generator | hosted image generation tool configuration and result decoding | [image_generator.rs](../crates/openai-agents/examples/image_generator.rs) |
| image tool output | function tool returning an image output item | [image_tool_output.rs](../crates/openai-agents/examples/image_tool_output.rs) |
| input guardrails | upstream-named input guardrail entry point | [input_guardrails.rs](../crates/openai-agents/examples/input_guardrails.rs) |
| input guardrail | local tripwire handling before model execution | [input_guardrail.rs](../crates/openai-agents/examples/input_guardrail.rs) |
| lifecycle | run-level and agent-level lifecycle hooks | [lifecycle.rs](../crates/openai-agents/examples/lifecycle.rs) |
| lifecycle example | upstream-named lifecycle entry point | [lifecycle_example.rs](../crates/openai-agents/examples/lifecycle_example.rs) |
| lifecycle hooks | run and agent lifecycle callbacks | [lifecycle_hooks.rs](../crates/openai-agents/examples/lifecycle_hooks.rs) |
| LiteLLM auto | upstream-named multi-provider LiteLLM routing entry point | [litellm_auto.rs](../crates/openai-agents/examples/litellm_auto.rs) |
| LiteLLM provider | upstream-named LiteLLM provider entry point | [litellm_provider.rs](../crates/openai-agents/examples/litellm_provider.rs) |
| llm as a judge | iterative generator and evaluator agent loop | [llm_as_a_judge.rs](../crates/openai-agents/examples/llm_as_a_judge.rs) |
| local file | local file input item encoded as data URL content | [local_file.rs](../crates/openai-agents/examples/local_file.rs) |
| local image | local image input item encoded as data URL content | [local_image.rs](../crates/openai-agents/examples/local_image.rs) |
| memory session HITL example | upstream-named memory session HITL entry point | [memory_session_hitl_example.rs](../crates/openai-agents/examples/memory_session_hitl_example.rs) |
| memory session | session-backed follow-up turns | [memory_session.rs](../crates/openai-agents/examples/memory_session.rs) |
| mcp filesystem example | upstream-named MCP filesystem entry point | [mcp_filesystem_example.rs](../crates/openai-agents/examples/mcp_filesystem_example.rs) |
| mcp filesystem | MCP server tool discovery and local tool calls | [mcp_filesystem.rs](../crates/openai-agents/examples/mcp_filesystem.rs) |
| mcp get all MCP tools example | upstream-named MCP tool-prefetch entry point | [mcp_get_all_mcp_tools_example.rs](../crates/openai-agents/examples/mcp_get_all_mcp_tools_example.rs) |
| mcp git example | upstream-named MCP git entry point | [mcp_git_example.rs](../crates/openai-agents/examples/mcp_git_example.rs) |
| mcp manager example | upstream-named MCP manager entry point | [mcp_manager_example.rs](../crates/openai-agents/examples/mcp_manager_example.rs) |
| mcp prompt server | MCP prompt discovery and prompt-driven agent instructions | [mcp_prompt_server.rs](../crates/openai-agents/examples/mcp_prompt_server.rs) |
| mcp SSE example | upstream-named MCP SSE transport entry point | [mcp_sse_example.rs](../crates/openai-agents/examples/mcp_sse_example.rs) |
| mcp SSE remote example | upstream-named MCP remote SSE transport entry point | [mcp_sse_remote_example.rs](../crates/openai-agents/examples/mcp_sse_remote_example.rs) |
| mcp streamable HTTP custom client example | upstream-named MCP streamable HTTP custom client entry point | [mcp_streamablehttp_custom_client_example.rs](../crates/openai-agents/examples/mcp_streamablehttp_custom_client_example.rs) |
| mcp streamable HTTP example | upstream-named MCP streamable HTTP entry point | [mcp_streamablehttp_example.rs](../crates/openai-agents/examples/mcp_streamablehttp_example.rs) |
| mcp streamable HTTP remote example | upstream-named MCP remote streamable HTTP entry point | [mcp_streamable_http_remote_example.rs](../crates/openai-agents/examples/mcp_streamable_http_remote_example.rs) |
| mcp tool filter example | upstream-named MCP tool filter entry point | [mcp_tool_filter_example.rs](../crates/openai-agents/examples/mcp_tool_filter_example.rs) |
| mcp tool filter | static MCP tool filtering before model-visible tool discovery | [mcp_tool_filter.rs](../crates/openai-agents/examples/mcp_tool_filter.rs) |
| message filter | handoff input filtering that removes tool history | [message_filter.rs](../crates/openai-agents/examples/message_filter.rs) |
| message filter streaming | streamed handoff input filtering that removes tool history | [message_filter_streaming.rs](../crates/openai-agents/examples/message_filter_streaming.rs) |
| MongoDB session example | upstream-named MongoDB session entry point | [mongodb_session_example.rs](../crates/openai-agents/examples/mongodb_session_example.rs) |
| mongodb session | MongoDB-backed extension session with shared client isolation | [mongodb_session.rs](../crates/openai-agents/examples/mongodb_session.rs) |
| non-strict output type | structured output validation with non-strict and custom schemas | [non_strict_output_type.rs](../crates/openai-agents/examples/non_strict_output_type.rs) |
| OpenAI session example | upstream-named OpenAI session entry point | [openai_session_example.rs](../crates/openai-agents/examples/openai_session_example.rs) |
| OpenAI session HITL example | upstream-named OpenAI session HITL entry point | [openai_session_hitl_example.rs](../crates/openai-agents/examples/openai_session_hitl_example.rs) |
| openai session | OpenAI conversation-aware session continuation metadata | [openai_session.rs](../crates/openai-agents/examples/openai_session.rs) |
| output guardrails | upstream-named output guardrail entry point | [output_guardrails.rs](../crates/openai-agents/examples/output_guardrails.rs) |
| output guardrail | final output tripwire handling | [output_guardrail.rs](../crates/openai-agents/examples/output_guardrail.rs) |
| parallelization | run multiple agent calls concurrently and pick the best result | [parallelization.rs](../crates/openai-agents/examples/parallelization.rs) |
| previous response id | continue a Responses API conversation by id | [previous_response_id.rs](../crates/openai-agents/examples/previous_response_id.rs) |
| prompt template | static and dynamic reusable prompt config | [prompt_template.rs](../crates/openai-agents/examples/prompt_template.rs) |
| reasoning content | reasoning items from normal and streamed runner output | [reasoning_content.rs](../crates/openai-agents/examples/reasoning_content.rs) |
| reasoning content GPT OSS stream | optional local GPT-OSS reasoning stream smoke example | [reasoning_content_gpt_oss_stream.rs](../crates/openai-agents/examples/reasoning_content_gpt_oss_stream.rs) |
| reasoning content runner example | upstream-named reasoning content Runner entry point | [reasoning_content_runner_example.rs](../crates/openai-agents/examples/reasoning_content_runner_example.rs) |
| realtime app agent | realtime airline customer-service agent tools and handoffs | [realtime_app_agent.rs](../crates/openai-agents/examples/realtime_app_agent.rs) |
| sandbox workspace | local sandbox workspace preparation and shell confinement | [sandbox_workspace.rs](../crates/openai-agents/examples/sandbox_workspace.rs) |
| sandbox Blaxel extension | Blaxel hosted sandbox client feature-gate and session lifecycle | [sandbox_blaxel_extension.rs](../crates/openai-agents/examples/sandbox_blaxel_extension.rs) |
| sandbox Cloudflare extension | Cloudflare hosted sandbox client feature-gate and session lifecycle | [sandbox_cloudflare_extension.rs](../crates/openai-agents/examples/sandbox_cloudflare_extension.rs) |
| sandbox Daytona extension | Daytona hosted sandbox client feature-gate and session lifecycle | [sandbox_daytona_extension.rs](../crates/openai-agents/examples/sandbox_daytona_extension.rs) |
| sandbox E2B extension | E2B hosted sandbox client feature-gate and session lifecycle | [sandbox_e2b_extension.rs](../crates/openai-agents/examples/sandbox_e2b_extension.rs) |
| sandbox Modal extension | Modal hosted sandbox client feature-gate and session lifecycle | [sandbox_modal_extension.rs](../crates/openai-agents/examples/sandbox_modal_extension.rs) |
| sandbox Runloop extension | Runloop hosted sandbox client feature-gate and session lifecycle | [sandbox_runloop_extension.rs](../crates/openai-agents/examples/sandbox_runloop_extension.rs) |
| sandbox Vercel extension | Vercel hosted sandbox client feature-gate and session lifecycle | [sandbox_vercel_extension.rs](../crates/openai-agents/examples/sandbox_vercel_extension.rs) |
| SQLAlchemy session example | upstream-named database session entry point | [sqlalchemy_session_example.rs](../crates/openai-agents/examples/sqlalchemy_session_example.rs) |
| sqlite session example | upstream-named SQLite session entry point | [sqlite_session_example.rs](../crates/openai-agents/examples/sqlite_session_example.rs) |
| sqlite session | SQLite-backed conversation memory | [sqlite_session.rs](../crates/openai-agents/examples/sqlite_session.rs) |
| stream function call args | streamed tool-call argument items | [stream_function_call_args.rs](../crates/openai-agents/examples/stream_function_call_args.rs) |
| stream items | streamed run-item events for tool calls and messages | [stream_items.rs](../crates/openai-agents/examples/stream_items.rs) |
| stream text | streamed message text output | [stream_text.rs](../crates/openai-agents/examples/stream_text.rs) |
| stream WebSocket | Responses WebSocket streaming with approvals and previous response ids | [stream_ws.rs](../crates/openai-agents/examples/stream_ws.rs) |
| streamed run | live events and completion | [streamed_run.rs](../crates/openai-agents/examples/streamed_run.rs) |
| streaming guardrails | incremental output checks during streaming | [streaming_guardrails.rs](../crates/openai-agents/examples/streaming_guardrails.rs) |
| tool guardrails | function-tool input and output guardrails | [tool_guardrails.rs](../crates/openai-agents/examples/tool_guardrails.rs) |
| tool search | hosted tool search with namespaces and top-level deferred tools | [tool_search.rs](../crates/openai-agents/examples/tool_search.rs) |
| tools | upstream-style basic function tool example | [tools.rs](../crates/openai-agents/examples/tools.rs) |
| usage tracking | token usage from a completed run | [usage_tracking.rs](../crates/openai-agents/examples/usage_tracking.rs) |
| realtime session | long-lived realtime interaction | [realtime_session.rs](../crates/openai-agents/examples/realtime_session.rs) |
| realtime Twilio media stream | Twilio media stream event bridge for realtime sessions | [realtime_twilio_media_stream.rs](../crates/openai-agents/examples/realtime_twilio_media_stream.rs) |
| realtime Twilio SIP | Twilio SIP-oriented realtime runner example | [realtime_twilio_sip.rs](../crates/openai-agents/examples/realtime_twilio_sip.rs) |
| remote image | remote image input item by URL | [remote_image.rs](../crates/openai-agents/examples/remote_image.rs) |
| remote pdf | remote PDF input item by URL | [remote_pdf.rs](../crates/openai-agents/examples/remote_pdf.rs) |
| redis session example | upstream-named Redis session entry point | [redis_session_example.rs](../crates/openai-agents/examples/redis_session_example.rs) |
| redis session | Redis-backed extension session with graceful availability check | [redis_session.rs](../crates/openai-agents/examples/redis_session.rs) |
| retry | runner-managed model retry settings | [retry.rs](../crates/openai-agents/examples/retry.rs) |
| retry LiteLLM | LiteLLM retry policy and backoff settings | [retry_litellm.rs](../crates/openai-agents/examples/retry_litellm.rs) |
| routing | streamed triage handoff to a specialist agent | [routing.rs](../crates/openai-agents/examples/routing.rs) |
| web search | hosted web search tool configuration | [web_search.rs](../crates/openai-agents/examples/web_search.rs) |
| web search filters | hosted web search domain filters and source includes | [web_search_filters.rs](../crates/openai-agents/examples/web_search_filters.rs) |
| voice pipeline | STT -> workflow -> TTS flow | [voice_pipeline.rs](../crates/openai-agents/examples/voice_pipeline.rs) |
| voice static | non-streamed voice pipeline example | [voice_static.rs](../crates/openai-agents/examples/voice_static.rs) |
| voice streamed | streamed voice pipeline example | [voice_streamed.rs](../crates/openai-agents/examples/voice_streamed.rs) |

## Read By Goal

- first program: [quickstart.md](quickstart.md)
- tools: [tools.md](tools.md)
- sessions: [sessions/README.md](sessions/README.md)
- streaming: [streaming.md](streaming.md)
- realtime: [realtime/README.md](realtime/README.md)
- voice: [voice/README.md](voice/README.md)

## Example Design Rules

Good examples in this repo should:

- compile
- stay short
- focus on one capability
- link back to the canonical docs page

If an example needs five unrelated setup steps, it should probably become a guide instead.
