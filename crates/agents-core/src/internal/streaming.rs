use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};
use tokio::sync::{Mutex, Notify};

use crate::agent::Agent;
use crate::errors::Result;
use crate::items::RunItem;
use crate::model::ModelResponse;
use crate::result::RunResult;
use crate::stream_events::{
    AgentUpdatedStreamEvent, RawResponsesStreamEvent, RunItemStreamEvent, RunLifecycleStreamEvent,
    StreamEvent,
};

#[derive(Debug, Default)]
pub(crate) struct LiveRunStreamState {
    events: Mutex<Vec<StreamEvent>>,
    completion: Mutex<Option<Result<RunResult>>>,
    notify: Notify,
    revision: AtomicU64,
}

impl LiveRunStreamState {
    pub(crate) async fn event_at(&self, index: usize) -> Option<StreamEvent> {
        self.events.lock().await.get(index).cloned()
    }

    pub(crate) async fn completion(&self) -> Option<Result<RunResult>> {
        self.completion.lock().await.clone()
    }

    pub(crate) async fn wait_for_completion(&self) -> Result<RunResult> {
        loop {
            if let Some(result) = self.completion().await {
                return result;
            }
            let revision = self.revision();
            self.wait_for_change_since(revision).await;
        }
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision.load(Ordering::SeqCst)
    }

    pub(crate) async fn wait_for_change_since(&self, revision: u64) {
        loop {
            let notified = self.notify.notified();
            if self.revision() != revision {
                return;
            }
            notified.await;
            if self.revision() != revision {
                return;
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct StreamRecorder {
    state: Arc<LiveRunStreamState>,
}

impl StreamRecorder {
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(LiveRunStreamState::default()),
        }
    }

    pub(crate) fn shared_state(&self) -> Arc<LiveRunStreamState> {
        self.state.clone()
    }

    pub(crate) async fn push_event(&self, event: StreamEvent) {
        self.state.events.lock().await.push(event);
        self.state.revision.fetch_add(1, Ordering::SeqCst);
        self.state.notify.notify_waiters();
    }

    pub(crate) async fn push_lifecycle(&self, name: impl Into<String>, data: Option<Value>) {
        self.push_event(StreamEvent::Lifecycle(RunLifecycleStreamEvent {
            name: name.into(),
            data,
        }))
        .await;
    }

    pub(crate) async fn push_raw_response(&self, response: &ModelResponse) {
        self.push_event(StreamEvent::RawResponseEvent(RawResponsesStreamEvent {
            type_name: "model_response".to_owned(),
            data: serde_json::to_value(response).unwrap_or(Value::Null),
        }))
        .await;
    }

    pub(crate) async fn push_run_items(&self, items: &[RunItem]) {
        for item in items {
            self.push_event(StreamEvent::RunItemEvent(RunItemStreamEvent {
                name: run_item_name(item),
                item: item.clone(),
            }))
            .await;
        }
    }

    pub(crate) async fn push_agent_updated(&self, agent: &Agent) {
        self.push_event(StreamEvent::AgentUpdated(AgentUpdatedStreamEvent {
            new_agent: agent.clone(),
        }))
        .await;
    }

    pub(crate) async fn complete(&self, result: Result<RunResult>) {
        match &result {
            Ok(run_result) => {
                self.push_lifecycle(
                    "run_completed",
                    Some(json!({
                        "agent_name": run_result.agent_name,
                        "final_output": run_result.final_output,
                    })),
                )
                .await;
            }
            Err(error) => {
                self.push_lifecycle(
                    "run_failed",
                    Some(json!({
                        "message": error.to_string(),
                    })),
                )
                .await;
            }
        }

        *self.state.completion.lock().await = Some(result);
        self.state.revision.fetch_add(1, Ordering::SeqCst);
        self.state.notify.notify_waiters();
    }
}

pub(crate) fn result_to_stream_events(
    initial_agent: &Agent,
    result: &RunResult,
) -> Vec<StreamEvent> {
    let mut events = Vec::new();
    events.push(StreamEvent::Lifecycle(RunLifecycleStreamEvent {
        name: "agent_start".to_owned(),
        data: Some(json!({
            "agent_name": initial_agent.name,
            "turn": 0,
        })),
    }));
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
            events.push(StreamEvent::Lifecycle(RunLifecycleStreamEvent {
                name: "handoff".to_owned(),
                data: Some(json!({
                    "from_agent": initial_agent.name,
                    "to_agent": last_agent.name,
                })),
            }));
            events.push(StreamEvent::AgentUpdated(AgentUpdatedStreamEvent {
                new_agent: last_agent.clone(),
            }));
        }
        events.push(StreamEvent::Lifecycle(RunLifecycleStreamEvent {
            name: "agent_end".to_owned(),
            data: Some(json!({
                "agent_name": last_agent.name,
                "final_output": result.final_output,
            })),
        }));
    }
    events.push(StreamEvent::Lifecycle(RunLifecycleStreamEvent {
        name: "run_completed".to_owned(),
        data: Some(json!({
            "agent_name": result.agent_name,
            "final_output": result.final_output,
        })),
    }));
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

#[cfg(test)]
mod tests {
    use tokio::time::{Duration, timeout};

    use super::*;

    #[tokio::test]
    async fn live_run_stream_wait_for_change_does_not_miss_completed_event_revisions() {
        let recorder = StreamRecorder::new();
        let state = recorder.shared_state();
        let revision = state.revision();

        recorder.push_lifecycle("tool_start", None).await;

        timeout(
            Duration::from_millis(100),
            state.wait_for_change_since(revision),
        )
        .await
        .expect("wait should observe completed revision change");
        assert!(state.event_at(0).await.is_some());
    }

    #[tokio::test]
    async fn live_run_stream_wait_for_completion_does_not_miss_completed_results() {
        let recorder = StreamRecorder::new();
        let state = recorder.shared_state();

        recorder
            .complete(Ok(RunResult {
                agent_name: "assistant".to_owned(),
                final_output: Some("done".to_owned()),
                ..RunResult::default()
            }))
            .await;

        let result = timeout(Duration::from_millis(100), state.wait_for_completion())
            .await
            .expect("wait should observe completed result")
            .expect("completion should succeed");
        assert_eq!(result.final_output.as_deref(), Some("done"));
    }
}
