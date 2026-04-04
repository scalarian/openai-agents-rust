use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeModelRawClientMessage {
    pub kind: String,
    pub payload: Value,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeModelInputTextContent {
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeModelInputImageContent {
    pub image_url: String,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeModelUserInputMessage {
    pub text: Option<RealtimeModelInputTextContent>,
    pub image: Option<RealtimeModelInputImageContent>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeModelSendRawMessage {
    pub message: RealtimeModelRawClientMessage,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeModelSendUserInput {
    pub message: RealtimeModelUserInputMessage,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeModelSendAudio {
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeModelSendToolOutput {
    pub call_id: String,
    pub output: String,
}
