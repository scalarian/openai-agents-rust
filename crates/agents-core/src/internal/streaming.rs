use crate::agent::Agent;
use crate::result::RunResult;
use crate::stream_events::{
    AgentUpdatedStreamEvent, RawResponsesStreamEvent, RunItemStreamEvent, StreamEvent,
};

pub(crate) fn result_to_stream_events(
    initial_agent: &Agent,
    result: &RunResult,
) -> Vec<StreamEvent> {
    let mut events = Vec::new();
    for response in &result.raw_responses {
        events.push(StreamEvent::RawResponseEvent(RawResponsesStreamEvent {
            type_name: "model_response".to_owned(),
            data: serde_json::to_value(response).unwrap_or(serde_json::Value::Null),
        }));
    }
    for item in &result.new_items {
        events.push(StreamEvent::RunItemEvent(RunItemStreamEvent {
            name: run_item_name(item),
            item: item.clone(),
        }));
    }
    if let Some(last_agent) = &result.last_agent {
        if last_agent.name != initial_agent.name {
            events.push(StreamEvent::AgentUpdated(AgentUpdatedStreamEvent {
                new_agent: last_agent.clone(),
            }));
        }
    }
    events
}

fn run_item_name(item: &crate::items::RunItem) -> String {
    match item {
        crate::items::RunItem::MessageOutput { .. } => "message_output".to_owned(),
        crate::items::RunItem::ToolCall { .. } => "tool_call".to_owned(),
        crate::items::RunItem::ToolCallOutput { .. } => "tool_call_output".to_owned(),
        crate::items::RunItem::HandoffCall { .. } => "handoff_call".to_owned(),
        crate::items::RunItem::HandoffOutput { .. } => "handoff_output".to_owned(),
        crate::items::RunItem::Reasoning { .. } => "reasoning".to_owned(),
    }
}
