use std::fmt;
use std::sync::Arc;

use futures::future::BoxFuture;
use serde_json::Value;

use crate::agent::Agent;
use crate::exceptions::{MaxTurnsExceeded, ModelRefusalError};
use crate::guardrail::{InputGuardrailResult, OutputGuardrailResult};
use crate::items::{InputItem, OutputItem, RunItem};
use crate::model::ModelResponse;
use crate::run_context::{RunContext, RunContextWrapper};

#[derive(Clone, Debug, Default)]
pub struct RunErrorData {
    pub input: Vec<InputItem>,
    pub new_items: Vec<RunItem>,
    pub history: Vec<InputItem>,
    pub output: Vec<OutputItem>,
    pub raw_responses: Vec<ModelResponse>,
    pub input_guardrail_results: Vec<InputGuardrailResult>,
    pub output_guardrail_results: Vec<OutputGuardrailResult>,
    pub last_agent: Agent,
}

#[derive(Clone, Debug)]
pub struct RunErrorHandlerInput {
    pub error: MaxTurnsExceeded,
    pub context: RunContextWrapper<RunContext>,
    pub run_data: RunErrorData,
}

#[derive(Clone, Debug)]
pub struct ModelRefusalHandlerInput {
    pub error: ModelRefusalError,
    pub context: RunContextWrapper<RunContext>,
    pub run_data: RunErrorData,
}

#[derive(Clone, Debug)]
pub struct RunErrorHandlerResult {
    pub final_output: Value,
    pub include_in_history: bool,
}

impl RunErrorHandlerResult {
    pub fn new(final_output: Value) -> Self {
        Self {
            final_output,
            include_in_history: true,
        }
    }
}

pub type RunErrorHandler = Arc<
    dyn Fn(RunErrorHandlerInput) -> BoxFuture<'static, Option<RunErrorHandlerResult>> + Send + Sync,
>;

pub type ModelRefusalHandler = Arc<
    dyn Fn(ModelRefusalHandlerInput) -> BoxFuture<'static, Option<RunErrorHandlerResult>>
        + Send
        + Sync,
>;

#[derive(Clone, Default)]
pub struct RunErrorHandlers {
    pub max_turns: Option<RunErrorHandler>,
    pub model_refusal: Option<ModelRefusalHandler>,
}

impl fmt::Debug for RunErrorHandlers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RunErrorHandlers")
            .field("max_turns", &self.max_turns.as_ref().map(|_| "<handler>"))
            .field(
                "model_refusal",
                &self.model_refusal.as_ref().map(|_| "<handler>"),
            )
            .finish()
    }
}
