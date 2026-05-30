//! Public facade for the Rust port of the OpenAI Agents SDK.

pub use agents_core::VERSION as __version__;
pub use agents_core::{
    Agent, AgentAsToolInput, AgentAsToolOptions, AgentBase, AgentBuilder, AgentHookContext,
    AgentHooks, AgentOutputSchema, AgentOutputSchemaBase, AgentRunner, AgentSpanData,
    AgentToolFailureFormatter, AgentToolInvocation, AgentToolOutputExtractor, AgentToolRunResult,
    AgentToolStreamEvent, AgentToolStreamHandler, AgentUpdatedStreamEvent, AgentsError,
    AgentsException, ApplyPatchEditor, ApplyPatchOperation, ApplyPatchResult, ApplyPatchTool,
    ApprovalRecord, AsyncComputer, Button, CURRENT_RUN_STATE_SCHEMA_VERSION, CallModelData,
    CompactionItem, Computer, ComputerProvider, ComputerTool, CustomSpanData,
    DEFAULT_CONVERSATION_HISTORY_END, DEFAULT_CONVERSATION_HISTORY_START, DEFAULT_MAX_TURNS,
    DebugSettings, DefaultOpenAIApi, DefaultOpenAIResponsesTransport, Dir, DocstringStyle,
    DynamicInstructionsFunction, DynamicPromptFunction, Editor, Environment, File, FunctionSchema,
    FunctionSpanData, FunctionTool, FunctionToolLookupKey, FunctionToolResult,
    GenerateDynamicPromptData, GenerationSpanData, GuardrailFunctionOutput, GuardrailSpanData,
    Handoff, HandoffBuilder, HandoffCallItem, HandoffHistoryMapper, HandoffInputData,
    HandoffInputFilter, HandoffOutputItem, HandoffSpanData, HostedMCPTool, InputGuardrail,
    InputGuardrailResult, InputGuardrailTripwireTriggered, InputItem, ItemHelpers, LOGGER_TARGET,
    LocalDir, LocalSandboxPtySession, LocalSandboxSession, LocalShellCommandRequest,
    LocalShellExecutor, LocalShellOutput, LocalShellTool, MCPApprovalRequestItem,
    MCPApprovalResponseItem, MCPBlobResourceContents, MCPGetPromptResult, MCPListPromptsResult,
    MCPListResourceTemplatesResult, MCPListResourcesResult, MCPListToolsItem, MCPListToolsSpanData,
    MCPPrompt, MCPPromptArgument, MCPPromptContent, MCPPromptMessage, MCPPromptTextContent,
    MCPReadResourceResult, MCPResource, MCPResourceContents, MCPResourceTemplate, MCPServer,
    MCPServerManager, MCPServerSse, MCPServerSseParams, MCPServerStdio, MCPServerStdioParams,
    MCPServerStreamableHttp, MCPServerStreamableHttpParams, MCPTextResourceContents, MCPTool,
    MCPToolAnnotations, MCPToolApprovalFunction, MCPToolApprovalFunctionResult,
    MCPToolApprovalRequest, MCPToolMetaContext, MCPToolMetaResolver, MCPTransportAuth,
    MCPTransportClient, MCPTransportClientBuilder, MCPTransportClientConfig, MCPTransportKind,
    MCPUtil, Manifest, ManifestEntry, MaxTurnsExceeded, MaybeAwaitable, MemorySession,
    MessageOutputItem, Model, ModelBehaviorError, ModelInputData, ModelProvider, ModelRefusalError,
    ModelRefusalHandler, ModelRefusalHandlerInput, ModelRequest, ModelResponse, ModelRetryAdvice,
    ModelRetryAdviceRequest, ModelRetryBackoffSettings, ModelRetryNormalizedError,
    ModelRetrySettings, ModelSettings, ModelTracing, MultiProvider, MultiProviderMap,
    MultiProviderOpenAIPrefixMode, MultiProviderUnknownPrefixMode, OpenAIConversationAwareSession,
    OpenAIConversationSessionState, OpenAIResponsesCompactionArgs,
    OpenAIResponsesCompactionAwareSession, OutputGuardrail, OutputGuardrailResult,
    OutputGuardrailTripwireTriggered, OutputItem, OutputSchemaDefinition, PreparedSandboxRun,
    Prompt, PromptSpec, PromptUtil, RawResponsesStreamEvent, ReasoningItem, ReasoningItemIdPolicy,
    ReasoningSettings, RequireApprovalObject, RequireApprovalPolicy, RequireApprovalSetting,
    RequireApprovalToolList, RequireApprovalValue, ResolvedToolInput, ResponseSpanData, Result,
    RetryDecision, RetryPolicy, RetryPolicyContext, RunConfig, RunContext, RunContextWrapper,
    RunErrorData, RunErrorDetails, RunErrorHandler, RunErrorHandlerInput, RunErrorHandlerResult,
    RunErrorHandlers, RunHooks, RunInterruption, RunInterruptionKind, RunItem, RunItemStreamEvent,
    RunOptions, RunResult, RunResultStreaming, RunState, RunStateContextSnapshot, Runner,
    SQLiteSession, SandboxAgent, SandboxAgentBuilder, SandboxCapability, SandboxConcurrencyLimits,
    SandboxPathGrant, SandboxRunConfig, SdkConfig, Session, Session as SessionABC,
    SessionInputCallback, SessionSettings, SharedAgentHooks, SharedRunHooks, ShellActionRequest,
    ShellCallData, ShellCallOutcome, ShellCommandOutput, ShellCommandRequest, ShellExecutor,
    ShellResult, ShellTool, ShellToolContainerAutoEnvironment, ShellToolContainerNetworkPolicy,
    ShellToolContainerNetworkPolicyAllowlist, ShellToolContainerNetworkPolicyDisabled,
    ShellToolContainerNetworkPolicyDomainSecret, ShellToolContainerReferenceEnvironment,
    ShellToolContainerSkill, ShellToolEnvironment, ShellToolHostedEnvironment,
    ShellToolInlineSkill, ShellToolInlineSkillSource, ShellToolLocalEnvironment,
    ShellToolLocalSkill, ShellToolSkillReference, Span, SpanData, SpanError, SpeechGroupSpanData,
    SpeechSpanData, StaticTool, StopAtTools, StreamEvent, StructuredInputSchemaInfo,
    StructuredToolInputBuilder, TResponseInputItem, TaskSpanData, ToInputListMode, Tool,
    ToolApprovalFunction, ToolApprovalItem, ToolCall, ToolCallItem, ToolCallOutputItem,
    ToolContext, ToolDefinition, ToolErrorFormatter, ToolErrorFormatterArgs, ToolExecutionConfig,
    ToolFilter, ToolFilterCallable, ToolFilterContext, ToolFilterStatic, ToolGuardrailBehavior,
    ToolGuardrailFunctionOutput, ToolInputGuardrail, ToolInputGuardrailData,
    ToolInputGuardrailResult, ToolInputGuardrailTripwireTriggered, ToolNotFoundBehavior,
    ToolOrigin, ToolOriginType, ToolOutput, ToolOutputFileContent, ToolOutputGuardrail,
    ToolOutputGuardrailData, ToolOutputGuardrailResult, ToolOutputGuardrailTripwireTriggered,
    ToolOutputImage, ToolOutputText, ToolSearchCallItem, ToolSearchOutputItem, ToolTimeoutError,
    ToolUseBehavior, ToolsToFinalOutputFunction, ToolsToFinalOutputResult, Trace, TracingProcessor,
    TranscriptionSpanData, TurnSpanData, Usage, UserError, VERSION, add_trace_processor,
    agent_span, apply_diff, attach_error_to_current_span, attach_error_to_span,
    build_function_tool_lookup_map, consume_agent_tool_run_result, create_static_tool_filter,
    custom_span, debug_flag_enabled, default_handoff_history_mapper,
    default_openai_responses_transport, default_tool_error_function, default_tool_input_builder,
    default_tracing_export_api_key, dispose_resolved_computers, dont_log_model_data,
    dont_log_tool_data, drop_agent_tool_run_result, enable_verbose_stdout_logging,
    ensure_strict_json_schema, evaluate_needs_approval_setting, flush_traces, function_span,
    function_tool, gen_group_id, gen_span_id, gen_trace_id, generation_span,
    get_agent_tool_state_scope, get_conversation_history_wrappers, get_current_span,
    get_current_trace, get_default_agent_runner, get_default_model, get_default_model_settings,
    get_function_tool_approval_keys, get_function_tool_lookup_key,
    get_function_tool_lookup_key_for_call, get_function_tool_lookup_key_for_definition,
    get_function_tool_lookup_keys, get_function_tool_origin, get_function_tool_qualified_name,
    get_function_tool_trace_name, get_tool_call_name, get_tool_call_namespace,
    get_tool_call_qualified_name, get_tool_call_trace_name, gpt_5_reasoning_settings_required,
    guardrail_span, handoff, handoff_span, input_guardrail, is_gpt_5_default,
    is_openai_conversation_aware_session, is_openai_responses_compaction_aware_session,
    is_reserved_synthetic_tool_namespace, load_dont_log_model_data, load_dont_log_tool_data,
    mcp_tools_span, nest_handoff_history, nest_handoff_history_with_mapper, noop_coroutine,
    output_guardrail, peek_agent_tool_run_result, prepare_sandbox_run, pretty_print_result,
    pretty_print_run_error_details, pretty_print_run_result_streaming,
    record_agent_tool_run_result, reset_conversation_history_wrappers, resolve_agent_tool_input,
    resolve_computer, response_span, resume_streamed, resume_streamed_with_agent, retry_policies,
    run, run_demo_loop, run_streamed, run_streamed_with_options, run_sync, run_sync_with_options,
    run_with_options, run_with_session, set_agent_tool_state_scope,
    set_conversation_history_wrappers, set_default_agent_runner,
    set_default_openai_responses_transport, set_default_tracing_export_api_key,
    set_trace_processors, set_trace_provider, set_tracing_disabled, speech_group_span, speech_span,
    task_span, tool_input_guardrail, tool_namespace, tool_output_guardrail, tool_qualified_name,
    tool_trace_name, trace, transcription_span, transform_string_function_style, turn_span,
    validate_function_tool_namespace_shape, validate_json,
};
pub use agents_openai::{
    ChatCmplHelpers, ChatCmplStreamHandler, CodeInterpreterTool, CodeInterpreterToolOptions,
    Converter, FAKE_RESPONSES_ID, FileSearchTool, FileSearchToolOptions, ImageGenerationTool,
    ImageGenerationToolOptions, OPENAI_DEFAULT_BASE_URL, OPENAI_DEFAULT_WEBSOCKET_BASE_URL,
    OpenAIAgentRegistrationConfig, OpenAIApi, OpenAIChatCompletionsModel, OpenAIClientOptions,
    OpenAIConversationsSession, OpenAIProvider, OpenAIResponsesCompactionMode,
    OpenAIResponsesCompactionSession, OpenAIResponsesModel, OpenAIResponsesTransport,
    OpenAIResponsesWSModel, OpenAIResponsesWebSocketOptions, OpenAIResponsesWsModel, Part,
    ReasoningContentReplayContext, ReasoningContentSource, ResolvedOpenAIAgentRegistrationConfig,
    ResponsesWebSocketSession, SequenceNumber, StreamingState, ToolSearchTool,
    ToolSearchToolOptions, WebSearchTool, WebSearchToolOptions, code_interpreter_tool,
    code_interpreter_tool_with_options, default_openai_api, default_openai_base_url,
    default_openai_key, default_openai_websocket_base_url, default_should_replay_reasoning_content,
    default_should_trigger_compaction, fake_id, file_search_tool, file_search_tool_with_options,
    file_search_tool_with_vector_store_ids, get_default_openai_agent_registration,
    get_default_openai_client, get_default_openai_key, get_default_openai_websocket_base_url,
    get_openai_base_url, get_openai_retry_advice, get_use_responses_by_default,
    get_use_responses_websocket_by_default, image_generation_tool,
    image_generation_tool_with_options, is_openai_model_name, provider_managed_retries_disabled,
    resolve_openai_agent_registration_config, responses_websocket_session,
    responses_websocket_session_with_options, select_compaction_candidate_items,
    set_default_openai_agent_registration, set_default_openai_api, set_default_openai_client,
    set_default_openai_harness, set_default_openai_key, set_default_openai_key_shared,
    set_default_openai_websocket_base_url, set_openai_base_url, set_tracing_export_api_key,
    set_use_responses_by_default, set_use_responses_websocket_by_default,
    should_disable_provider_managed_retries, should_disable_websocket_pre_event_retries,
    start_openai_conversations_session, tool_search_tool, tool_search_tool_with_options,
    tracing_export_api_key, web_search_tool, web_search_tool_with_options,
    websocket_pre_event_retries_disabled,
};

pub mod realtime {
    pub use agents_realtime::*;
}

pub mod voice {
    pub use agents_voice::*;
}

pub mod extensions {
    pub use agents_extensions::*;
}

pub mod sandbox {
    pub use agents_core::sandbox::*;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn facade_run_uses_core_runner() {
        let agent = Agent::builder("assistant")
            .instructions("Be brief.")
            .build();

        let result = run(&agent, "hello").await.expect("run should succeed");

        assert_eq!(result.agent_name, "assistant");
        assert_eq!(result.final_output.as_deref(), Some("hello"));
    }
}
