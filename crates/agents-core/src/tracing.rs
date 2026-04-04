use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Trace metadata for an agent run.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Trace {
    pub id: Uuid,
    pub workflow_name: String,
}

impl Trace {
    pub fn new(workflow_name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            workflow_name: workflow_name.into(),
        }
    }
}

/// Span metadata attached to a trace.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Span {
    pub id: Uuid,
    pub trace_id: Uuid,
    pub name: String,
}

impl Span {
    pub fn new(trace_id: Uuid, name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            trace_id,
            name: name.into(),
        }
    }
}
