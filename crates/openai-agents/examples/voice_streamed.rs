use futures::StreamExt;
use futures::stream::{self, BoxStream};
use openai_agents::voice::{
    StreamedAudioInput, VoicePipeline, VoicePipelineConfig, VoiceStreamEvent, VoiceWorkflowBase,
};
use openai_agents::{AgentsError, Result as AgentsResult};

#[derive(Clone)]
struct SecretWordWorkflow {
    secret_word: String,
}

impl VoiceWorkflowBase for SecretWordWorkflow {
    fn on_start(&self) -> BoxStream<'static, AgentsResult<String>> {
        stream::iter(vec![Ok("Voice pipeline ready.".to_owned())]).boxed()
    }

    fn run(&self, transcription: String) -> BoxStream<'static, AgentsResult<String>> {
        let chunks = if transcription
            .to_lowercase()
            .contains(&self.secret_word.to_lowercase())
        {
            vec!["You guessed the secret word!".to_owned()]
        } else {
            vec![
                "I heard streamed audio input. ".to_owned(),
                format!("Transcription was: {transcription}"),
            ]
        };

        stream::iter(chunks.into_iter().map(Ok)).boxed()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let workflow = SecretWordWorkflow {
        secret_word: "dog".to_owned(),
    };
    let pipeline = VoicePipeline::new(VoicePipelineConfig {
        stream_audio: true,
        ..VoicePipelineConfig::default()
    });
    let input =
        StreamedAudioInput::from_pcm16_chunks(&[vec![0, 128, -128, 256], vec![-256, 512, -512, 0]]);
    let result = pipeline.run_streamed_audio_input(&workflow, input).await?;

    let mut audio_events = 0usize;
    let mut lifecycle_events = Vec::new();
    let mut events = Box::pin(result.stream_events());
    while let Some(event) = events.next().await {
        match event {
            VoiceStreamEvent::Audio(audio) => {
                audio_events += 1;
                println!("audio_samples={}", audio.data.unwrap_or_default().len());
            }
            VoiceStreamEvent::Lifecycle(lifecycle) => {
                lifecycle_events.push(lifecycle.event.clone());
                println!("lifecycle={}", lifecycle.event);
            }
            VoiceStreamEvent::Transcript(transcript) => {
                println!("transcript_event={}", transcript.text);
            }
            VoiceStreamEvent::Error(error) => {
                println!("error={}", error.error);
            }
        }
    }

    let completed = result.wait_for_completion().await?;
    println!("audio_event_count={audio_events}");
    println!("lifecycle_events={}", lifecycle_events.join(","));
    println!("completed_transcript={:?}", completed.transcript);
    Ok(())
}
