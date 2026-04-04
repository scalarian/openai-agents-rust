use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeModelErrorEvent {
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeModelToolCallEvent {
    pub call_id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeModelAudioEvent {
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeModelAudioInterruptedEvent {
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeModelAudioDoneEvent {
    pub total_bytes: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeModelTranscriptDeltaEvent {
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeModelResponseDoneEvent {
    pub response_id: Option<String>,
    pub request_id: Option<String>,
    pub payload: Option<Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RealtimeModelEvent {
    Error(RealtimeModelErrorEvent),
    ToolCall(RealtimeModelToolCallEvent),
    Audio(RealtimeModelAudioEvent),
    AudioInterrupted(RealtimeModelAudioInterruptedEvent),
    AudioDone(RealtimeModelAudioDoneEvent),
    TranscriptDelta(RealtimeModelTranscriptDeltaEvent),
    ResponseDone(RealtimeModelResponseDoneEvent),
}
