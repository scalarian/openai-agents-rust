use crate::agent::Agent;
use crate::errors::Result;
use crate::handoff::Handoff;
use crate::run_config::{CallModelData, ModelInputData, RunConfig};
use crate::run_context::RunContextWrapper;
use crate::tool::ToolDefinition;

pub(crate) fn validate_run_hooks() -> Result<()> {
    Ok(())
}

pub(crate) fn maybe_filter_model_input(
    config: &RunConfig,
    agent: &Agent,
    context: &RunContextWrapper,
    model_data: ModelInputData,
) -> Result<ModelInputData> {
    let Some(filter) = &config.call_model_input_filter else {
        return Ok(model_data);
    };
    filter(&CallModelData {
        model_data,
        agent: agent.clone(),
        context: Some(context.context.clone()),
    })
}

pub(crate) fn get_handoffs(agent: &Agent) -> Vec<Handoff> {
    agent.handoffs.clone()
}

pub(crate) fn get_all_tools(agent: &Agent) -> Vec<ToolDefinition> {
    agent.tool_definitions()
}

pub(crate) fn get_output_schema(_agent: &Agent) -> Option<serde_json::Value> {
    None
}

pub(crate) fn get_model(agent: &Agent) -> Option<String> {
    agent.model.clone()
}
