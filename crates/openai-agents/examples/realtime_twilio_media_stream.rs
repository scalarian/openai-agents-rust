use openai_agents::extensions::{TwilioRealtimeTransportAction, TwilioRealtimeTransportLayer};
use openai_agents::realtime::{
    RealtimeAgent, RealtimeRunConfig, RealtimeRunner, RealtimeSessionModelSettings,
    RealtimeTurnDetectionConfig,
};
use openai_agents::{AgentsError, function_tool};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
struct WeatherArgs {
    city: String,
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let get_weather = function_tool(
        "get_weather",
        "Get the weather in a city.",
        |_ctx, args: WeatherArgs| async move {
            Ok::<_, AgentsError>(format!("The weather in {} is sunny.", args.city))
        },
    )?;

    let get_current_time = function_tool(
        "get_current_time",
        "Get the current time.",
        |_ctx, _args: serde_json::Value| async move {
            Ok::<_, AgentsError>("The current time is 12:00:00 UTC.".to_owned())
        },
    )?;

    let agent = RealtimeAgent::new("Twilio Assistant")
        .with_instructions(
            "Start every phone conversation with a concise greeting and keep answers friendly.",
        )
        .with_function_tool(get_weather)
        .with_function_tool(get_current_time);

    let mut twilio = TwilioRealtimeTransportLayer::new();
    let model_settings = twilio.normalize_session_config(Some(RealtimeSessionModelSettings {
        model_name: Some("gpt-realtime-2".to_owned()),
        turn_detection: Some(RealtimeTurnDetectionConfig {
            kind: Some("semantic_vad".to_owned()),
            interrupt_response: Some(true),
            create_response: Some(true),
            ..RealtimeTurnDetectionConfig::default()
        }),
        ..RealtimeSessionModelSettings::default()
    }));
    let runner = RealtimeRunner::new(agent).with_config(RealtimeRunConfig {
        model_settings: Some(model_settings),
        ..RealtimeRunConfig::default()
    });
    let session = runner.run().await?;

    let start_actions = twilio.handle_incoming_message(
        r#"{"event":"start","start":{"streamSid":"MZ123"}}"#,
        session.connected().await,
    )?;
    let media_actions = twilio.handle_incoming_message(
        r#"{"event":"media","media":{"payload":"AQIDBA=="}}"#,
        session.connected().await,
    )?;

    for action in &media_actions {
        if let TwilioRealtimeTransportAction::ForwardInputAudio { bytes } = action {
            session.send_audio(bytes).await?;
        }
    }

    let outbound_audio = twilio.audio_messages(Some("response-item"), &[0, 1, 2, 3, 4, 5, 6, 7]);
    let mark_actions = twilio.handle_incoming_message(
        r#"{"event":"mark","mark":{"name":"response-item:1"}}"#,
        session.connected().await,
    )?;
    let interrupt = twilio.interrupt_decision(true);

    println!("start_actions={start_actions:?}");
    println!("media_actions={media_actions:?}");
    let outbound_messages_json = serde_json::to_string(
        &outbound_audio
            .iter()
            .map(|message| message.to_json_value())
            .collect::<Vec<_>>(),
    )
    .map_err(|error| AgentsError::message(format!("serialize Twilio messages: {error}")))?;
    println!("outbound_messages={outbound_messages_json}");
    println!("mark_actions={mark_actions:?}");
    println!("interrupt_messages={}", interrupt.messages.len());
    session.close().await?;
    Ok(())
}
