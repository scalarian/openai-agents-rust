use serde::{Deserialize, Serialize};

use crate::agent::Agent;
use crate::items::{OutputItem, RunItem};
use crate::run_state::RunInterruption;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct QueueCompleteSentinel;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolRunHandoff {
    pub target_agent: Agent,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolRunFunction {
    pub new_items: Vec<RunItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolRunComputerAction {
    pub action_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolRunMCPApprovalRequest {
    pub approval_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolRunLocalShellCall {
    pub command: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolRunShellCall {
    pub command: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolRunApplyPatchCall {
    pub patch: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NextStepFinalOutput {
    pub output: Vec<OutputItem>,
    pub final_output: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NextStepHandoff {
    pub agent: Agent,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NextStepInterruption {
    pub interruption: RunInterruption,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NextStepRunAgain {
    pub new_items: Vec<RunItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SingleStepResult {
    FinalOutput(NextStepFinalOutput),
    Handoff(NextStepHandoff),
    Interruption(NextStepInterruption),
    RunAgain(NextStepRunAgain),
}
