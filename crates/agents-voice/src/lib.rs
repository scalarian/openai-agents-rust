//! Voice models, workflow, and pipeline primitives.

mod events;
mod exceptions;
mod input;
mod model;
#[path = "models/openai_model_provider.rs"]
mod openai_model_provider;
#[path = "models/openai_stt.rs"]
mod openai_stt;
#[path = "models/openai_tts.rs"]
mod openai_tts;
mod pipeline;
mod pipeline_config;
mod result;
mod utils;
mod workflow;

pub use events::{
    VoiceStreamEvent, VoiceStreamEventAudio, VoiceStreamEventError, VoiceStreamEventLifecycle,
};
pub use exceptions::STTWebsocketConnectionError;
pub use input::{AudioInput, StreamedAudioInput};
pub use model::{
    STTModel, STTModelSettings, StreamedTranscriptionSession, TTSModel, TTSModelSettings,
    VoiceModelProvider,
};
pub use openai_model_provider::{OpenAIVoiceModelProvider, shared_http_client};
pub use openai_stt::{
    ErrorSentinel, OpenAISTTModel, OpenAISTTTranscriptionSession, SessionCompleteSentinel,
    WebsocketDoneSentinel,
};
pub use openai_tts::OpenAITTSModel;
pub use pipeline::VoicePipeline;
pub use pipeline_config::VoicePipelineConfig;
pub use result::StreamedAudioResult;
pub use utils::get_sentence_based_splitter;
pub use workflow::{
    SingleAgentVoiceWorkflow, SingleAgentWorkflowCallbacks, VoiceWorkflowBase, VoiceWorkflowHelper,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pipeline_runs_single_agent_workflow() {
        let workflow = SingleAgentVoiceWorkflow::new(|_input| async move {
            Ok(vec![VoiceStreamEvent::Lifecycle(
                VoiceStreamEventLifecycle {
                    event: "turn_ended".to_owned(),
                },
            )])
        });
        let pipeline = VoicePipeline::new(VoicePipelineConfig::default());
        let result = pipeline
            .run(
                &workflow,
                AudioInput {
                    mime_type: "audio/wav".to_owned(),
                    bytes: vec![1, 2, 3],
                },
            )
            .await
            .expect("pipeline should succeed");

        assert_eq!(result.audio_chunks, 0);
        assert_eq!(result.events.len(), 1);
    }
}
