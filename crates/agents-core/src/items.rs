use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Input items passed into a run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputItem {
    Text { text: String },
    Json { value: Value },
}

impl InputItem {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            Self::Json { .. } => None,
        }
    }
}

impl From<&str> for InputItem {
    fn from(value: &str) -> Self {
        Self::Text {
            text: value.to_owned(),
        }
    }
}

impl From<String> for InputItem {
    fn from(value: String) -> Self {
        Self::Text { text: value }
    }
}

/// Output items emitted by a run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputItem {
    Text { text: String },
    Json { value: Value },
}

impl OutputItem {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            Self::Json { .. } => None,
        }
    }
}
