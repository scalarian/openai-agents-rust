use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::agent::Agent;
use crate::items::InputItem;
use crate::run_context::{RunContext, RunContextWrapper};
use crate::run_error_handlers::RunErrorHandlers;

pub const DEFAULT_MAX_TURNS: usize = 10;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ModelInputData {
    pub input: Vec<InputItem>,
    pub instructions: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CallModelData<TContext = RunContext> {
    pub model_data: ModelInputData,
    pub agent: Agent,
    pub context: Option<TContext>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReasoningItemIdPolicy {
    #[default]
    Preserve,
    Omit,
}

#[derive(Clone, Debug)]
pub struct ToolErrorFormatterArgs<TContext = RunContext> {
    pub kind: &'static str,
    pub tool_type: &'static str,
    pub tool_name: String,
    pub call_id: String,
    pub default_message: String,
    pub run_context: RunContextWrapper<TContext>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RunConfig {
    pub model: Option<String>,
    pub max_turns: usize,
    pub tracing_disabled: bool,
    pub trace_include_sensitive_data: bool,
    pub workflow_name: String,
    pub trace_id: Option<String>,
    pub group_id: Option<String>,
    pub previous_response_id: Option<String>,
    pub auto_previous_response_id: bool,
    pub conversation_id: Option<String>,
    pub reasoning_item_id_policy: ReasoningItemIdPolicy,
    #[serde(skip, default)]
    pub call_model_input_filter: Option<CallModelInputFilter>,
    #[serde(skip, default)]
    pub run_error_handlers: RunErrorHandlers,
}

impl std::fmt::Debug for RunConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunConfig")
            .field("model", &self.model)
            .field("max_turns", &self.max_turns)
            .field("tracing_disabled", &self.tracing_disabled)
            .field(
                "trace_include_sensitive_data",
                &self.trace_include_sensitive_data,
            )
            .field("workflow_name", &self.workflow_name)
            .field("trace_id", &self.trace_id)
            .field("group_id", &self.group_id)
            .field("previous_response_id", &self.previous_response_id)
            .field("auto_previous_response_id", &self.auto_previous_response_id)
            .field("conversation_id", &self.conversation_id)
            .field("reasoning_item_id_policy", &self.reasoning_item_id_policy)
            .field(
                "call_model_input_filter",
                &self.call_model_input_filter.as_ref().map(|_| "<filter>"),
            )
            .field("run_error_handlers", &self.run_error_handlers)
            .finish()
    }
}

pub type CallModelInputFilter =
    Arc<dyn Fn(&CallModelData<RunContext>) -> crate::errors::Result<ModelInputData> + Send + Sync>;

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            model: None,
            max_turns: DEFAULT_MAX_TURNS,
            tracing_disabled: false,
            trace_include_sensitive_data: true,
            workflow_name: "Agent workflow".to_owned(),
            trace_id: None,
            group_id: None,
            previous_response_id: None,
            auto_previous_response_id: false,
            conversation_id: None,
            reasoning_item_id_policy: ReasoningItemIdPolicy::Preserve,
            call_model_input_filter: None,
            run_error_handlers: RunErrorHandlers::default(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RunOptions<TContext = RunContext> {
    pub context: Option<TContext>,
    pub max_turns: Option<usize>,
    pub run_config: Option<RunConfig>,
}
