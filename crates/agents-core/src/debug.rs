use serde::{Deserialize, Serialize};

/// Debug settings used while validating runtime behavior.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DebugSettings {
    pub log_model_payloads: bool,
    pub log_tool_payloads: bool,
    pub log_stream_events: bool,
}
