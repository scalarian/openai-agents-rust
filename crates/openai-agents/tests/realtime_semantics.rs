use futures::StreamExt;
use openai_agents::realtime::{
    OpenAIRealtimeSIPModel, OpenAIRealtimeWebSocketModel, RealtimeAgent, RealtimeAudioConfig,
    RealtimeAudioFormat, RealtimeAudioInputConfig, RealtimeAudioOutputConfig, RealtimeEvent,
    RealtimeModel, RealtimeModelConfig, RealtimeRunConfig, RealtimeRunner,
    RealtimeSessionModelSettings, TransportConfig,
};
use std::collections::BTreeMap;

#[tokio::test]
async fn realtime_session_streams_events_for_live_commands() {
    let runner = RealtimeRunner::new(RealtimeAgent::new("assistant"));
    let session = runner.run().await.expect("session should start");
    let collector = {
        let session = session.clone();
        tokio::spawn(async move { session.stream_events().collect::<Vec<_>>().await })
    };

    session
        .send_text("hello")
        .await
        .expect("text turn should succeed");
    session
        .send_audio(&[1, 2, 3, 4])
        .await
        .expect("audio turn should succeed");
    session
        .interrupt(Some("user_stop".to_owned()))
        .await
        .expect("interrupt should succeed");

    let mut specialist = RealtimeAgent::new("specialist");
    specialist.model_settings = Some(RealtimeSessionModelSettings {
        model_name: Some("gpt-realtime-specialist".to_owned()),
        ..RealtimeSessionModelSettings::default()
    });
    session
        .update_agent(specialist)
        .await
        .expect("agent update should succeed");
    session.close().await.expect("close should succeed");

    let events = collector.await.expect("collector should finish");

    assert_eq!(session.transcript().await, "hello");
    assert!(matches!(events.first(), Some(RealtimeEvent::AgentStart(_))));
    assert!(events.iter().any(
        |event| matches!(event, RealtimeEvent::TranscriptDelta(delta) if delta.text == "hello")
    ));
    assert!(events.iter().any(
        |event| matches!(event, RealtimeEvent::RawModelEvent(raw) if raw.event_type == "audio_done")
    ));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, RealtimeEvent::Interrupted(_)))
    );
    assert!(events.iter().any(
        |event| matches!(event, RealtimeEvent::AgentEnd(ended) if ended.info.agent_name.as_deref() == Some("assistant"))
    ));
    assert!(events.iter().any(
        |event| matches!(event, RealtimeEvent::AgentStart(started) if started.info.agent_name.as_deref() == Some("specialist"))
    ));
    assert!(events
        .iter()
        .any(|event| matches!(event, RealtimeEvent::SessionUpdated(updated) if updated.model.as_deref() == Some("gpt-realtime-specialist"))));
    assert_eq!(
        session
            .model_settings()
            .await
            .and_then(|settings| settings.model_name),
        Some("gpt-realtime-specialist".to_owned())
    );
    assert!(!session.playback_state().await.playing);
    assert!(matches!(
        events.last(),
        Some(RealtimeEvent::SessionClosed(_))
    ));
}

#[tokio::test]
async fn realtime_runner_applies_run_config_model_settings() {
    let runner =
        RealtimeRunner::new(RealtimeAgent::new("assistant")).with_config(RealtimeRunConfig {
            model_settings: Some(RealtimeSessionModelSettings {
                model_name: Some("gpt-realtime-configured".to_owned()),
                ..RealtimeSessionModelSettings::default()
            }),
            ..RealtimeRunConfig::default()
        });
    let session = runner.run().await.expect("session should start");

    assert_eq!(
        session
            .model_settings()
            .await
            .and_then(|settings| settings.model_name),
        Some("gpt-realtime-configured".to_owned())
    );
}

#[tokio::test]
async fn realtime_websocket_runtime_state_exposes_connected_transport_and_applied_output_settings()
{
    let mut query_params = BTreeMap::new();
    query_params.insert("model".to_owned(), "ignored-by-custom-query".to_owned());
    query_params.insert("foo".to_owned(), "bar".to_owned());

    let mut model = OpenAIRealtimeWebSocketModel {
        config: RealtimeModelConfig {
            model: Some("gpt-realtime-1.5".to_owned()),
        },
        transport: TransportConfig {
            api_key: Some("sk-test".to_owned()),
            websocket_url: Some("https://example.com/realtime".to_owned()),
            call_id: None,
            query_params,
        },
        ..OpenAIRealtimeWebSocketModel::default()
    };

    model.connect().await.expect("connect should succeed");
    model
        .update_session(&RealtimeSessionModelSettings {
            model_name: Some("gpt-realtime-2".to_owned()),
            audio: Some(RealtimeAudioConfig {
                input: Some(RealtimeAudioInputConfig {
                    format: Some(RealtimeAudioFormat::G711Ulaw),
                    ..RealtimeAudioInputConfig::default()
                }),
                output: Some(RealtimeAudioOutputConfig {
                    voice: Some("marin".to_owned()),
                    speed: Some(1.25),
                    ..RealtimeAudioOutputConfig::default()
                }),
            }),
            ..RealtimeSessionModelSettings::default()
        })
        .await
        .expect("session update should succeed");

    let runtime_state = model.runtime_state();
    assert_eq!(
        runtime_state.transport.connection_url.as_deref(),
        Some("wss://example.com/realtime?foo=bar&model=ignored-by-custom-query")
    );
    assert!(runtime_state.transport.api_key_present);
    assert_eq!(
        runtime_state
            .last_session_payload
            .as_ref()
            .and_then(|payload| payload.payload.get("audio"))
            .and_then(|audio| audio.get("output"))
            .and_then(|output| output.get("voice"))
            .and_then(serde_json::Value::as_str),
        Some("marin")
    );
    assert_eq!(
        runtime_state
            .last_session_payload
            .as_ref()
            .and_then(|payload| payload.payload.get("audio"))
            .and_then(|audio| audio.get("output"))
            .and_then(|output| output.get("speed"))
            .and_then(serde_json::Value::as_f64),
        Some(1.25)
    );
}

#[tokio::test]
async fn realtime_sip_runtime_state_exposes_call_attachment_and_applied_output_settings() {
    let mut query_params = BTreeMap::new();
    query_params.insert("foo".to_owned(), "bar".to_owned());

    let mut model = OpenAIRealtimeSIPModel {
        config: RealtimeModelConfig {
            model: Some("gpt-realtime-1.5".to_owned()),
        },
        transport: TransportConfig {
            api_key: Some("sk-test".to_owned()),
            websocket_url: Some("https://example.com/realtime".to_owned()),
            call_id: Some("call_123".to_owned()),
            query_params,
        },
        ..OpenAIRealtimeSIPModel::default()
    };

    model.connect().await.expect("connect should succeed");
    model
        .update_session(&RealtimeSessionModelSettings {
            audio: Some(RealtimeAudioConfig {
                output: Some(RealtimeAudioOutputConfig {
                    voice: Some("verse".to_owned()),
                    speed: Some(1.5),
                    ..RealtimeAudioOutputConfig::default()
                }),
                ..RealtimeAudioConfig::default()
            }),
            ..RealtimeSessionModelSettings::default()
        })
        .await
        .expect("session update should succeed");

    let runtime_state = model.runtime_state();
    assert_eq!(
        runtime_state.transport.connection_url.as_deref(),
        Some("wss://example.com/realtime?call_id=call_123&foo=bar")
    );
    assert_eq!(runtime_state.transport.call_id.as_deref(), Some("call_123"));
    assert_eq!(
        runtime_state
            .last_session_payload
            .as_ref()
            .and_then(|payload| payload.payload.get("audio"))
            .and_then(|audio| audio.get("output"))
            .and_then(|output| output.get("voice"))
            .and_then(serde_json::Value::as_str),
        Some("verse")
    );
    assert_eq!(
        runtime_state
            .last_session_payload
            .as_ref()
            .and_then(|payload| payload.payload.get("audio"))
            .and_then(|audio| audio.get("output"))
            .and_then(|output| output.get("speed"))
            .and_then(serde_json::Value::as_f64),
        Some(1.5)
    );
}
