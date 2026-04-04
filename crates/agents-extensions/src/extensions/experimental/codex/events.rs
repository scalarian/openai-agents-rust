use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::extensions::experimental::codex::items::{ThreadItem, coerce_thread_item};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadStartedEvent {
    pub thread_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnStartedEvent;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnCompletedEvent {
    pub usage: Option<Usage>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadError {
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnFailedEvent {
    pub error: ThreadError,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemStartedEvent {
    pub item: ThreadItem,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemUpdatedEvent {
    pub item: ThreadItem,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemCompletedEvent {
    pub item: ThreadItem,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadErrorEvent {
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ThreadEvent {
    #[serde(rename = "thread.started")]
    ThreadStarted { thread_id: String },
    #[serde(rename = "turn.started")]
    TurnStarted,
    #[serde(rename = "turn.completed")]
    TurnCompleted { usage: Option<Usage> },
    #[serde(rename = "turn.failed")]
    TurnFailed { error: ThreadError },
    #[serde(rename = "item.started")]
    ItemStarted { item: Value },
    #[serde(rename = "item.updated")]
    ItemUpdated { item: Value },
    #[serde(rename = "item.completed")]
    ItemCompleted { item: Value },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(other)]
    Unknown,
}

pub fn coerce_thread_event(raw: Value) -> ThreadEvent {
    serde_json::from_value::<ThreadEvent>(raw).unwrap_or(ThreadEvent::Unknown)
}

impl ThreadEvent {
    pub fn typed_item(&self) -> Option<ThreadItem> {
        match self {
            ThreadEvent::ItemStarted { item }
            | ThreadEvent::ItemUpdated { item }
            | ThreadEvent::ItemCompleted { item } => Some(coerce_thread_item(item.clone())),
            _ => None,
        }
    }
}
