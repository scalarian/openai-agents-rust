use agents_core::{InputItem, ModelRequest, RunState};
use agents_openai::{OpenAIClientOptions, ResponsesWebSocketSession};
use serde_json::json;

#[test]
fn responses_websocket_helpers_normalize_urls_and_payloads() {
    let session = ResponsesWebSocketSession::new(
        Some("gpt-5".to_owned()),
        OpenAIClientOptions {
            api_key: Some("sk-test".to_owned()),
            base_url: "https://api.openai.com/v1".to_owned(),
            websocket_base_url: "https://api.openai.com/v1".to_owned(),
            organization: None,
            project: None,
        },
    )
    .with_response_id("resp_123");

    assert_eq!(
        session.websocket_url().expect("url should build"),
        "wss://api.openai.com/v1/responses"
    );

    let payload = session
        .request_frame(&ModelRequest {
            trace_id: None,
            model: Some("gpt-5".to_owned()),
            instructions: None,
            previous_response_id: Some("resp_request".to_owned()),
            conversation_id: Some("conv_123".to_owned()),
            settings: Default::default(),
            input: vec![InputItem::from("hello")],
            tools: Vec::new(),
            output_schema: None,
        })
        .expect("request frame should build");

    assert_eq!(payload["conversation"], "conv_123");
    assert_eq!(payload["type"], "response.create");
    assert_eq!(payload["stream"], true);
    assert!(payload.get("previous_response_id").is_none());
}

#[test]
fn hosted_tool_replay_drops_orphans_and_keeps_pending_items() {
    let state = RunState {
        normalized_input: Some(vec![
            InputItem::Json {
                value: json!({
                    "type": "shell_call",
                    "call_id": "shell-orphan",
                    "status": "completed",
                    "action": {"command": "echo orphan"},
                }),
            },
            InputItem::Json {
                value: json!({
                    "type": "tool_search_call",
                    "call_id": "search-keep",
                    "status": "completed",
                    "arguments": {"query": "rust"},
                }),
            },
            InputItem::Json {
                value: json!({
                    "type": "tool_search_output",
                    "call_id": "search-keep",
                    "status": "completed",
                    "tools": [],
                }),
            },
            InputItem::Json {
                value: json!({
                    "type": "tool_search_output",
                    "call_id": "search-orphan-output",
                    "status": "completed",
                    "tools": [],
                }),
            },
            InputItem::Json {
                value: json!({
                    "type": "function_call",
                    "call_id": "pending-user-call",
                    "name": "lookup",
                    "arguments": "{}",
                }),
            },
        ]),
        ..RunState::default()
    };

    let replay = state.resume_input();

    assert_eq!(replay.len(), 3);
    assert!(replay.iter().any(|item| matches!(
        item,
        InputItem::Json { value }
            if value.get("type").and_then(serde_json::Value::as_str) == Some("tool_search_call")
                && value.get("call_id").and_then(serde_json::Value::as_str) == Some("search-keep")
    )));
    assert!(replay.iter().any(|item| matches!(
        item,
        InputItem::Json { value }
            if value.get("type").and_then(serde_json::Value::as_str) == Some("tool_search_output")
                && value.get("call_id").and_then(serde_json::Value::as_str) == Some("search-keep")
    )));
    assert!(replay.iter().any(|item| matches!(
        item,
        InputItem::Json { value }
            if value.get("type").and_then(serde_json::Value::as_str) == Some("function_call")
                && value.get("call_id").and_then(serde_json::Value::as_str) == Some("pending-user-call")
    )));
    assert!(!replay.iter().any(|item| matches!(
        item,
        InputItem::Json { value }
            if value.get("call_id").and_then(serde_json::Value::as_str) == Some("shell-orphan")
    )));
    assert!(!replay.iter().any(|item| matches!(
        item,
        InputItem::Json { value }
            if value.get("call_id").and_then(serde_json::Value::as_str) == Some("search-orphan-output")
    )));
}
