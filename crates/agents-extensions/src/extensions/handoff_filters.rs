use agents_core::{InputItem, RunItem};

fn is_filtered_run_item(item: &RunItem) -> bool {
    matches!(
        item,
        RunItem::ToolCall { .. }
            | RunItem::ToolCallOutput { .. }
            | RunItem::HandoffCall { .. }
            | RunItem::HandoffOutput { .. }
            | RunItem::Reasoning { .. }
    )
}

fn is_filtered_input_item(item: &InputItem) -> bool {
    let InputItem::Json { value } = item else {
        return false;
    };
    matches!(
        value.get("type").and_then(serde_json::Value::as_str),
        Some(
            "function_call"
                | "function_call_output"
                | "computer_call"
                | "computer_call_output"
                | "file_search_call"
                | "tool_search_call"
                | "tool_search_output"
                | "web_search_call"
                | "mcp_call"
                | "mcp_list_tools"
                | "mcp_approval_request"
                | "mcp_approval_response"
                | "reasoning"
                | "code_interpreter_call"
                | "image_generation_call"
                | "local_shell_call"
                | "local_shell_call_output"
                | "shell_call"
                | "shell_call_output"
                | "apply_patch_call"
                | "apply_patch_call_output"
                | "custom_tool_call"
                | "custom_tool_call_output"
                | "hosted_tool_call"
                | "tool_call"
                | "tool_call_output"
                | "handoff_call"
                | "handoff_output"
        )
    )
}

/// Removes tool, handoff, and reasoning items from replayable run history.
pub fn remove_all_tools(items: &[RunItem]) -> Vec<RunItem> {
    items
        .iter()
        .filter(|item| !is_filtered_run_item(item))
        .cloned()
        .collect()
}

/// Removes tool and reasoning records from model input history.
pub fn remove_tool_types_from_input(items: &[InputItem]) -> Vec<InputItem> {
    items
        .iter()
        .filter(|item| !is_filtered_input_item(item))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use agents_core::OutputItem;
    use serde_json::json;

    use super::*;

    #[test]
    fn filters_toolish_run_items() {
        let items = vec![
            RunItem::MessageOutput {
                content: OutputItem::Text {
                    text: "hello".to_owned(),
                },
            },
            RunItem::ToolCall {
                tool_name: "search".to_owned(),
                arguments: json!({"q":"rust"}),
                call_id: None,
                namespace: None,
            },
            RunItem::Reasoning {
                text: "thinking".to_owned(),
            },
        ];

        let filtered = remove_all_tools(&items);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn filters_hosted_tool_calls_from_input_items() {
        let items = vec![
            InputItem::from("keep"),
            InputItem::Json {
                value: json!({
                    "type": "hosted_tool_call",
                    "id": "hosted-1",
                }),
            },
            InputItem::Json {
                value: json!({
                    "type": "custom_tool_call_output",
                    "id": "custom-1",
                }),
            },
            InputItem::Json {
                value: json!({
                    "type": "message",
                    "content": "also keep",
                }),
            },
        ];

        let filtered = remove_tool_types_from_input(&items);

        assert_eq!(filtered.len(), 2);
        assert!(matches!(filtered[0], InputItem::Text { .. }));
        assert!(matches!(
            &filtered[1],
            InputItem::Json { value }
                if value.get("type").and_then(serde_json::Value::as_str) == Some("message")
        ));
    }
}
