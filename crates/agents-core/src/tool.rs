use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Shared tool definition metadata.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
}

impl ToolDefinition {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

/// Common behavior for tools exposed to an agent.
pub trait Tool: Send + Sync {
    fn definition(&self) -> &ToolDefinition;
}

/// Static tool metadata used during bootstrap.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaticTool {
    pub definition: ToolDefinition,
}

impl StaticTool {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            definition: ToolDefinition::new(name, description),
        }
    }
}

impl Tool for StaticTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }
}
