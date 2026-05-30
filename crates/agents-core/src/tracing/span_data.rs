use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SpanData {
    Agent(AgentSpanData),
    Function(FunctionSpanData),
    Generation(GenerationSpanData),
    Response(ResponseSpanData),
    Handoff(HandoffSpanData),
    Custom(CustomSpanData),
    Guardrail(GuardrailSpanData),
    MpcListTools(MCPListToolsSpanData),
    SpeechGroup(SpeechGroupSpanData),
    Speech(SpeechSpanData),
    Transcription(TranscriptionSpanData),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AgentSpanData {
    pub name: String,
    pub handoffs: Option<Vec<String>>,
    pub tools: Option<Vec<String>>,
    pub output_type: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FunctionSpanData {
    pub name: String,
    pub input: Option<String>,
    pub output: Option<String>,
    pub mcp_data: Option<Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GenerationSpanData {
    pub input: Option<Vec<Value>>,
    pub output: Option<Vec<Value>>,
    pub model: Option<String>,
    pub model_config: Option<BTreeMap<String, Value>>,
    pub usage: Option<Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResponseSpanData {
    pub response_id: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TaskSpanData {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Value>,
}

impl TaskSpanData {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            usage: None,
        }
    }

    pub fn with_usage(mut self, usage: Value) -> Self {
        self.usage = Some(usage);
        self
    }

    pub fn into_span_data(self) -> SpanData {
        let mut data = BTreeMap::new();
        data.insert("sdk_span_type".to_owned(), Value::String("task".to_owned()));
        data.insert("name".to_owned(), Value::String(self.name));
        if let Some(usage) = self.usage {
            data.insert("usage".to_owned(), usage);
        }
        SpanData::Custom(CustomSpanData {
            name: "task".to_owned(),
            data,
        })
    }
}

impl From<TaskSpanData> for SpanData {
    fn from(value: TaskSpanData) -> Self {
        value.into_span_data()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TurnSpanData {
    pub turn: usize,
    pub agent_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Value>,
}

impl TurnSpanData {
    pub fn new(turn: usize, agent_name: impl Into<String>) -> Self {
        Self {
            turn,
            agent_name: agent_name.into(),
            usage: None,
        }
    }

    pub fn with_usage(mut self, usage: Value) -> Self {
        self.usage = Some(usage);
        self
    }

    pub fn into_span_data(self) -> SpanData {
        let mut data = BTreeMap::new();
        data.insert("sdk_span_type".to_owned(), Value::String("turn".to_owned()));
        data.insert(
            "turn".to_owned(),
            Value::Number(serde_json::Number::from(self.turn)),
        );
        data.insert("agent_name".to_owned(), Value::String(self.agent_name));
        if let Some(usage) = self.usage {
            data.insert("usage".to_owned(), usage);
        }
        SpanData::Custom(CustomSpanData {
            name: "turn".to_owned(),
            data,
        })
    }
}

impl From<TurnSpanData> for SpanData {
    fn from(value: TurnSpanData) -> Self {
        value.into_span_data()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HandoffSpanData {
    pub from_agent: Option<String>,
    pub to_agent: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CustomSpanData {
    pub name: String,
    #[serde(default)]
    pub data: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GuardrailSpanData {
    pub name: String,
    pub triggered: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MCPListToolsSpanData {
    pub server: String,
    #[serde(default)]
    pub tools: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SpeechGroupSpanData {
    pub name: String,
    pub voice: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SpeechSpanData {
    pub name: String,
    pub transcript: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TranscriptionSpanData {
    pub name: String,
    pub language: Option<String>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn task_span_data_exports_as_custom_sdk_span() {
        let span_data = TaskSpanData::new("Agent workflow")
            .with_usage(json!({
                "input_tokens": 20,
                "output_tokens": 6,
            }))
            .into_span_data();

        assert_eq!(
            serde_json::to_value(span_data).expect("span data should serialize"),
            json!({
                "type": "custom",
                "name": "task",
                "data": {
                    "sdk_span_type": "task",
                    "name": "Agent workflow",
                    "usage": {
                        "input_tokens": 20,
                        "output_tokens": 6,
                    },
                },
            })
        );
    }

    #[test]
    fn turn_span_data_exports_as_custom_sdk_span() {
        let span_data = TurnSpanData::new(2, "assistant")
            .with_usage(json!({
                "input_tokens": 10,
                "output_tokens": 3,
            }))
            .into_span_data();

        assert_eq!(
            serde_json::to_value(span_data).expect("span data should serialize"),
            json!({
                "type": "custom",
                "name": "turn",
                "data": {
                    "sdk_span_type": "turn",
                    "turn": 2,
                    "agent_name": "assistant",
                    "usage": {
                        "input_tokens": 10,
                        "output_tokens": 3,
                    },
                },
            })
        );
    }
}
