use serde::{Deserialize, Serialize};
use serde_json::Value;

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
}

impl ChatCmplStreamHandler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(&self) -> &StreamingState {
        &self.state
    }

    pub fn apply_chunk(&mut self, chunk: &Value) -> Vec<Part> {
        self.state.sequence.0 += 1;
        let sequence = self.state.sequence;
        let mut parts = Vec::new();

        if let Some(text) = chunk
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
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

        if let Some(tool_calls) = chunk
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("delta"))
            .and_then(|delta| delta.get("tool_calls"))
            .and_then(Value::as_array)
        {
            for tool_call in tool_calls {
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

        if chunk
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("finish_reason"))
            .and_then(Value::as_str)
            .is_some()
        {
            parts.push(Part::Done { sequence });
        }

        if parts.is_empty() {
            parts.push(Part::Unknown {
                sequence,
                payload: chunk.clone(),
            });
        }

        parts
    }
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
}
