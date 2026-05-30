# Voice Quickstart

Voice pipelines run speech-to-text, a workflow, and optional text-to-speech on top of the same agent runtime used by text agents.

## Minimal Buffered Audio Flow

The runnable version lives in [voice_pipeline.rs](../../crates/openai-agents/examples/voice_pipeline.rs).

```rust,no_run
use openai_agents::voice::{
    AudioInput, SingleAgentVoiceWorkflow, VoicePipeline, VoicePipelineConfig,
};
use openai_agents::{Agent, AgentsError};

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let workflow = SingleAgentVoiceWorkflow::new(
        Agent::builder("assistant")
            .instructions("Be concise.")
            .build(),
    );
    let pipeline = VoicePipeline::new(VoicePipelineConfig {
        stream_audio: false,
        ..VoicePipelineConfig::default()
    });

    let result = pipeline
        .run(
            &workflow,
            AudioInput {
                mime_type: "audio/wav".to_owned(),
                bytes: vec![1, 2, 3],
            },
        )
        .await?;
    let completed = result.wait_for_completion().await?;
    println!("{:?}", completed.transcript);
    Ok(())
}
```

## Streamed Audio Input

Use `StreamedAudioInput` when audio arrives in chunks. The runnable version is [voice_streamed.rs](../../crates/openai-agents/examples/voice_streamed.rs).

```rust,no_run
use openai_agents::voice::{
    StreamedAudioInput, VoicePipeline, VoicePipelineConfig, VoiceWorkflowBase,
};

# fn build_workflow() -> impl VoiceWorkflowBase { todo!() }
# async fn run() -> Result<(), openai_agents::AgentsError> {
let workflow = build_workflow();
let pipeline = VoicePipeline::new(VoicePipelineConfig {
    stream_audio: true,
    ..VoicePipelineConfig::default()
});
let input = StreamedAudioInput::from_pcm16_chunks(&[vec![0, 128], vec![-128, 0]]);
let result = pipeline.run_streamed_audio_input(&workflow, input).await?;
let completed = result.wait_for_completion().await?;
println!("{:?}", completed.transcript);
# Ok(())
# }
```

## Main Types

| Type | Role |
| --- | --- |
| `AudioInput` | Buffered audio input with MIME type and bytes. |
| `StreamedAudioInput` | Chunked PCM16 input. |
| `VoicePipeline` | Runs STT, workflow, and optional TTS. |
| `VoicePipelineConfig` | Controls streaming, sentence splitting, STT settings, and TTS settings. |
| `SingleAgentVoiceWorkflow` | Adapts a normal `Agent` to the voice workflow trait. |
| `StreamedAudioResult` | Live result with transcript, audio, lifecycle, completion, and error state. |

## Read Next

- [README.md](README.md)
- [workflow.md](workflow.md)
- [pipeline.md](pipeline.md)
- [tracing.md](tracing.md)
