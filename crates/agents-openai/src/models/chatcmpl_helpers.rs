use agents_core::{AgentsError, InputItem, LOGGER_TARGET, Result, ToolDefinition, UserError};
use serde_json::{Value, json};

pub struct ChatCmplHelpers;

pub const OMITTED_TOOL_OUTPUT_PLACEHOLDER: &str = "[tool output omitted]";

const EMPTY_TOOL_OUTPUT_MESSAGE: &str = "Chat Completions tool outputs cannot be empty or contain \
only non-text content unless preserve_tool_output_all_content=True.";

impl ChatCmplHelpers {
    pub fn input_to_messages(items: &[InputItem]) -> Vec<Value> {
        items
            .iter()
            .flat_map(|item| match item {
                InputItem::Text { text } => vec![json!({
                    "role": "user",
                    "content": text,
                })],
                InputItem::Json { value } => input_json_to_messages(value),
            })
            .collect()
    }

    pub fn tools_to_payload(tools: &[ToolDefinition]) -> Vec<Value> {
        tools
            .iter()
            .filter(|tool| !matches!(tool.kind, agents_core::ToolDefinitionKind::Custom))
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.input_json_schema.clone().unwrap_or_else(|| json!({
                            "type": "object",
                            "properties": {}
                        })),
                    }
                })
            })
            .collect()
    }
}

pub(crate) fn chat_tool_output_content(value: Option<&Value>, strict: bool) -> Result<Value> {
    let Some(value) = value else {
        return Ok(Value::String("null".to_owned()));
    };

    match value {
        Value::String(text) => Ok(Value::String(text.clone())),
        Value::Array(items) => text_tool_output_parts(items, strict),
        Value::Object(map) => match map.get("type").and_then(Value::as_str) {
            Some("text") => Ok(Value::String(
                map.get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            )),
            Some("json") => Ok(Value::String(
                map.get("value")
                    .map(json_value_to_string)
                    .unwrap_or_else(|| "null".to_owned()),
            )),
            Some("input_text") => Ok(Value::Array(vec![json!({
                "type": "text",
                "text": map.get("text").and_then(Value::as_str).unwrap_or_default(),
            })])),
            Some("input_image") | Some("image_url") | Some("input_audio") | Some("input_file")
            | Some("video_url") => omitted_tool_output_content(strict),
            _ => Ok(Value::String(json_value_to_string(value))),
        },
        Value::Null => Ok(Value::String("null".to_owned())),
        _ => Ok(Value::String(json_value_to_string(value))),
    }
}

fn text_tool_output_parts(items: &[Value], strict: bool) -> Result<Value> {
    let text_parts = items
        .iter()
        .filter_map(chat_text_part)
        .collect::<Vec<Value>>();

    if text_parts.is_empty() {
        omitted_tool_output_content(strict)
    } else {
        Ok(Value::Array(text_parts))
    }
}

fn chat_text_part(value: &Value) -> Option<Value> {
    let map = value.as_object()?;
    match map.get("type").and_then(Value::as_str) {
        Some("input_text") | Some("text") => Some(json!({
            "type": "text",
            "text": map.get("text").and_then(Value::as_str).unwrap_or_default(),
        })),
        _ => None,
    }
}

fn omitted_tool_output_content(strict: bool) -> Result<Value> {
    if strict {
        return Err(AgentsError::from(UserError {
            message: EMPTY_TOOL_OUTPUT_MESSAGE.to_owned(),
        }));
    }

    log::warn!(
        target: LOGGER_TARGET,
        "{} Replacing the tool output with a placeholder; enable strict feature validation to raise an error instead.",
        EMPTY_TOOL_OUTPUT_MESSAGE
    );

    Ok(Value::String(OMITTED_TOOL_OUTPUT_PLACEHOLDER.to_owned()))
}

fn json_value_to_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| other.to_string()),
    }
}

fn input_json_to_messages(value: &Value) -> Vec<Value> {
    if let Some(role) = value.get("role").and_then(Value::as_str) {
        return vec![json!({
            "role": role,
            "content": value.get("content").cloned().unwrap_or_else(|| json!(value.to_string())),
        })];
    }

    match value.get("type").and_then(Value::as_str) {
        Some("tool_call_output") => vec![json!({
            "role": "tool",
            "tool_call_id": value.get("call_id"),
            "content": chat_tool_output_content(value.get("output"), false)
                .unwrap_or_else(|_| Value::String(OMITTED_TOOL_OUTPUT_PLACEHOLDER.to_owned())),
        })],
        Some("tool_call") => vec![json!({
            "role": "assistant",
            "content": Value::Null,
            "tool_calls": [{
                "id": value.get("call_id").cloned().unwrap_or_else(|| json!("")),
                "type": "function",
                "function": {
                    "name": value.get("tool_name").cloned().unwrap_or_else(|| json!("")),
                    "arguments": value.get("arguments").cloned().unwrap_or_else(|| json!({})).to_string(),
                }
            }]
        })],
        Some("custom_tool_call") | Some("custom_tool_call_output") => Vec::new(),
        Some("reasoning") => vec![json!({
            "role": "assistant",
            "content": value.get("text").cloned().unwrap_or_else(|| json!("")),
        })],
        _ => vec![value.clone()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_messages_and_tools() {
        let messages = ChatCmplHelpers::input_to_messages(&[
            InputItem::from("hello"),
            InputItem::Json {
                value: json!({"type":"tool_call","tool_name":"search","call_id":"call-1","arguments":{"q":"rust"}}),
            },
        ]);
        let tools = ChatCmplHelpers::tools_to_payload(&[ToolDefinition::new("search", "Search")]);

        assert_eq!(messages[0]["role"], "user");
        assert_eq!(tools[0]["type"], "function");
    }

    #[test]
    fn skips_custom_tools_in_chat_payload_helper() {
        let tools = ChatCmplHelpers::tools_to_payload(&[
            ToolDefinition::new("search", "Search"),
            ToolDefinition::custom("raw_editor", "Edit raw text."),
        ]);

        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["function"]["name"], "search");
    }

    #[test]
    fn skips_custom_tool_replay_items_in_chat_messages() {
        let messages = ChatCmplHelpers::input_to_messages(&[
            InputItem::from("hello"),
            InputItem::Json {
                value: json!({
                    "type": "custom_tool_call",
                    "call_id": "call-custom",
                    "name": "raw_editor",
                    "input": "hello",
                }),
            },
            InputItem::Json {
                value: json!({
                    "type": "custom_tool_call_output",
                    "call_id": "call-custom",
                    "output": "HELLO",
                }),
            },
        ]);

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["content"], "hello");
    }

    #[test]
    fn empty_tool_output_uses_placeholder() {
        let messages = ChatCmplHelpers::input_to_messages(&[InputItem::Json {
            value: json!({
                "type": "tool_call_output",
                "call_id": "call-empty",
                "output": [],
            }),
        }]);

        assert_eq!(messages[0]["role"], "tool");
        assert_eq!(messages[0]["content"], OMITTED_TOOL_OUTPUT_PLACEHOLDER);
    }

    #[test]
    fn non_text_tool_output_uses_placeholder() {
        let messages = ChatCmplHelpers::input_to_messages(&[InputItem::Json {
            value: json!({
                "type": "tool_call_output",
                "call_id": "call-image",
                "output": [
                    {
                        "type": "input_image",
                        "image_url": "https://example.com/image.png",
                    },
                ],
            }),
        }]);

        assert_eq!(messages[0]["content"], OMITTED_TOOL_OUTPUT_PLACEHOLDER);
    }

    #[test]
    fn mixed_tool_output_keeps_text_parts() {
        let messages = ChatCmplHelpers::input_to_messages(&[InputItem::Json {
            value: json!({
                "type": "tool_call_output",
                "call_id": "call-mixed",
                "output": [
                    {
                        "type": "input_text",
                        "text": "visible text",
                    },
                    {
                        "type": "input_image",
                        "image_url": "https://example.com/image.png",
                    },
                ],
            }),
        }]);

        assert_eq!(
            messages[0]["content"],
            json!([
                {
                    "type": "text",
                    "text": "visible text",
                },
            ])
        );
    }

    #[test]
    fn strict_non_text_tool_output_errors() {
        let error = chat_tool_output_content(
            Some(&json!([
                {
                    "type": "input_image",
                    "image_url": "https://example.com/image.png",
                },
            ])),
            true,
        )
        .expect_err("strict conversion should reject non-text-only tool output");

        assert!(
            error
                .to_string()
                .contains("cannot be empty or contain only non-text content")
        );
    }
}
