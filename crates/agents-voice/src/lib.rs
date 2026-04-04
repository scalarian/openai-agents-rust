//! Voice pipeline scaffolding.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use agents_core::Result;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AudioInput {
    pub mime_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StreamedAudioResult {
    pub transcript: Vec<String>,
    pub audio_chunks: usize,
}

#[async_trait]
pub trait VoiceWorkflow: Send + Sync {
    async fn run(&self, input_text: String) -> Result<Vec<String>>;
}

#[derive(Clone, Debug, Default)]
pub struct VoicePipeline;

impl VoicePipeline {
    pub async fn run<W: VoiceWorkflow>(
        &self,
        workflow: &W,
        _input: AudioInput,
    ) -> Result<StreamedAudioResult> {
        let transcript = workflow.run(String::new()).await?;
        Ok(StreamedAudioResult {
            audio_chunks: transcript.len(),
            transcript,
        })
    }
}
