//! OpenAI-specific providers, models, tools, and sessions.

#[path = "models/_openai_retry.rs"]
mod _openai_retry;
#[path = "models/_openai_shared.rs"]
mod _openai_shared;
#[path = "models/_retry_runtime.rs"]
mod _retry_runtime;
#[path = "models/chatcmpl_converter.rs"]
mod chatcmpl_converter;
#[path = "models/chatcmpl_helpers.rs"]
mod chatcmpl_helpers;
#[path = "models/chatcmpl_stream_handler.rs"]
mod chatcmpl_stream_handler;
mod defaults;
#[path = "models/fake_id.rs"]
mod fake_id;
mod memory;
mod models;
mod provider;
#[path = "models/reasoning_content_replay.rs"]
mod reasoning_content_replay;
mod tools;
mod websocket;

pub use _openai_retry::get_openai_retry_advice;
pub use _openai_shared::{
    get_default_openai_client, get_default_openai_key, get_default_openai_websocket_base_url,
    get_openai_base_url, get_use_responses_by_default, get_use_responses_websocket_by_default,
    set_default_openai_client, set_default_openai_key_shared,
    set_default_openai_websocket_base_url, set_openai_base_url, set_use_responses_by_default,
    set_use_responses_websocket_by_default,
};
pub use _retry_runtime::{
    provider_managed_retries_disabled, should_disable_provider_managed_retries,
    should_disable_websocket_pre_event_retries, websocket_pre_event_retries_disabled,
};
pub use chatcmpl_converter::Converter;
pub use chatcmpl_helpers::ChatCmplHelpers;
pub use chatcmpl_stream_handler::{ChatCmplStreamHandler, Part, SequenceNumber, StreamingState};
pub use defaults::{
    OPENAI_DEFAULT_BASE_URL, OPENAI_DEFAULT_WEBSOCKET_BASE_URL, OpenAIApi, default_openai_api,
    default_openai_base_url, default_openai_key, default_openai_websocket_base_url,
    set_default_openai_api, set_default_openai_key, set_tracing_export_api_key,
    tracing_export_api_key,
};
pub use fake_id::{FAKE_RESPONSES_ID, fake_id};
pub use memory::{
    OpenAIConversationsSession, OpenAIResponsesCompactionMode, OpenAIResponsesCompactionSession,
};
pub use models::{
    OpenAIChatCompletionsModel, OpenAIClientOptions, OpenAIResponsesModel, OpenAIResponsesWsModel,
};
pub use provider::{OpenAIProvider, OpenAIResponsesTransport};
pub use reasoning_content_replay::{
    ReasoningContentReplayContext, ReasoningContentSource, default_should_replay_reasoning_content,
};
pub use tools::{
    code_interpreter_tool, file_search_tool, image_generation_tool, tool_search_tool,
    web_search_tool,
};
pub use websocket::{ResponsesWebSocketSession, responses_websocket_session};
