//! Realtime runtime types and transports.

mod _default_tracker;
mod _util;
mod agent;
mod audio_formats;
mod config;
mod events;
mod handoffs;
mod items;
mod model;
mod model_events;
mod model_inputs;
mod openai_realtime;
mod runner;
mod session;

pub use _default_tracker::{ModelAudioState, ModelAudioTracker};
pub use _util::calculate_audio_length_ms;
pub use agent::{RealtimeAgent, RealtimeAgentHooks, RealtimeRunHooks};
pub use audio_formats::{RealtimeAudioFormat, to_realtime_audio_format};
pub use config::{
    RealtimeAudioConfig, RealtimeAudioInputConfig, RealtimeAudioOutputConfig,
    RealtimeClientMessage, RealtimeGuardrailsSettings, RealtimeInputAudioNoiseReductionConfig,
    RealtimeInputAudioTranscriptionConfig, RealtimeModelTracingConfig, RealtimeRunConfig,
    RealtimeSessionModelSettings, RealtimeTurnDetectionConfig,
};
pub use events::{
    RealtimeAgentEndEvent, RealtimeAgentStartEvent, RealtimeErrorEvent, RealtimeEvent,
    RealtimeEventInfo, RealtimeHandoffEvent, RealtimeRawModelEvent, RealtimeToolApprovalRequired,
    RealtimeToolEnd, RealtimeToolStart, RealtimeTranscriptDeltaEvent,
};
pub use handoffs::realtime_handoff;
pub use items::{
    AssistantAudio, AssistantMessageItem, AssistantText, InputAudio, InputImage, InputText,
    RealtimeItem, SystemMessageItem, ToolCallItem, ToolCallOutputItem, UserMessageItem,
};
pub use model::{
    RealtimeModel, RealtimeModelConfig, RealtimeModelListener, RealtimePlaybackState,
    RealtimePlaybackTracker,
};
pub use model_events::{
    RealtimeModelAudioDoneEvent, RealtimeModelAudioEvent, RealtimeModelAudioInterruptedEvent,
    RealtimeModelErrorEvent, RealtimeModelEvent, RealtimeModelResponseDoneEvent,
    RealtimeModelToolCallEvent, RealtimeModelTranscriptDeltaEvent,
};
pub use model_inputs::{
    RealtimeModelInputImageContent, RealtimeModelInputTextContent, RealtimeModelRawClientMessage,
    RealtimeModelSendAudio, RealtimeModelSendRawMessage, RealtimeModelSendToolOutput,
    RealtimeModelSendUserInput, RealtimeModelUserInputMessage,
};
pub use openai_realtime::{
    OpenAIRealtimeSIPModel, OpenAIRealtimeWebSocketModel, TransportConfig, get_api_key,
    get_server_event_type_adapter,
};
pub use runner::RealtimeRunner;
pub use session::RealtimeSession;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn runner_records_text_turns() {
        let agent = RealtimeAgent::new("assistant");
        let mut session = RealtimeSession::new(Some("gpt-realtime".to_owned()));
        let runner = RealtimeRunner::new(agent);

        let event = runner
            .run_text_turn(&mut session, "hello")
            .await
            .expect("turn should succeed");

        assert!(matches!(event, RealtimeEvent::TranscriptDelta(_)));
        assert_eq!(session.transcript(), "hello");
    }
}
