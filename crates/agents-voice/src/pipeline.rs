use std::sync::Arc;

use agents_core::Result;
use futures::StreamExt;

use crate::events::{VoiceStreamEvent, VoiceStreamEventError, VoiceStreamEventLifecycle};
use crate::input::{AudioInput, StreamedAudioInput};
use crate::model::{TTSModel, TTSModelSettings, VoiceModelProvider};
use crate::openai_model_provider::OpenAIVoiceModelProvider;
use crate::pipeline_config::VoicePipelineConfig;
use crate::result::{StreamedAudioResult, VoiceStreamRecorder};
use crate::workflow::VoiceWorkflowBase;

#[cfg(test)]
use async_trait::async_trait;
#[cfg(test)]
use tokio::sync::Mutex;

#[cfg(test)]
use crate::model::{STTModel, STTModelSettings, StreamedTranscriptionSession};

#[derive(Clone)]
pub struct VoicePipeline {
    config: VoicePipelineConfig,
    model_provider: Arc<dyn VoiceModelProvider>,
}

impl Default for VoicePipeline {
    fn default() -> Self {
        Self::new(VoicePipelineConfig::default())
    }
}

impl VoicePipeline {
    pub fn new(config: VoicePipelineConfig) -> Self {
        Self {
            config,
            model_provider: Arc::new(OpenAIVoiceModelProvider::default()),
        }
    }

    pub fn with_model_provider(mut self, model_provider: Arc<dyn VoiceModelProvider>) -> Self {
        self.model_provider = model_provider;
        self
    }

    pub async fn run<W: VoiceWorkflowBase + Clone + 'static>(
        &self,
        workflow: &W,
        input: AudioInput,
    ) -> Result<StreamedAudioResult> {
        let stt_model = self.model_provider.stt_model();
        let tts_model = self.model_provider.tts_model();
        let transcription = stt_model
            .transcribe(&input, &self.config.stt_settings)
            .await?;
        self.run_transcription(workflow, transcription, tts_model)
            .await
    }

    pub async fn run_streamed_audio_input<W: VoiceWorkflowBase + Clone + 'static>(
        &self,
        workflow: &W,
        input: StreamedAudioInput,
    ) -> Result<StreamedAudioResult> {
        let stt_model = self.model_provider.stt_model();
        let tts_model = self.model_provider.tts_model();
        let mut session = stt_model.start_session(&self.config.stt_settings).await?;
        for chunk in input.chunks {
            session.push_audio(&chunk).await?;
        }
        let transcription = session.finish().await?;
        self.run_transcription(workflow, transcription, tts_model)
            .await
    }

    async fn run_transcription<W: VoiceWorkflowBase + Clone + 'static>(
        &self,
        workflow: &W,
        transcription: String,
        tts_model: Box<dyn TTSModel>,
    ) -> Result<StreamedAudioResult> {
        let recorder = VoiceStreamRecorder::new(self.config.stream_audio);
        let result = recorder.result();
        let workflow = workflow.clone();
        let stream_audio = self.config.stream_audio;
        let tts_settings = self.config.tts_settings.clone();

        tokio::spawn(async move {
            recorder
                .push_events(vec![
                    VoiceStreamEvent::Lifecycle(VoiceStreamEventLifecycle {
                        event: "started".to_owned(),
                    }),
                    VoiceStreamEvent::Lifecycle(VoiceStreamEventLifecycle {
                        event: "session_started".to_owned(),
                    }),
                ])
                .await;

            let completion = async {
                let mut intro = Box::pin(workflow.on_start());
                while let Some(chunk) = intro.next().await {
                    synthesize_chunk(
                        &recorder,
                        tts_model.as_ref(),
                        chunk?,
                        &tts_settings,
                        stream_audio,
                    )
                    .await?;
                }

                let mut text_stream = Box::pin(workflow.run(transcription));
                while let Some(chunk) = text_stream.next().await {
                    synthesize_chunk(
                        &recorder,
                        tts_model.as_ref(),
                        chunk?,
                        &tts_settings,
                        stream_audio,
                    )
                    .await?;
                }

                Result::<()>::Ok(())
            }
            .await;

            match completion {
                Ok(()) => {
                    recorder
                        .push_events(vec![
                            VoiceStreamEvent::Lifecycle(VoiceStreamEventLifecycle {
                                event: "completed".to_owned(),
                            }),
                            VoiceStreamEvent::Lifecycle(VoiceStreamEventLifecycle {
                                event: "session_ended".to_owned(),
                            }),
                        ])
                        .await;
                    recorder.complete().await;
                }
                Err(error) => {
                    recorder
                        .push_events(vec![VoiceStreamEvent::Error(VoiceStreamEventError {
                            error: error.to_string(),
                        })])
                        .await;
                    recorder.fail(error).await;
                }
            }
        });

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{VoiceStreamEvent, VoiceStreamEventAudio};
    use futures::stream::{self, BoxStream};

    #[derive(Clone, Default)]
    struct RecordingProvider {
        stt_settings: Arc<Mutex<Vec<STTModelSettings>>>,
        tts_settings: Arc<Mutex<Vec<TTSModelSettings>>>,
    }

    impl VoiceModelProvider for RecordingProvider {
        fn stt_model(&self) -> Box<dyn STTModel> {
            Box::new(RecordingSttModel {
                settings: self.stt_settings.clone(),
            })
        }

        fn tts_model(&self) -> Box<dyn TTSModel> {
            Box::new(RecordingTtsModel {
                settings: self.tts_settings.clone(),
            })
        }
    }

    struct RecordingSttModel {
        settings: Arc<Mutex<Vec<STTModelSettings>>>,
    }

    #[async_trait]
    impl STTModel for RecordingSttModel {
        async fn transcribe(
            &self,
            _input: &AudioInput,
            settings: &STTModelSettings,
        ) -> Result<String> {
            self.settings.lock().await.push(settings.clone());
            Ok("normalized transcript".to_owned())
        }

        async fn start_session(
            &self,
            settings: &STTModelSettings,
        ) -> Result<Box<dyn StreamedTranscriptionSession>> {
            self.settings.lock().await.push(settings.clone());
            Ok(Box::new(RecordingSession::default()))
        }
    }

    #[derive(Default)]
    struct RecordingSession {
        transcript: String,
    }

    #[async_trait]
    impl StreamedTranscriptionSession for RecordingSession {
        async fn push_audio(&mut self, chunk: &[u8]) -> Result<()> {
            self.transcript.push_str(&format!("[{}]", chunk.len()));
            Ok(())
        }

        async fn finish(&mut self) -> Result<String> {
            Ok(std::mem::take(&mut self.transcript))
        }
    }

    struct RecordingTtsModel {
        settings: Arc<Mutex<Vec<TTSModelSettings>>>,
    }

    #[async_trait]
    impl TTSModel for RecordingTtsModel {
        async fn synthesize(
            &self,
            _text: &str,
            settings: &TTSModelSettings,
        ) -> Result<Vec<VoiceStreamEvent>> {
            self.settings.lock().await.push(settings.clone());
            Ok(vec![VoiceStreamEvent::Audio(VoiceStreamEventAudio {
                data: Some(vec![1.0]),
            })])
        }
    }

    struct FailingTtsModel {
        calls: Arc<Mutex<usize>>,
    }

    #[async_trait]
    impl TTSModel for FailingTtsModel {
        async fn synthesize(
            &self,
            _text: &str,
            _settings: &TTSModelSettings,
        ) -> Result<Vec<VoiceStreamEvent>> {
            let mut calls = self.calls.lock().await;
            *calls += 1;
            Err(agents_core::AgentsError::message(
                "tts should be skipped when audio streaming is disabled",
            ))
        }
    }

    #[derive(Clone)]
    struct TranscriptOnlyProvider {
        tts_calls: Arc<Mutex<usize>>,
    }

    impl TranscriptOnlyProvider {
        fn new() -> Self {
            Self {
                tts_calls: Arc::new(Mutex::new(0)),
            }
        }
    }

    impl VoiceModelProvider for TranscriptOnlyProvider {
        fn stt_model(&self) -> Box<dyn STTModel> {
            Box::new(RecordingSttModel {
                settings: Arc::new(Mutex::new(Vec::new())),
            })
        }

        fn tts_model(&self) -> Box<dyn TTSModel> {
            Box::new(FailingTtsModel {
                calls: self.tts_calls.clone(),
            })
        }
    }

    #[derive(Clone)]
    struct StaticWorkflow;

    impl VoiceWorkflowBase for StaticWorkflow {
        fn run(&self, transcription: String) -> BoxStream<'static, Result<String>> {
            stream::iter(vec![Ok(transcription)]).boxed()
        }
    }

    #[tokio::test]
    async fn pipeline_forwards_configured_stt_and_tts_settings() {
        let provider = Arc::new(RecordingProvider::default());
        let pipeline = VoicePipeline::new(VoicePipelineConfig {
            stream_audio: true,
            split_sentences: false,
            stt_settings: STTModelSettings {
                model: Some("whisper-1".to_owned()),
                language: Some("en".to_owned()),
                prompt: Some("be accurate".to_owned()),
            },
            tts_settings: TTSModelSettings {
                model: Some("gpt-4o-mini-tts".to_owned()),
                voice: Some("fable".to_owned()),
                speed: Some(1.25),
            },
        })
        .with_model_provider(provider.clone());

        let result = pipeline
            .run(
                &StaticWorkflow,
                AudioInput {
                    mime_type: "audio/wav".to_owned(),
                    bytes: vec![1, 2, 3],
                },
            )
            .await
            .expect("pipeline should succeed");

        let completed = result
            .wait_for_completion()
            .await
            .expect("pipeline should complete");

        assert_eq!(completed.audio_chunks, 1);
        assert_eq!(
            provider.stt_settings.lock().await.as_slice(),
            &[STTModelSettings {
                model: Some("whisper-1".to_owned()),
                language: Some("en".to_owned()),
                prompt: Some("be accurate".to_owned()),
            }]
        );
        assert_eq!(
            provider.tts_settings.lock().await.as_slice(),
            &[TTSModelSettings {
                model: Some("gpt-4o-mini-tts".to_owned()),
                voice: Some("fable".to_owned()),
                speed: Some(1.25),
            }]
        );
    }

    #[tokio::test]
    async fn pipeline_skips_tts_when_audio_streaming_is_disabled() {
        let provider = Arc::new(TranscriptOnlyProvider::new());
        let pipeline = VoicePipeline::new(VoicePipelineConfig {
            stream_audio: false,
            split_sentences: false,
            ..VoicePipelineConfig::default()
        })
        .with_model_provider(provider.clone());

        let result = pipeline
            .run(
                &StaticWorkflow,
                AudioInput {
                    mime_type: "audio/wav".to_owned(),
                    bytes: vec![1, 2, 3],
                },
            )
            .await
            .expect("pipeline should start");

        let completed = result
            .wait_for_completion()
            .await
            .expect("pipeline should not depend on TTS success");

        assert_eq!(
            completed.transcript,
            vec!["normalized transcript".to_owned()]
        );
        assert_eq!(completed.audio_chunks, 0);
        assert_eq!(*provider.tts_calls.lock().await, 0);
    }
}

async fn synthesize_chunk(
    recorder: &VoiceStreamRecorder,
    tts_model: &dyn TTSModel,
    text: String,
    settings: &TTSModelSettings,
    stream_audio: bool,
) -> Result<()> {
    recorder.push_transcript(text.clone()).await;
    if !stream_audio {
        return Ok(());
    }
    let synthesized = tts_model.synthesize(&text, settings).await?;
    recorder.push_events(synthesized).await;
    Ok(())
}
