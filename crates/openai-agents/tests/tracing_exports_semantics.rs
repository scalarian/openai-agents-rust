use openai_agents::{SpanData, TaskSpanData, TurnSpanData, task_span, turn_span};
use serde_json::json;

#[test]
fn facade_exports_task_and_turn_tracing_helpers() {
    let task_data: SpanData = TaskSpanData::new("Agent workflow")
        .with_usage(json!({
            "input_tokens": 20,
            "output_tokens": 6,
        }))
        .into();
    let turn_data: SpanData = TurnSpanData::new(2, "assistant")
        .with_usage(json!({
            "input_tokens": 10,
            "output_tokens": 3,
        }))
        .into();

    assert_eq!(
        serde_json::to_value(task_data).expect("task span data should serialize"),
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
    assert_eq!(
        serde_json::to_value(turn_data).expect("turn span data should serialize"),
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

    assert_eq!(task_span("Agent workflow").name, "task");
    assert_eq!(turn_span(2, "assistant").name, "turn");
}
