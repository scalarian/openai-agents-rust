use std::sync::Arc;

use futures::StreamExt;
use openai_agents::voice::{
    AudioInput, SingleAgentVoiceWorkflow, SingleAgentWorkflowCallbacks, VoicePipeline,
    VoicePipelineConfig, VoiceStreamEvent,
};
use openai_agents::{Agent, AgentsError, function_tool};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
struct WeatherArgs {
    city: String,
}

#[derive(Clone, Default)]
struct WorkflowCallbacks;

impl SingleAgentWorkflowCallbacks for WorkflowCallbacks {
    fn on_run(&self, transcription: &str) {
        println!("callback_transcription={transcription}");
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let get_weather = function_tool(
        "get_weather",
        "Get the weather for a given city.",
        |_ctx, args: WeatherArgs| async move {
            Ok::<_, AgentsError>(format!("The weather in {} is sunny.", args.city))
        },
    )?;

    let spanish_agent = Agent::builder("Spanish")
        .handoff_description("A Spanish speaking agent.")
        .instructions("Speak in Spanish and keep responses concise.")
        .build();

    let assistant = Agent::builder("Assistant")
        .instructions("Be polite and concise. If the user speaks in Spanish, hand off to Spanish.")
        .function_tool(get_weather)
        .handoff_to_agent(spanish_agent)
        .build();

    let workflow =
        SingleAgentVoiceWorkflow::new(assistant).with_callbacks(Arc::new(WorkflowCallbacks));
    let pipeline = VoicePipeline::new(VoicePipelineConfig {
        stream_audio: true,
        ..VoicePipelineConfig::default()
    });
    let audio_input = AudioInput::from_pcm16(&[0, 256, -256, 512, -512, 0]);
    let result = pipeline.run(&workflow, audio_input).await?;
    let mut audio_events = 0usize;

    let mut events = Box::pin(result.stream_events());
    while let Some(event) = events.next().await {
        match event {
            VoiceStreamEvent::Audio(audio) => {
                audio_events += 1;
                println!(
                    "received_audio_samples={}",
                    audio.data.unwrap_or_default().len()
                );
            }
            VoiceStreamEvent::Lifecycle(lifecycle) => {
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
    println!("completed_audio_events={audio_events}");
    println!("completed_transcript={:?}", completed.transcript);
    Ok(())
}
