//! Public facade for the Rust port of the OpenAI Agents SDK.

pub use agents_core::{
    Agent, AgentBuilder, AgentsError, ApplyPatchOperation, ApplyPatchResult,
    CURRENT_RUN_STATE_SCHEMA_VERSION, Computer, DebugSettings, DocstringStyle, Editor,
    FunctionSchema, FunctionTool, FunctionToolResult, GuardrailFunctionOutput, Handoff,
    HandoffBuilder, HandoffHistoryMapper, HandoffInputData, HandoffInputFilter, InputGuardrail,
    InputGuardrailResult, InputItem, MaybeAwaitable, MemorySession, Model, ModelProvider,
    ModelRequest, ModelResponse, ModelSettings, OutputGuardrail, OutputGuardrailResult, OutputItem,
    ReasoningSettings, Result, RunConfig, RunContext, RunErrorData, RunErrorHandler,
    RunErrorHandlerInput, RunErrorHandlerResult, RunErrorHandlers, RunInterruption,
    RunInterruptionKind, RunResult, RunResultStreaming, RunState, Runner, SdkConfig, Session, Span,
    StaticTool, StreamEvent, Tool, ToolCall, ToolContext, ToolDefinition, ToolGuardrailBehavior,
    ToolGuardrailFunctionOutput, ToolInputGuardrail, ToolInputGuardrailData,
    ToolInputGuardrailResult, ToolOutput, ToolOutputFileContent, ToolOutputGuardrail,
    ToolOutputGuardrailData, ToolOutputGuardrailResult, ToolOutputImage, ToolOutputText, Trace,
    Usage, VERSION, apply_diff, attach_error_to_current_span, attach_error_to_span, function_tool,
    get_default_model, get_default_model_settings, gpt_5_reasoning_settings_required, handoff,
    input_guardrail, is_gpt_5_default, noop_coroutine, output_guardrail, pretty_print_result,
    pretty_print_run_error_details, pretty_print_run_result_streaming, run, run_demo_loop,
    run_with_session, tool_input_guardrail, tool_output_guardrail, transform_string_function_style,
    validate_json,
};
pub use agents_openai::{
    OPENAI_DEFAULT_BASE_URL, OPENAI_DEFAULT_WEBSOCKET_BASE_URL, OpenAIApi,
    OpenAIChatCompletionsModel, OpenAIClientOptions, OpenAIConversationsSession, OpenAIProvider,
    OpenAIResponsesCompactionMode, OpenAIResponsesCompactionSession, OpenAIResponsesModel,
    OpenAIResponsesTransport, OpenAIResponsesWsModel, ResponsesWebSocketSession,
    code_interpreter_tool, default_openai_api, default_openai_base_url, default_openai_key,
    default_openai_websocket_base_url, file_search_tool, image_generation_tool,
    responses_websocket_session, set_default_openai_api, set_default_openai_key,
    set_tracing_export_api_key, tool_search_tool, tracing_export_api_key, web_search_tool,
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
