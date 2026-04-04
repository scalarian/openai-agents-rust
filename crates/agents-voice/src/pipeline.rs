use agents_core::Result;

use crate::events::VoiceStreamEvent;
use crate::input::AudioInput;
use crate::pipeline_config::VoicePipelineConfig;
use crate::result::StreamedAudioResult;
use crate::workflow::VoiceWorkflowBase;

#[derive(Clone, Debug, Default)]
pub struct VoicePipeline {
    config: VoicePipelineConfig,
}

impl VoicePipeline {
    pub fn new(config: VoicePipelineConfig) -> Self {
        Self { config }
    }

    pub async fn run<W: VoiceWorkflowBase>(
        &self,
        workflow: &W,
        input: AudioInput,
    ) -> Result<StreamedAudioResult> {
        let events = workflow.run(input).await?;
        let transcript = events
            .iter()
            .filter_map(|event| match event {
                VoiceStreamEvent::Audio(_) => None,
                VoiceStreamEvent::Lifecycle(_) => None,
                VoiceStreamEvent::Error(error) => Some(error.error.clone()),
            })
            .collect::<Vec<_>>();
        let audio_chunks = if self.config.stream_audio {
            events
                .iter()
                .filter(|event| matches!(event, VoiceStreamEvent::Audio(_)))
                .count()
        } else {
            0
        };

        Ok(StreamedAudioResult {
            transcript,
            audio_chunks,
            events,
        })
    }
}
