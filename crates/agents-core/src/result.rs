use serde::{Deserialize, Serialize};

use crate::items::{InputItem, OutputItem};
use crate::tracing::Trace;
use crate::usage::Usage;

/// Result of an agent run.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RunResult {
    pub agent_name: String,
    pub input: Vec<InputItem>,
    pub output: Vec<OutputItem>,
    pub final_output: Option<String>,
    pub usage: Usage,
    pub trace: Option<Trace>,
}
