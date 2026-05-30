use std::sync::{OnceLock, RwLock};

use crate::handoff::HandoffHistoryMapper;
use crate::items::{InputItem, RunItem};

pub const DEFAULT_CONVERSATION_HISTORY_START: &str = "<CONVERSATION HISTORY>";
pub const DEFAULT_CONVERSATION_HISTORY_END: &str = "</CONVERSATION HISTORY>";

static CONVERSATION_HISTORY_WRAPPERS: OnceLock<RwLock<(String, String)>> = OnceLock::new();

fn wrappers() -> &'static RwLock<(String, String)> {
    CONVERSATION_HISTORY_WRAPPERS.get_or_init(|| {
        RwLock::new((
            DEFAULT_CONVERSATION_HISTORY_START.to_owned(),
            DEFAULT_CONVERSATION_HISTORY_END.to_owned(),
        ))
    })
}

pub fn set_conversation_history_wrappers(start: Option<&str>, end: Option<&str>) {
    let mut wrappers = wrappers().write().expect("conversation history wrappers");
    if let Some(start) = start {
        wrappers.0 = start.to_owned();
    }
    if let Some(end) = end {
        wrappers.1 = end.to_owned();
    }
}

pub fn reset_conversation_history_wrappers() {
    *wrappers().write().expect("conversation history wrappers") = (
        DEFAULT_CONVERSATION_HISTORY_START.to_owned(),
        DEFAULT_CONVERSATION_HISTORY_END.to_owned(),
    );
}

pub fn get_conversation_history_wrappers() -> (String, String) {
    wrappers()
        .read()
        .expect("conversation history wrappers")
        .clone()
}

pub fn default_handoff_history_mapper(transcript: Vec<InputItem>) -> Vec<InputItem> {
    vec![build_summary_message(&transcript)]
}

pub fn nest_handoff_history(
    input_data: crate::handoff::HandoffInputData,
) -> crate::handoff::HandoffInputData {
    nest_handoff_history_with_mapper(input_data, None)
}

pub fn nest_handoff_history_with_mapper(
    input_data: crate::handoff::HandoffInputData,
    history_mapper: Option<HandoffHistoryMapper>,
) -> crate::handoff::HandoffInputData {
    let transcript = build_transcript(
        &input_data.input_history,
        &input_data.pre_handoff_items,
        &input_data.new_items,
    );
    let mapped_history = history_mapper
        .map(|mapper| mapper(transcript.clone()))
        .unwrap_or_else(|| default_handoff_history_mapper(transcript));

    crate::handoff::HandoffInputData {
        input_history: mapped_history,
        pre_handoff_items: input_data
            .pre_handoff_items
            .into_iter()
            .filter(|item| should_forward_run_item(item))
            .collect(),
        new_items: input_data.new_items.clone(),
        input_items: Some(
            input_data
                .new_items
                .into_iter()
                .filter(|item| should_forward_run_item(item))
                .collect(),
        ),
    }
}

fn build_transcript(
    input_history: &[InputItem],
    pre_handoff_items: &[RunItem],
    new_items: &[RunItem],
) -> Vec<InputItem> {
    let mut transcript = flatten_nested_history_messages(input_history);
    transcript.extend(pre_handoff_items.iter().filter_map(RunItem::to_input_item));
    transcript.extend(new_items.iter().filter_map(RunItem::to_input_item));
    transcript
}

fn build_summary_message(transcript: &[InputItem]) -> InputItem {
    let summary_lines = if transcript.is_empty() {
        vec!["(no previous turns recorded)".to_owned()]
    } else {
        transcript
            .iter()
            .enumerate()
            .map(|(index, item)| format!("{}. {}", index + 1, format_transcript_item(item)))
            .collect()
    };
    let (start, end) = get_conversation_history_wrappers();
    let content = std::iter::once(
        "For context, here is the conversation so far between the user and the previous agent:"
            .to_owned(),
    )
    .chain(std::iter::once(start))
    .chain(summary_lines)
    .chain(std::iter::once(end))
    .collect::<Vec<_>>()
    .join("\n");

    InputItem::Json {
        value: serde_json::json!({
            "role": "assistant",
            "content": content,
        }),
    }
}

fn format_transcript_item(item: &InputItem) -> String {
    match item {
        InputItem::Text { text } => {
            if contains_newline(text) {
                format_transcript_item_json(item)
            } else {
                format!("user: {text}")
            }
        }
        InputItem::Json { value } => {
            if let Some(role) = value.get("role").and_then(serde_json::Value::as_str) {
                let content = value.get("content");
                if content.is_none()
                    || matches!(content, Some(serde_json::Value::Null))
                    || matches!(content, Some(serde_json::Value::String(text)) if !contains_newline(text))
                {
                    format_transcript_item_legacy(value, role)
                } else {
                    format_transcript_item_json(item)
                }
            } else {
                let item_type = value
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("item");
                let rest = value_without_keys(value, &["type", "provider_data"]);
                let serialized =
                    serde_json::to_string(&rest).unwrap_or_else(|_| stringify_content(&rest));
                if serialized == "{}" {
                    item_type.to_owned()
                } else {
                    format!("{item_type}: {serialized}")
                }
            }
        }
    }
}

fn contains_newline(value: &str) -> bool {
    value.contains('\n') || value.contains('\r')
}

fn format_transcript_item_legacy(value: &serde_json::Value, role: &str) -> String {
    let mut prefix = role.to_owned();
    if let Some(name) = value
        .get("name")
        .and_then(serde_json::Value::as_str)
        .filter(|name| !name.is_empty())
    {
        prefix = format!("{prefix} ({name})");
    }
    let content = value
        .get("content")
        .map(stringify_content)
        .unwrap_or_default();
    if content.is_empty() {
        prefix
    } else {
        format!("{prefix}: {content}")
    }
}

fn format_transcript_item_json(item: &InputItem) -> String {
    match item {
        InputItem::Text { .. } => {
            serde_json::to_string(item).unwrap_or_else(|_| format!("{item:?}"))
        }
        InputItem::Json { value } => {
            let payload = value_without_keys(value, &["provider_data"]);
            serde_json::to_string(&payload).unwrap_or_else(|_| stringify_content(&payload))
        }
    }
}

fn stringify_content(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        _ => serde_json::to_string(value).unwrap_or_else(|_| value.to_string()),
    }
}

fn value_without_keys(value: &serde_json::Value, keys: &[&str]) -> serde_json::Value {
    let mut value = value.clone();
    if let serde_json::Value::Object(map) = &mut value {
        for key in keys {
            map.remove(*key);
        }
    }
    value
}

fn flatten_nested_history_messages(items: &[InputItem]) -> Vec<InputItem> {
    items
        .iter()
        .flat_map(|item| {
            extract_nested_history_transcript(item).unwrap_or_else(|| vec![item.clone()])
        })
        .collect()
}

fn extract_nested_history_transcript(item: &InputItem) -> Option<Vec<InputItem>> {
    let InputItem::Json { value } = item else {
        return None;
    };
    let content = value.get("content")?.as_str()?;
    let (start_marker, end_marker) = get_conversation_history_wrappers();
    let start_idx = content.find(&start_marker)?;
    let end_idx = content.rfind(&end_marker)?;
    if end_idx <= start_idx {
        return None;
    }
    let body = &content[start_idx + start_marker.len()..end_idx];
    let parsed = split_summary_records(body)
        .into_iter()
        .filter_map(parse_summary_line)
        .collect::<Vec<_>>();
    Some(parsed)
}

fn split_summary_records(body: &str) -> Vec<String> {
    let mut records = Vec::new();
    let mut current = Vec::new();
    let mut current_is_numbered = false;

    for raw_line in body.lines() {
        if raw_line.trim().is_empty() {
            continue;
        }

        let starts_numbered_record = starts_numbered_summary_record(raw_line);
        if current.is_empty() {
            current.push(raw_line.trim().to_owned());
            current_is_numbered = starts_numbered_record;
            continue;
        }

        if starts_numbered_record || !current_is_numbered {
            records.push(current.join("\n"));
            current = vec![raw_line.trim().to_owned()];
            current_is_numbered = starts_numbered_record;
            continue;
        }

        current.push(raw_line.trim_end().to_owned());
    }

    if !current.is_empty() {
        records.push(current.join("\n"));
    }

    records
}

fn starts_numbered_summary_record(line: &str) -> bool {
    let stripped = line.trim_start();
    stripped
        .split_once('.')
        .is_some_and(|(prefix, _)| !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_digit()))
}

fn parse_summary_line(line: String) -> Option<InputItem> {
    let stripped = strip_summary_line_number(line.trim()).trim();
    if let Some(item) = parse_summary_json_item(stripped) {
        return Some(item);
    }

    let (role_part, remainder) = stripped.split_once(':')?;
    let role_text = role_part.trim();
    if role_text.is_empty() {
        return None;
    }
    let (role, name) = split_role_and_name(role_text);
    let content = remainder.trim();
    if !content.is_empty() {
        if let Some(item) = parse_legacy_typed_item(&role, content) {
            return Some(item);
        }
    }

    let mut value = serde_json::json!({ "role": role });
    if let Some(name) = name {
        value["name"] = serde_json::Value::String(name);
    }
    if !content.is_empty() {
        value["content"] = serde_json::Value::String(content.to_owned());
    }
    Some(InputItem::Json { value })
}

fn strip_summary_line_number(stripped: &str) -> &str {
    stripped
        .split_once('.')
        .and_then(|(prefix, rest)| {
            (!prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_digit()))
                .then_some(rest.trim())
        })
        .unwrap_or(stripped)
}

fn parse_summary_json_item(value: &str) -> Option<InputItem> {
    let mut parsed = serde_json::from_str::<serde_json::Value>(value).ok()?;
    if let serde_json::Value::Object(map) = &mut parsed {
        map.remove("provider_data");
    } else {
        return None;
    }
    serde_json::from_value::<InputItem>(parsed.clone())
        .ok()
        .or(Some(InputItem::Json { value: parsed }))
}

fn parse_legacy_typed_item(item_type: &str, content: &str) -> Option<InputItem> {
    if matches!(item_type, "assistant" | "user" | "system" | "developer") {
        return None;
    }
    let mut parsed = serde_json::from_str::<serde_json::Value>(content).ok()?;
    let serde_json::Value::Object(map) = &mut parsed else {
        return None;
    };
    map.remove("provider_data");
    map.insert(
        "type".to_owned(),
        serde_json::Value::String(item_type.to_owned()),
    );
    Some(InputItem::Json { value: parsed })
}

fn split_role_and_name(role_text: &str) -> (String, Option<String>) {
    if role_text.ends_with(')') && role_text.contains('(') {
        if let Some(open_idx) = role_text.rfind('(') {
            let possible_name = role_text[open_idx + 1..role_text.len() - 1].trim();
            let role_candidate = role_text[..open_idx].trim();
            if !possible_name.is_empty() {
                return (
                    if role_candidate.is_empty() {
                        "developer".to_owned()
                    } else {
                        role_candidate.to_owned()
                    },
                    Some(possible_name.to_owned()),
                );
            }
        }
    }
    (role_text.to_owned(), None)
}

fn should_forward_run_item(item: &RunItem) -> bool {
    !matches!(
        item,
        RunItem::ToolCall { .. } | RunItem::ToolCallOutput { .. } | RunItem::Reasoning { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::OutputItem;

    #[test]
    fn nests_history_into_summary_message() {
        let input_data = crate::handoff::HandoffInputData {
            input_history: vec![InputItem::Json {
                value: serde_json::json!({"role":"user","content":"hello"}),
            }],
            pre_handoff_items: vec![],
            new_items: vec![RunItem::MessageOutput {
                content: OutputItem::Text {
                    text: "hi".to_owned(),
                },
            }],
            input_items: None,
        };

        let nested = nest_handoff_history(input_data);
        assert_eq!(nested.input_history.len(), 1);
    }

    #[test]
    fn applies_custom_history_mapper_when_requested() {
        let input_data = crate::handoff::HandoffInputData {
            input_history: vec![InputItem::from("hello")],
            pre_handoff_items: vec![],
            new_items: vec![],
            input_items: None,
        };

        let nested = nest_handoff_history_with_mapper(
            input_data,
            Some(std::sync::Arc::new(|items| {
                let mut items = items;
                items.push(InputItem::from("mapped"));
                items
            })),
        );

        assert_eq!(nested.input_history.len(), 2);
        assert_eq!(nested.input_history[1].as_text(), Some("mapped"));
    }

    #[test]
    fn flattens_multiline_nested_history_without_truncation() {
        let original = InputItem::Json {
            value: serde_json::json!({
                "role": "user",
                "content": "first line\n2. not a new record",
            }),
        };
        let first_nested = nest_handoff_history(crate::handoff::HandoffInputData {
            input_history: vec![original.clone()],
            pre_handoff_items: vec![],
            new_items: vec![],
            input_items: None,
        });

        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured_mapper = captured.clone();
        let _ = nest_handoff_history_with_mapper(
            crate::handoff::HandoffInputData {
                input_history: first_nested.input_history,
                pre_handoff_items: vec![],
                new_items: vec![],
                input_items: None,
            },
            Some(std::sync::Arc::new(move |items| {
                *captured_mapper.lock().expect("captured transcript") = items.clone();
                items
            })),
        );

        assert_eq!(
            *captured.lock().expect("captured transcript"),
            vec![original]
        );
    }

    #[test]
    fn flattens_structured_nested_history_without_stringifying() {
        let original = InputItem::Json {
            value: serde_json::json!({
                "role": "user",
                "content": [
                    {"type": "input_text", "text": "look at this"},
                    {"type": "input_image", "image_url": "https://example.com/image.png"},
                ],
            }),
        };
        let first_nested = nest_handoff_history(crate::handoff::HandoffInputData {
            input_history: vec![original.clone()],
            pre_handoff_items: vec![],
            new_items: vec![],
            input_items: None,
        });

        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured_mapper = captured.clone();
        let _ = nest_handoff_history_with_mapper(
            crate::handoff::HandoffInputData {
                input_history: first_nested.input_history,
                pre_handoff_items: vec![],
                new_items: vec![],
                input_items: None,
            },
            Some(std::sync::Arc::new(move |items| {
                *captured_mapper.lock().expect("captured transcript") = items.clone();
                items
            })),
        );

        assert_eq!(
            *captured.lock().expect("captured transcript"),
            vec![original]
        );
    }

    #[test]
    fn flattens_legacy_multiline_summary_records() {
        let summary = InputItem::Json {
            value: serde_json::json!({
                "role": "assistant",
                "content": "For context, here is the conversation so far:\n<CONVERSATION HISTORY>\n1. user: first line\nsecond line\n2. assistant: reply\n</CONVERSATION HISTORY>",
            }),
        };

        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured_mapper = captured.clone();
        let _ = nest_handoff_history_with_mapper(
            crate::handoff::HandoffInputData {
                input_history: vec![summary],
                pre_handoff_items: vec![],
                new_items: vec![],
                input_items: None,
            },
            Some(std::sync::Arc::new(move |items| {
                *captured_mapper.lock().expect("captured transcript") = items.clone();
                items
            })),
        );

        assert_eq!(
            *captured.lock().expect("captured transcript"),
            vec![
                InputItem::Json {
                    value: serde_json::json!({"role": "user", "content": "first line\nsecond line"}),
                },
                InputItem::Json {
                    value: serde_json::json!({"role": "assistant", "content": "reply"}),
                },
            ]
        );
    }
}
