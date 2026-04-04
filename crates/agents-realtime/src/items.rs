use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InputText {
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InputAudio {
    pub format: Option<String>,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InputImage {
    pub image_url: String,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AssistantText {
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AssistantAudio {
    pub format: Option<String>,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SystemMessageItem {
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UserMessageItem {
    pub text: Option<InputText>,
    pub audio: Option<InputAudio>,
    pub image: Option<InputImage>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AssistantMessageItem {
    pub text: Option<AssistantText>,
    pub audio: Option<AssistantAudio>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolCallItem {
    pub call_id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolCallOutputItem {
    pub call_id: String,
    pub output: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RealtimeItem {
    SystemMessage(SystemMessageItem),
    UserMessage(UserMessageItem),
    AssistantMessage(AssistantMessageItem),
    ToolCall(ToolCallItem),
    ToolCallOutput(ToolCallOutputItem),
}
