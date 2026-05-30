use std::collections::BTreeSet;

use agents_core::{LOGGER_TARGET, Result, UserError};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const UNSUPPORTED_CHOICE_STREAM_MESSAGE: &str = "Chat Completions streaming with multiple choices \
or nonzero choice indexes is not fully supported; only choice index 0 can be processed.";

const CUSTOM_TOOL_CALL_STREAM_MESSAGE: &str =
    "Custom tool calls are not supported by the Chat Completions converter";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SequenceNumber(pub u64);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Part {
    TextDelta {
        sequence: SequenceNumber,
        text: String,
    },
    ToolCallDelta {
        sequence: SequenceNumber,
        call_id: Option<String>,
        name: Option<String>,
        arguments_delta: Option<String>,
    },
    Done {
        sequence: SequenceNumber,
    },
    Unknown {
        sequence: SequenceNumber,
        payload: Value,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StreamingState {
    pub sequence: SequenceNumber,
    pub transcript: String,
}

#[derive(Clone, Debug, Default)]
pub struct ChatCmplStreamHandler {
    state: StreamingState,
    has_warned_unsupported_choice: bool,
    ignored_tool_call_indexes: BTreeSet<u64>,
}

impl ChatCmplStreamHandler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(&self) -> &StreamingState {
        &self.state
    }

    pub fn apply_chunk(&mut self, chunk: &Value) -> Vec<Part> {
        self.apply_chunk_inner(chunk, false)
            .expect("non-strict chat completion stream parsing cannot fail")
    }

    pub fn try_apply_chunk_with_strict_validation(&mut self, chunk: &Value) -> Result<Vec<Part>> {
        self.apply_chunk_inner(chunk, true)
    }

    fn apply_chunk_inner(
        &mut self,
        chunk: &Value,
        strict_feature_validation: bool,
    ) -> Result<Vec<Part>> {
        self.state.sequence.0 += 1;
        let sequence = self.state.sequence;
        let mut parts = Vec::new();
        let choices = chunk.get("choices").and_then(Value::as_array);
        let has_unsupported_choices =
            choices.is_some_and(|choices| has_unsupported_choices(choices.as_slice()));
        if has_unsupported_choices {
            self.handle_unsupported_choices(strict_feature_validation)?;
        }
        let supported_choice = choices
            .and_then(|choices| choices.iter().enumerate().find(is_choice_index_zero))
            .map(|(_, choice)| choice);

        if let Some(text) = supported_choice
            .and_then(|choice| choice.get("delta"))
            .and_then(|delta| delta.get("content"))
            .and_then(Value::as_str)
        {
            self.state.transcript.push_str(text);
            parts.push(Part::TextDelta {
                sequence,
                text: text.to_owned(),
            });
        }

        if let Some(tool_calls) = supported_choice
            .and_then(|choice| choice.get("delta"))
            .and_then(|delta| delta.get("tool_calls"))
            .and_then(Value::as_array)
        {
            for (position, tool_call) in tool_calls.iter().enumerate() {
                let tool_call_index = tool_call_index(tool_call, position);
                if self.ignored_tool_call_indexes.contains(&tool_call_index) {
                    continue;
                }

                match tool_call.get("type").and_then(Value::as_str) {
                    Some("custom") => {
                        if strict_feature_validation {
                            return Err(UserError {
                                message: CUSTOM_TOOL_CALL_STREAM_MESSAGE.to_owned(),
                            }
                            .into());
                        }
                        self.ignored_tool_call_indexes.insert(tool_call_index);
                        continue;
                    }
                    Some("function") | None => {}
                    Some(_) => continue,
                }

                parts.push(Part::ToolCallDelta {
                    sequence,
                    call_id: tool_call
                        .get("id")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    name: tool_call
                        .get("function")
                        .and_then(|f| f.get("name"))
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    arguments_delta: tool_call
                        .get("function")
                        .and_then(|f| f.get("arguments"))
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                });
            }
        }

        if supported_choice
            .and_then(|choice| choice.get("finish_reason"))
            .and_then(Value::as_str)
            .is_some()
        {
            parts.push(Part::Done { sequence });
        }

        if supported_choice.is_none() && has_unsupported_choices {
            return Ok(parts);
        }

        if parts.is_empty() {
            parts.push(Part::Unknown {
                sequence,
                payload: chunk.clone(),
            });
        }

        Ok(parts)
    }

    fn handle_unsupported_choices(&mut self, strict_feature_validation: bool) -> Result<()> {
        if strict_feature_validation {
            return Err(UserError {
                message: UNSUPPORTED_CHOICE_STREAM_MESSAGE.to_owned(),
            }
            .into());
        }

        if !self.has_warned_unsupported_choice {
            log::warn!(
                target: LOGGER_TARGET,
                "{} Ignoring the other choices; enable strict feature validation to raise an error instead.",
                UNSUPPORTED_CHOICE_STREAM_MESSAGE
            );
            self.has_warned_unsupported_choice = true;
        }

        Ok(())
    }
}

fn has_unsupported_choices(choices: &[Value]) -> bool {
    choices.len() > 1
        || choices
            .iter()
            .enumerate()
            .any(|(position, choice)| choice_index(choice, position) != 0)
}

fn is_choice_index_zero((position, choice): &(usize, &Value)) -> bool {
    choice_index(choice, *position) == 0
}

fn choice_index(choice: &Value, fallback_position: usize) -> i64 {
    choice
        .get("index")
        .and_then(Value::as_i64)
        .unwrap_or(fallback_position as i64)
}

fn tool_call_index(tool_call: &Value, fallback_position: usize) -> u64 {
    tool_call
        .get("index")
        .and_then(Value::as_u64)
        .unwrap_or(fallback_position as u64)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn extracts_text_and_tool_chunks() {
        let mut handler = ChatCmplStreamHandler::new();
        let parts = handler.apply_chunk(&json!({
            "choices": [{
                "delta": {
                    "content": "hel",
                    "tool_calls": [{"id":"call_1","function":{"name":"search","arguments":"{\"q\":\"r\"}"}}]
                }
            }]
        }));

        assert_eq!(handler.state().transcript, "hel");
        assert_eq!(parts.len(), 2);
    }

    #[test]
    fn filters_unsupported_choices_by_default() {
        let mut handler = ChatCmplStreamHandler::new();

        let ignored = handler.apply_chunk(&json!({
            "choices": [{
                "index": 1,
                "delta": {"content": "ignored"}
            }]
        }));
        assert!(
            ignored
                .iter()
                .all(|part| !matches!(part, Part::TextDelta { .. }))
        );
        assert_eq!(handler.state().transcript, "");

        let parts = handler.apply_chunk(&json!({
            "choices": [
                {"index": 0, "delta": {"content": "kept"}},
                {"index": 1, "delta": {"content": "ignored"}}
            ]
        }));

        assert_eq!(
            parts
                .iter()
                .filter_map(|part| match part {
                    Part::TextDelta { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            vec!["kept"]
        );
        assert_eq!(handler.state().transcript, "kept");
    }

    #[test]
    fn rejects_unsupported_choices_in_strict_mode() {
        let mut handler = ChatCmplStreamHandler::new();

        let error = handler
            .try_apply_chunk_with_strict_validation(&json!({
                "choices": [{
                    "index": 1,
                    "delta": {"content": "ignored"}
                }]
            }))
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("multiple choices or nonzero choice indexes")
        );
    }

    #[test]
    fn ignores_streaming_custom_tool_calls_by_default() {
        let mut handler = ChatCmplStreamHandler::new();

        let custom_start = handler.apply_chunk(&json!({
            "choices": [{
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "type": "custom",
                        "id": "call_custom"
                    }]
                }
            }]
        }));
        assert!(
            custom_start
                .iter()
                .all(|part| !matches!(part, Part::ToolCallDelta { .. }))
        );

        let custom_arguments = handler.apply_chunk(&json!({
            "choices": [{
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "function": {"arguments": "ignored"}
                    }]
                }
            }]
        }));
        assert!(
            custom_arguments
                .iter()
                .all(|part| !matches!(part, Part::ToolCallDelta { .. }))
        );

        let function_call = handler.apply_chunk(&json!({
            "choices": [{
                "delta": {
                    "tool_calls": [{
                        "index": 1,
                        "type": "function",
                        "id": "call_function",
                        "function": {"name": "search", "arguments": "{\"q\":\"r\"}"}
                    }]
                }
            }]
        }));

        assert_eq!(
            function_call
                .iter()
                .filter(|part| matches!(part, Part::ToolCallDelta { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn rejects_streaming_custom_tool_calls_in_strict_mode() {
        let mut handler = ChatCmplStreamHandler::new();

        let error = handler
            .try_apply_chunk_with_strict_validation(&json!({
                "choices": [{
                    "delta": {
                        "tool_calls": [{
                            "index": 0,
                            "type": "custom",
                            "id": "call_custom"
                        }]
                    }
                }]
            }))
            .unwrap_err();

        assert!(error.to_string().contains("Custom tool calls"));
    }
}
