//! Realtime agent scaffolding.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RealtimeAgent {
    pub name: String,
    pub instructions: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RealtimeSession {
    pub model: Option<String>,
    pub connected: bool,
}

impl RealtimeSession {
    pub fn connect(mut self) -> Self {
        self.connected = true;
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RealtimeEvent {
    ResponseStarted,
    ResponseCompleted,
    TranscriptDelta { text: String },
    ToolCall { name: String },
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OpenAIRealtimeWebSocket {
    pub model: Option<String>,
}
