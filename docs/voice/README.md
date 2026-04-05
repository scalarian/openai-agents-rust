# Voice

Use this section when you want STT -> workflow -> TTS orchestration on top of the shared agents runtime.

## Main Types

- `SingleAgentVoiceWorkflow`
- `VoicePipeline`
- `VoicePipelineConfig`
- `StreamedAudioResult`
- `AudioInput`
- `StreamedAudioInput`

## Minimal Example

```rust
use openai_agents::Agent;
use openai_agents::voice::{AudioInput, SingleAgentVoiceWorkflow, VoicePipeline, VoicePipelineConfig};

#[tokio::main]
async fn main() -> Result<(), openai_agents::AgentsError> {
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

Runnable version: [voice_pipeline.rs](../../crates/openai-agents/examples/voice_pipeline.rs)

## In This Section

- [workflow.md](workflow.md)
- [pipeline.md](pipeline.md)

## Read Next

- [workflow.md](workflow.md)
- [pipeline.md](pipeline.md)
- [../realtime/README.md](../realtime/README.md)
