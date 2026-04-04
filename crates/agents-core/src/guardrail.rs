use serde::{Deserialize, Serialize};

/// Input guardrail metadata.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InputGuardrail {
    pub name: String,
    pub description: Option<String>,
}

impl InputGuardrail {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Output guardrail metadata.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OutputGuardrail {
    pub name: String,
    pub description: Option<String>,
}

impl OutputGuardrail {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}
