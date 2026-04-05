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
