use serde::{Deserialize, Serialize};

/// Declarative handoff metadata for routing between agents.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Handoff {
    pub target: String,
    pub description: Option<String>,
}

impl Handoff {
    pub fn new(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}
