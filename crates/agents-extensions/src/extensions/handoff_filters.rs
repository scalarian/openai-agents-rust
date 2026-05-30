use std::sync::Arc;

use agents_core::{HandoffInputData, HandoffInputFilter, InputItem, RunItem};

fn is_filtered_run_item(item: &RunItem) -> bool {
    matches!(
        item,
        RunItem::ToolCall { .. }
            | RunItem::ToolCallOutput { .. }
            | RunItem::CustomToolCall { .. }
            | RunItem::CustomToolCallOutput { .. }
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

/// Removes tool, handoff, and reasoning items from all handoff input buckets.
pub fn remove_all_tools_from_handoff_input(data: HandoffInputData) -> HandoffInputData {
    HandoffInputData {
        input_history: remove_tool_types_from_input(&data.input_history),
        pre_handoff_items: remove_all_tools(&data.pre_handoff_items),
        new_items: remove_all_tools(&data.new_items),
        input_items: data
            .input_items
            .map(|input_items| remove_all_tools(&input_items)),
    }
}

/// Returns a handoff input filter that removes tool, handoff, and reasoning items.
pub fn remove_all_tools_handoff_filter() -> HandoffInputFilter {
    Arc::new(|data| Box::pin(async move { remove_all_tools_from_handoff_input(data) }))
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
                tool_origin: None,
            },
            RunItem::CustomToolCall {
                tool_name: "raw_editor".to_owned(),
                input: "hello".to_owned(),
                call_id: Some("call-custom".to_owned()),
            },
            RunItem::CustomToolCallOutput {
                output: "HELLO".to_owned(),
                call_id: Some("call-custom".to_owned()),
                tool_name: Some("raw_editor".to_owned()),
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

    #[test]
    fn handoff_filter_preserves_and_filters_input_items() {
        let keep_history = InputItem::from("history");
        let keep_pre = RunItem::MessageOutput {
            content: OutputItem::Text {
                text: "pre".to_owned(),
            },
        };
        let keep_new = RunItem::MessageOutput {
            content: OutputItem::Text {
                text: "new".to_owned(),
            },
        };
        let keep_input = RunItem::MessageOutput {
            content: OutputItem::Text {
                text: "input".to_owned(),
            },
        };

        let filtered = remove_all_tools_from_handoff_input(HandoffInputData {
            input_history: vec![
                keep_history.clone(),
                InputItem::Json {
                    value: json!({
                        "type": "function_call",
                        "call_id": "call-1",
                    }),
                },
            ],
            pre_handoff_items: vec![
                keep_pre.clone(),
                RunItem::ToolCall {
                    tool_name: "search".to_owned(),
                    arguments: json!({"q":"rust"}),
                    call_id: Some("call-1".to_owned()),
                    namespace: None,
                    tool_origin: None,
                },
            ],
            new_items: vec![
                RunItem::HandoffOutput {
                    source_agent: "triage".to_owned(),
                },
                keep_new.clone(),
            ],
            input_items: Some(vec![
                RunItem::Reasoning {
                    text: "thinking".to_owned(),
                },
                keep_input.clone(),
            ]),
        });

        assert_eq!(filtered.input_history, vec![keep_history]);
        assert_eq!(filtered.pre_handoff_items, vec![keep_pre]);
        assert_eq!(filtered.new_items, vec![keep_new]);
        assert_eq!(filtered.input_items, Some(vec![keep_input]));
    }
}
