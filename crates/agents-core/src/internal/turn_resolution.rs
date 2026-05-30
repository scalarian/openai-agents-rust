use crate::items::{OutputItem, RunItem};

pub(crate) fn extract_text_outputs(output: &[OutputItem]) -> Vec<String> {
    output
        .iter()
        .filter_map(|item| match item {
            OutputItem::Text { text } => Some(text.clone()),
            OutputItem::Refusal { .. }
            | OutputItem::Json { .. }
            | OutputItem::ToolCall { .. }
            | OutputItem::Handoff { .. }
            | OutputItem::Reasoning { .. } => None,
        })
        .collect()
}

pub(crate) fn extract_final_output_text(output: &[OutputItem]) -> Option<String> {
    extract_text_outputs(output).into_iter().next()
}

pub(crate) fn extract_refusal(output: &[OutputItem]) -> Option<String> {
    let refusal = output
        .iter()
        .filter_map(|item| match item {
            OutputItem::Refusal { refusal } => Some(refusal.as_str()),
            OutputItem::Json { value } => refusal_from_json_message(value),
            OutputItem::Text { .. }
            | OutputItem::ToolCall { .. }
            | OutputItem::Handoff { .. }
            | OutputItem::Reasoning { .. } => None,
        })
        .collect::<String>();
    (!refusal.is_empty()).then_some(refusal)
}

fn refusal_from_json_message(value: &serde_json::Value) -> Option<&str> {
    match value.get("type").and_then(serde_json::Value::as_str) {
        Some("refusal") => value.get("refusal").and_then(serde_json::Value::as_str),
        Some("message") => value
            .get("content")
            .and_then(serde_json::Value::as_array)?
            .iter()
            .find_map(refusal_from_json_message),
        _ => None,
    }
}

pub(crate) fn build_message_output_items(output: &[OutputItem]) -> Vec<RunItem> {
    output
        .iter()
        .cloned()
        .map(|content| match content {
            OutputItem::ToolCall {
                call_id,
                tool_name,
                arguments,
                namespace,
            } => RunItem::ToolCall {
                tool_name,
                arguments,
                call_id: Some(call_id),
                namespace,
            },
            OutputItem::Handoff { target_agent } => RunItem::HandoffCall { target_agent },
            OutputItem::Reasoning { text } => RunItem::Reasoning { text },
            content => RunItem::MessageOutput { content },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_run_items_from_output() {
        let output = vec![OutputItem::Text {
            text: "hello".to_owned(),
        }];

        let items = build_message_output_items(&output);

        assert_eq!(items.len(), 1);
        assert_eq!(extract_final_output_text(&output).as_deref(), Some("hello"));
    }

    #[test]
    fn extracts_refusal_from_output_items() {
        let output = vec![
            OutputItem::Refusal {
                refusal: "no".to_owned(),
            },
            OutputItem::Json {
                value: serde_json::json!({
                    "type": "message",
                    "content": [{"type": "refusal", "refusal": " thanks"}],
                }),
            },
        ];

        assert_eq!(extract_refusal(&output).as_deref(), Some("no thanks"));
    }
}
