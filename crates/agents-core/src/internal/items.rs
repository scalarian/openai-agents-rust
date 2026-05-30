use crate::items::{InputItem, RunItem};
use crate::run_config::ReasoningItemIdPolicy;
use serde_json::Value;

pub(crate) fn copy_input_items(items: &[InputItem]) -> Vec<InputItem> {
    items.to_vec()
}

pub(crate) fn run_item_to_input_item(
    run_item: &RunItem,
    reasoning_item_id_policy: ReasoningItemIdPolicy,
) -> Option<InputItem> {
    if matches!(
        (run_item, reasoning_item_id_policy),
        (RunItem::Reasoning { .. }, ReasoningItemIdPolicy::Omit)
    ) {
        return None;
    }
    run_item
        .to_input_item()
        .map(strip_null_status_from_input_item)
}

pub(crate) fn run_items_to_input_items(
    run_items: &[RunItem],
    reasoning_item_id_policy: ReasoningItemIdPolicy,
) -> Vec<InputItem> {
    run_items
        .iter()
        .filter_map(|run_item| run_item_to_input_item(run_item, reasoning_item_id_policy))
        .collect()
}

pub(crate) fn prepare_model_input_items(
    caller_items: &[InputItem],
    generated_items: &[RunItem],
    reasoning_item_id_policy: ReasoningItemIdPolicy,
) -> Vec<InputItem> {
    let mut prepared = copy_input_items(caller_items);
    let generated_inputs = run_items_to_input_items(generated_items, reasoning_item_id_policy);
    prepared.extend(drop_orphan_generated_tool_call_items(&generated_inputs));
    prepared
}

pub(crate) fn compose_replay_input_items(
    base_items: &[InputItem],
    generated_items: &[RunItem],
    reasoning_item_id_policy: ReasoningItemIdPolicy,
) -> Vec<InputItem> {
    let mut replay = sanitize_hosted_tool_replay_items(base_items);
    let generated_inputs = run_items_to_input_items(generated_items, reasoning_item_id_policy);
    let overlap = trailing_generated_overlap(base_items, &generated_inputs);
    replay.extend(generated_inputs.into_iter().skip(overlap));
    replay
}

fn sanitize_hosted_tool_replay_items(items: &[InputItem]) -> Vec<InputItem> {
    drop_orphan_tool_call_items(items, ToolCallPrunePolicy::HostedReplay)
}

fn strip_null_status_from_input_item(item: InputItem) -> InputItem {
    match item {
        InputItem::Json { value } => InputItem::Json {
            value: strip_null_status_from_value(value),
        },
        InputItem::Text { .. } => item,
    }
}

fn strip_null_status_from_value(value: Value) -> Value {
    let Value::Object(mut object) = value else {
        return value;
    };
    if matches!(object.get("status"), Some(Value::Null)) {
        object.remove("status");
    }
    Value::Object(object)
}

#[derive(Default)]
struct HostedPairingInfo {
    tool_call_ids: std::collections::BTreeSet<String>,
    tool_output_ids: std::collections::BTreeSet<String>,
    shell_call_ids: std::collections::BTreeSet<String>,
    shell_output_ids: std::collections::BTreeSet<String>,
    tool_search_call_ids: std::collections::BTreeSet<String>,
    tool_search_output_ids: std::collections::BTreeSet<String>,
    custom_tool_call_ids: std::collections::BTreeSet<String>,
    custom_tool_output_ids: std::collections::BTreeSet<String>,
    matched_anonymous_tool_search_call_indexes: std::collections::BTreeSet<usize>,
    matched_anonymous_tool_search_output_indexes: std::collections::BTreeSet<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ToolCallPrunePolicy {
    HostedReplay,
    GeneratedHistory,
}

fn drop_orphan_generated_tool_call_items(items: &[InputItem]) -> Vec<InputItem> {
    drop_orphan_tool_call_items(items, ToolCallPrunePolicy::GeneratedHistory)
}

fn drop_orphan_tool_call_items(items: &[InputItem], policy: ToolCallPrunePolicy) -> Vec<InputItem> {
    let hosted_pairs = hosted_pairing_info(items);
    let dropped_indexes = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            (!should_keep_replay_item(index, item, &hosted_pairs, policy)).then_some(index)
        })
        .collect::<std::collections::BTreeSet<_>>();

    if dropped_indexes.is_empty() {
        return items.to_vec();
    }

    let reasoning_indexes = reasoning_indexes_preceding_dropped_calls(items, &dropped_indexes);
    items
        .iter()
        .enumerate()
        .filter(|(index, _)| !dropped_indexes.contains(index) && !reasoning_indexes.contains(index))
        .map(|(_, item)| item.clone())
        .collect()
}

fn hosted_pairing_info(items: &[InputItem]) -> HostedPairingInfo {
    let mut info = HostedPairingInfo::default();
    let mut anonymous_tool_search_calls = Vec::new();

    for (index, item) in items.iter().enumerate() {
        let Some(value) = input_item_json(item) else {
            continue;
        };

        match item_type(value) {
            Some("tool_call") | Some("function_call") => {
                if let Some(call_id) = call_id(value) {
                    info.tool_call_ids.insert(call_id.to_owned());
                }
            }
            Some("tool_call_output") | Some("function_call_output") => {
                if let Some(call_id) = call_id(value) {
                    info.tool_output_ids.insert(call_id.to_owned());
                }
            }
            Some("custom_tool_call") => {
                if let Some(call_id) = call_id(value) {
                    info.custom_tool_call_ids.insert(call_id.to_owned());
                }
            }
            Some("custom_tool_call_output") => {
                if let Some(call_id) = call_id(value) {
                    info.custom_tool_output_ids.insert(call_id.to_owned());
                }
            }
            Some("shell_call") => {
                if let Some(call_id) = call_id(value) {
                    info.shell_call_ids.insert(call_id.to_owned());
                }
            }
            Some("shell_call_output") => {
                if let Some(call_id) = call_id(value) {
                    info.shell_output_ids.insert(call_id.to_owned());
                }
            }
            Some("tool_search_call") => match call_id(value) {
                Some(call_id) => {
                    info.tool_search_call_ids.insert(call_id.to_owned());
                }
                None => anonymous_tool_search_calls.push(index),
            },
            Some("tool_search_output") => match call_id(value) {
                Some(call_id) => {
                    info.tool_search_output_ids.insert(call_id.to_owned());
                }
                None => {
                    if let Some(call_index) = anonymous_tool_search_calls.pop() {
                        info.matched_anonymous_tool_search_call_indexes
                            .insert(call_index);
                        info.matched_anonymous_tool_search_output_indexes
                            .insert(index);
                    }
                }
            },
            _ => {}
        }
    }

    info
}

fn should_keep_replay_item(
    index: usize,
    item: &InputItem,
    pairing: &HostedPairingInfo,
    policy: ToolCallPrunePolicy,
) -> bool {
    let Some(value) = input_item_json(item) else {
        return true;
    };

    match item_type(value) {
        Some("tool_call") | Some("function_call") => {
            policy == ToolCallPrunePolicy::HostedReplay
                || call_id(value).is_some_and(|call_id| pairing.tool_output_ids.contains(call_id))
        }
        Some("tool_call_output") | Some("function_call_output") => {
            policy == ToolCallPrunePolicy::GeneratedHistory
                || call_id(value).is_some_and(|call_id| pairing.tool_call_ids.contains(call_id))
        }
        Some("custom_tool_call") => {
            policy == ToolCallPrunePolicy::HostedReplay
                || call_id(value)
                    .is_some_and(|call_id| pairing.custom_tool_output_ids.contains(call_id))
        }
        Some("custom_tool_call_output") => {
            policy == ToolCallPrunePolicy::GeneratedHistory
                || call_id(value)
                    .is_some_and(|call_id| pairing.custom_tool_call_ids.contains(call_id))
        }
        Some("shell_call") => {
            if !is_completed_hosted_item(value) {
                return true;
            }
            call_id(value).is_some_and(|call_id| pairing.shell_output_ids.contains(call_id))
        }
        Some("shell_call_output") => {
            call_id(value).is_some_and(|call_id| pairing.shell_call_ids.contains(call_id))
        }
        Some("tool_search_call") => {
            if !is_completed_hosted_item(value) {
                return true;
            }
            match call_id(value) {
                Some(call_id) => pairing.tool_search_output_ids.contains(call_id),
                None => pairing
                    .matched_anonymous_tool_search_call_indexes
                    .contains(&index),
            }
        }
        Some("tool_search_output") => match call_id(value) {
            Some(call_id) => pairing.tool_search_call_ids.contains(call_id),
            None => pairing
                .matched_anonymous_tool_search_output_indexes
                .contains(&index),
        },
        _ => true,
    }
}

fn reasoning_indexes_preceding_dropped_calls(
    items: &[InputItem],
    dropped_indexes: &std::collections::BTreeSet<usize>,
) -> std::collections::BTreeSet<usize> {
    let mut reasoning_indexes = std::collections::BTreeSet::new();

    for index in (0..items.len()).rev() {
        if !is_reasoning_item(&items[index]) || dropped_indexes.contains(&index) {
            continue;
        }

        for next_index in index + 1..items.len() {
            if reasoning_indexes.contains(&next_index) {
                continue;
            }
            if is_reasoning_item(&items[next_index]) {
                continue;
            }
            if dropped_indexes.contains(&next_index) {
                reasoning_indexes.insert(index);
            }
            break;
        }
    }

    reasoning_indexes
}

fn input_item_json(item: &InputItem) -> Option<&Value> {
    match item {
        InputItem::Text { .. } => None,
        InputItem::Json { value } => Some(value),
    }
}

fn item_type(value: &Value) -> Option<&str> {
    value.get("type").and_then(Value::as_str)
}

fn is_reasoning_item(item: &InputItem) -> bool {
    input_item_json(item).and_then(item_type) == Some("reasoning")
}

fn call_id(value: &Value) -> Option<&str> {
    value
        .get("call_id")
        .or_else(|| value.get("id"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

fn is_completed_hosted_item(value: &Value) -> bool {
    value.get("status").and_then(Value::as_str) == Some("completed")
}

fn trailing_generated_overlap(base_items: &[InputItem], generated_inputs: &[InputItem]) -> usize {
    let max_overlap = base_items.len().min(generated_inputs.len());
    (1..=max_overlap)
        .rev()
        .find(|overlap| {
            let compared_items = base_items[base_items.len() - overlap..]
                .iter()
                .zip(generated_inputs[..*overlap].iter())
                .collect::<Vec<_>>();

            compared_items
                .iter()
                .all(|(base_item, generated_item)| base_item == generated_item)
                && compared_items.iter().any(|(base_item, generated_item)| {
                    stable_replay_dedupe_key(base_item)
                        .zip(stable_replay_dedupe_key(generated_item))
                        .is_some_and(|(base_key, generated_key)| base_key == generated_key)
                })
        })
        .unwrap_or(0)
}

fn stable_replay_dedupe_key(item: &InputItem) -> Option<String> {
    let value = match item {
        InputItem::Text { .. } => return None,
        InputItem::Json { value } => value,
    };

    let item_type = value
        .get("type")
        .and_then(Value::as_str)
        .or_else(|| value.get("role").and_then(Value::as_str))?;

    if value.get("role").is_some() || item_type == "message" {
        return None;
    }

    if let Some(item_id) = value.get("id").and_then(Value::as_str) {
        if item_id != "__fake_id__" {
            return Some(format!("id:{item_type}:{item_id}"));
        }
    }

    value
        .get("call_id")
        .and_then(Value::as_str)
        .map(|call_id| format!("call:{item_type}:{call_id}"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::items::OutputItem;

    use super::*;

    #[test]
    fn prepares_model_input_from_history() {
        let input = vec![InputItem::from("hello")];
        let generated = vec![
            RunItem::Reasoning {
                text: "thinking".to_owned(),
            },
            RunItem::ToolCallOutput {
                tool_name: "search".to_owned(),
                output: OutputItem::Text {
                    text: "found".to_owned(),
                },
                call_id: Some("call-1".to_owned()),
                namespace: None,
                tool_origin: None,
            },
        ];

        let prepared =
            prepare_model_input_items(&input, &generated, ReasoningItemIdPolicy::Preserve);

        assert_eq!(prepared.len(), 3);
        assert_eq!(prepared[0].as_text(), Some("hello"));
        assert_eq!(
            prepared[1],
            InputItem::Json {
                value: json!({
                    "type": "reasoning",
                    "text": "thinking"
                })
            }
        );
        assert_eq!(
            prepared[2],
            InputItem::Json {
                value: json!({
                    "type": "tool_call_output",
                    "tool_name": "search",
                    "output": {
                        "type": "text",
                        "text": "found"
                    },
                    "call_id": "call-1",
                    "namespace": null
                })
            }
        );
    }

    #[test]
    fn omits_reasoning_items_when_policy_requests_it() {
        let prepared = prepare_model_input_items(
            &[InputItem::from("hello")],
            &[
                RunItem::Reasoning {
                    text: "thinking".to_owned(),
                },
                RunItem::ToolCallOutput {
                    tool_name: "search".to_owned(),
                    output: OutputItem::Text {
                        text: "found".to_owned(),
                    },
                    call_id: Some("call-1".to_owned()),
                    namespace: None,
                    tool_origin: None,
                },
            ],
            ReasoningItemIdPolicy::Omit,
        );

        assert_eq!(prepared.len(), 2);
        assert_eq!(prepared[0].as_text(), Some("hello"));
        assert_eq!(
            prepared[1],
            InputItem::Json {
                value: json!({
                    "type": "tool_call_output",
                    "tool_name": "search",
                    "output": {
                        "type": "text",
                        "text": "found"
                    },
                    "call_id": "call-1",
                    "namespace": null
                })
            }
        );
    }

    #[test]
    fn omits_null_status_when_replaying_generated_json_items() {
        let prepared = prepare_model_input_items(
            &[InputItem::from("hello")],
            &[
                RunItem::MessageOutput {
                    content: OutputItem::Json {
                        value: json!({
                            "type": "message",
                            "role": "assistant",
                            "content": [{"type": "output_text", "text": "done"}],
                            "status": null
                        }),
                    },
                },
                RunItem::MessageOutput {
                    content: OutputItem::Json {
                        value: json!({
                            "type": "message",
                            "role": "assistant",
                            "content": [{"type": "output_text", "text": "kept"}],
                            "status": "completed"
                        }),
                    },
                },
            ],
            ReasoningItemIdPolicy::Preserve,
        );

        assert_eq!(prepared.len(), 3);
        assert!(matches!(
            &prepared[1],
            InputItem::Json { value }
                if !value.as_object().is_some_and(|object| object.contains_key("status"))
        ));
        assert!(matches!(
            &prepared[2],
            InputItem::Json { value }
                if value.get("status").and_then(Value::as_str) == Some("completed")
        ));
    }

    #[test]
    fn drops_reasoning_preceding_dropped_generated_tool_call() {
        let prepared = prepare_model_input_items(
            &[InputItem::from("hello")],
            &[
                RunItem::Reasoning {
                    text: "orphan-a".to_owned(),
                },
                RunItem::Reasoning {
                    text: "orphan-b".to_owned(),
                },
                RunItem::ToolCall {
                    tool_name: "orphan".to_owned(),
                    arguments: json!({}),
                    call_id: Some("orphan-call".to_owned()),
                    namespace: None,
                    tool_origin: None,
                },
                RunItem::Reasoning {
                    text: "paired".to_owned(),
                },
                RunItem::ToolCall {
                    tool_name: "paired".to_owned(),
                    arguments: json!({}),
                    call_id: Some("paired-call".to_owned()),
                    namespace: None,
                    tool_origin: None,
                },
                RunItem::ToolCallOutput {
                    tool_name: "paired".to_owned(),
                    output: OutputItem::Text {
                        text: "ok".to_owned(),
                    },
                    call_id: Some("paired-call".to_owned()),
                    namespace: None,
                    tool_origin: None,
                },
            ],
            ReasoningItemIdPolicy::Preserve,
        );

        let reasoning_texts = prepared
            .iter()
            .filter_map(|item| match item {
                InputItem::Json { value }
                    if value.get("type").and_then(Value::as_str) == Some("reasoning") =>
                {
                    value.get("text").and_then(Value::as_str)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(reasoning_texts, vec!["paired"]);
        assert!(!prepared.iter().any(|item| matches!(
            item,
            InputItem::Json { value }
                if value.get("type").and_then(Value::as_str) == Some("tool_call")
                    && value.get("call_id").and_then(Value::as_str) == Some("orphan-call")
        )));
    }

    #[test]
    fn keeps_lone_reasoning_when_no_generated_tool_call_is_dropped() {
        let prepared = prepare_model_input_items(
            &[InputItem::from("hello")],
            &[RunItem::Reasoning {
                text: "standalone".to_owned(),
            }],
            ReasoningItemIdPolicy::Preserve,
        );

        assert!(prepared.iter().any(|item| matches!(
            item,
            InputItem::Json { value }
                if value.get("type").and_then(Value::as_str) == Some("reasoning")
                    && value.get("text").and_then(Value::as_str) == Some("standalone")
        )));
    }

    #[test]
    fn composes_replay_without_duplicating_overlapping_generated_items() {
        let replay = compose_replay_input_items(
            &[
                InputItem::from("filtered"),
                InputItem::Json {
                    value: json!({
                        "type": "tool_call",
                        "tool_name": "search",
                        "arguments": {"query": "rust"},
                        "call_id": "call-1",
                        "namespace": null
                    }),
                },
                InputItem::Json {
                    value: json!({
                        "type": "tool_call_output",
                        "tool_name": "search",
                        "output": {"type": "text", "text": "found"},
                        "call_id": "call-1",
                        "namespace": null
                    }),
                },
            ],
            &[
                RunItem::ToolCall {
                    tool_name: "search".to_owned(),
                    arguments: json!({"query":"rust"}),
                    call_id: Some("call-1".to_owned()),
                    namespace: None,
                    tool_origin: None,
                },
                RunItem::ToolCallOutput {
                    tool_name: "search".to_owned(),
                    output: OutputItem::Text {
                        text: "found".to_owned(),
                    },
                    call_id: Some("call-1".to_owned()),
                    namespace: None,
                    tool_origin: None,
                },
                RunItem::MessageOutput {
                    content: OutputItem::Text {
                        text: "done".to_owned(),
                    },
                },
            ],
            ReasoningItemIdPolicy::Preserve,
        );

        assert_eq!(replay[0].as_text(), Some("filtered"));
        assert_eq!(
            replay
                .iter()
                .filter(|item| item.as_text() == Some("done"))
                .count(),
            1
        );
        assert_eq!(replay.len(), 4);
    }

    #[test]
    fn composes_replay_preserving_repeated_generated_message_items() {
        let replay = compose_replay_input_items(
            &[InputItem::from("done")],
            &[RunItem::MessageOutput {
                content: OutputItem::Text {
                    text: "done".to_owned(),
                },
            }],
            ReasoningItemIdPolicy::Preserve,
        );

        assert_eq!(
            replay,
            vec![InputItem::from("done"), InputItem::from("done")]
        );
    }

    #[test]
    fn drops_orphan_hosted_tool_artifacts_and_keeps_pending_calls() {
        let replay = compose_replay_input_items(
            &[
                InputItem::Json {
                    value: json!({
                        "type": "shell_call",
                        "call_id": "shell-orphan",
                        "status": "completed",
                        "action": {"command": "echo orphan"},
                    }),
                },
                InputItem::Json {
                    value: json!({
                        "type": "tool_search_call",
                        "call_id": "search-keep",
                        "status": "completed",
                        "arguments": {"query": "rust"},
                    }),
                },
                InputItem::Json {
                    value: json!({
                        "type": "tool_search_output",
                        "call_id": "search-keep",
                        "status": "completed",
                        "tools": [],
                    }),
                },
                InputItem::Json {
                    value: json!({
                        "type": "tool_search_output",
                        "call_id": "search-orphan-output",
                        "status": "completed",
                        "tools": [],
                    }),
                },
                InputItem::Json {
                    value: json!({
                        "type": "function_call",
                        "call_id": "pending-user-call",
                        "name": "lookup",
                        "arguments": "{}",
                    }),
                },
            ],
            &[],
            ReasoningItemIdPolicy::Preserve,
        );

        assert_eq!(replay.len(), 3);
        assert!(replay.iter().any(|item| matches!(
            item,
            InputItem::Json { value }
                if value.get("type").and_then(Value::as_str) == Some("tool_search_call")
                    && value.get("call_id").and_then(Value::as_str) == Some("search-keep")
        )));
        assert!(replay.iter().any(|item| matches!(
            item,
            InputItem::Json { value }
                if value.get("type").and_then(Value::as_str) == Some("tool_search_output")
                    && value.get("call_id").and_then(Value::as_str) == Some("search-keep")
        )));
        assert!(replay.iter().any(|item| matches!(
            item,
            InputItem::Json { value }
                if value.get("type").and_then(Value::as_str) == Some("function_call")
                    && value.get("call_id").and_then(Value::as_str) == Some("pending-user-call")
        )));
        assert!(!replay.iter().any(|item| matches!(
            item,
            InputItem::Json { value }
                if value.get("call_id").and_then(Value::as_str) == Some("shell-orphan")
        )));
        assert!(!replay.iter().any(|item| matches!(
            item,
            InputItem::Json { value }
                if value.get("call_id").and_then(Value::as_str) == Some("search-orphan-output")
        )));
    }

    #[test]
    fn keeps_pending_hosted_tool_calls_without_outputs() {
        let replay = compose_replay_input_items(
            &[InputItem::Json {
                value: json!({
                    "type": "tool_search_call",
                    "call_id": "pending-tool-search",
                    "status": "pending",
                    "arguments": {"query": "continue"},
                }),
            }],
            &[],
            ReasoningItemIdPolicy::Preserve,
        );

        assert_eq!(replay.len(), 1);
        assert!(matches!(
            &replay[0],
            InputItem::Json { value }
                if value.get("call_id").and_then(Value::as_str) == Some("pending-tool-search")
        ));
    }

    #[test]
    fn prepare_model_input_preserves_caller_supplied_hosted_items() {
        let prepared = prepare_model_input_items(
            &[
                InputItem::Json {
                    value: json!({
                        "type": "shell_call",
                        "call_id": "shell-orphan",
                        "status": "completed",
                        "action": {"command": "echo keep-me"},
                    }),
                },
                InputItem::Json {
                    value: json!({
                        "type": "tool_search_output",
                        "call_id": "search-orphan-output",
                        "status": "completed",
                        "tools": [],
                    }),
                },
                InputItem::Json {
                    value: json!({
                        "type": "function_call",
                        "call_id": "pending-user-call",
                        "name": "lookup",
                        "arguments": "{}",
                    }),
                },
            ],
            &[],
            ReasoningItemIdPolicy::Preserve,
        );

        assert_eq!(prepared.len(), 3);
        assert!(prepared.iter().any(|item| matches!(
            item,
            InputItem::Json { value }
                if value.get("call_id").and_then(Value::as_str) == Some("shell-orphan")
        )));
        assert!(prepared.iter().any(|item| matches!(
            item,
            InputItem::Json { value }
                if value.get("call_id").and_then(Value::as_str) == Some("search-orphan-output")
        )));
        assert!(prepared.iter().any(|item| matches!(
            item,
            InputItem::Json { value }
                if value.get("call_id").and_then(Value::as_str) == Some("pending-user-call")
        )));
    }
}
