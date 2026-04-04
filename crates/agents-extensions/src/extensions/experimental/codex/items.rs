use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandExecutionStatus {
    InProgress,
    Completed,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchChangeKind {
    Add,
    Delete,
    Update,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchApplyStatus {
    Completed,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpToolCallStatus {
    InProgress,
    Completed,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandExecutionItem {
    pub id: String,
    pub command: String,
    pub status: CommandExecutionStatus,
    pub aggregated_output: String,
    pub exit_code: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileUpdateChange {
    pub path: String,
    pub kind: PatchChangeKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileChangeItem {
    pub id: String,
    pub changes: Vec<FileUpdateChange>,
    pub status: PatchApplyStatus,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct McpToolCallResult {
    pub content: Vec<Value>,
    pub structured_content: Option<Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpToolCallError {
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct McpToolCallItem {
    pub id: String,
    pub server: String,
    pub tool: String,
    pub arguments: Value,
    pub status: McpToolCallStatus,
    pub result: Option<McpToolCallResult>,
    pub error: Option<McpToolCallError>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentMessageItem {
    pub id: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReasoningItem {
    pub id: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchItem {
    pub id: String,
    pub query: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorItem {
    pub id: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoItem {
    pub text: String,
    pub completed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoListItem {
    pub id: String,
    pub items: Vec<TodoItem>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ThreadItem {
    AgentMessage(AgentMessageItem),
    Reasoning(ReasoningItem),
    CommandExecution(CommandExecutionItem),
    FileChange(FileChangeItem),
    McpToolCall(McpToolCallItem),
    WebSearch(WebSearchItem),
    TodoList(TodoListItem),
    Error(ErrorItem),
    Unknown {
        item_type: String,
        id: Option<String>,
        payload: Value,
    },
}

pub fn is_agent_message_item(item: &ThreadItem) -> bool {
    matches!(item, ThreadItem::AgentMessage(_))
}

pub fn coerce_thread_item(raw: Value) -> ThreadItem {
    let item_type = raw
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_owned();
    match item_type.as_str() {
        "command_execution" => serde_json::from_value(raw.clone())
            .map(ThreadItem::CommandExecution)
            .unwrap_or(ThreadItem::Unknown {
                item_type,
                id: raw.get("id").and_then(Value::as_str).map(ToOwned::to_owned),
                payload: raw,
            }),
        "file_change" => serde_json::from_value(raw.clone())
            .map(ThreadItem::FileChange)
            .unwrap_or(ThreadItem::Unknown {
                item_type,
                id: raw.get("id").and_then(Value::as_str).map(ToOwned::to_owned),
                payload: raw,
            }),
        "mcp_tool_call" => serde_json::from_value(raw.clone())
            .map(ThreadItem::McpToolCall)
            .unwrap_or(ThreadItem::Unknown {
                item_type,
                id: raw.get("id").and_then(Value::as_str).map(ToOwned::to_owned),
                payload: raw,
            }),
        "agent_message" => serde_json::from_value(raw.clone())
            .map(ThreadItem::AgentMessage)
            .unwrap_or(ThreadItem::Unknown {
                item_type,
                id: raw.get("id").and_then(Value::as_str).map(ToOwned::to_owned),
                payload: raw,
            }),
        "reasoning" => serde_json::from_value(raw.clone())
            .map(ThreadItem::Reasoning)
            .unwrap_or(ThreadItem::Unknown {
                item_type,
                id: raw.get("id").and_then(Value::as_str).map(ToOwned::to_owned),
                payload: raw,
            }),
        "web_search" => serde_json::from_value(raw.clone())
            .map(ThreadItem::WebSearch)
            .unwrap_or(ThreadItem::Unknown {
                item_type,
                id: raw.get("id").and_then(Value::as_str).map(ToOwned::to_owned),
                payload: raw,
            }),
        "todo_list" => serde_json::from_value(raw.clone())
            .map(ThreadItem::TodoList)
            .unwrap_or(ThreadItem::Unknown {
                item_type,
                id: raw.get("id").and_then(Value::as_str).map(ToOwned::to_owned),
                payload: raw,
            }),
        "error" => serde_json::from_value(raw.clone())
            .map(ThreadItem::Error)
            .unwrap_or(ThreadItem::Unknown {
                item_type,
                id: raw.get("id").and_then(Value::as_str).map(ToOwned::to_owned),
                payload: raw,
            }),
        _ => ThreadItem::Unknown {
            item_type,
            id: raw.get("id").and_then(Value::as_str).map(ToOwned::to_owned),
            payload: raw,
        },
    }
}
