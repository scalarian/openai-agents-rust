use serde::{Deserialize, Serialize};

/// Per-run metadata passed through agent execution.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RunContext {
    pub conversation_id: Option<String>,
    pub workflow_name: Option<String>,
}
