use std::io::{Read, Write};
use std::net::TcpListener as StdTcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use agents_core::{InputItem, ModelProvider, ModelRequest, ModelSettings, OutputItem};
use agents_openai::OpenAIApi;
use agents_openai::{OpenAIProvider, OpenAIResponsesTransport};
use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        Message,
        handshake::server::{Request, Response},
    },
};

#[derive(Debug)]
struct HttpCapture {
    request_line: String,
    headers: Vec<(String, String)>,
    body: Value,
}

#[derive(Debug)]
struct WsCapture {
    path_and_query: String,
    headers: Vec<(String, String)>,
    payload: Value,
}

fn start_http_responses_stub(
    response_text: &'static str,
) -> (String, thread::JoinHandle<HttpCapture>) {
    let listener = StdTcpListener::bind("127.0.0.1:0").expect("http listener should bind");
    let address = listener.local_addr().expect("http listener address");
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("http request should connect");
        let mut buffer = Vec::new();
        let mut chunk = [0_u8; 4096];
        let header_end = loop {
            let read = stream.read(&mut chunk).expect("http request should read");
            assert!(read > 0, "http request closed before sending headers");
            buffer.extend_from_slice(&chunk[..read]);
            if let Some(index) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
                break index + 4;
            }
        };

        let header_text =
            String::from_utf8(buffer[..header_end].to_vec()).expect("headers should be utf-8");
        let mut header_lines = header_text.split("\r\n").filter(|line| !line.is_empty());
        let request_line = header_lines
            .next()
            .expect("request line should exist")
            .to_owned();
        let headers = header_lines
            .map(|line| {
                let (name, value) = line.split_once(':').expect("header should contain colon");
                (name.trim().to_owned(), value.trim().to_owned())
            })
            .collect::<Vec<_>>();
        let content_length = headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .and_then(|(_, value)| value.parse::<usize>().ok())
            .unwrap_or(0);

        let mut body_bytes = buffer[header_end..].to_vec();
        while body_bytes.len() < content_length {
            let read = stream.read(&mut chunk).expect("http body should read");
            assert!(read > 0, "http request closed before full body was read");
            body_bytes.extend_from_slice(&chunk[..read]);
        }
        body_bytes.truncate(content_length);
        let body: Value = serde_json::from_slice(&body_bytes).expect("http body should be json");

        let body_bytes = response_text.as_bytes();
        let response = format!(
            concat!(
                "HTTP/1.1 200 OK\r\n",
                "content-type: application/json\r\n",
                "content-length: {}\r\n",
                "x-request-id: req_http_123\r\n",
                "connection: close\r\n\r\n",
                "{}"
            ),
            body_bytes.len(),
            response_text
        );
        stream
            .write_all(response.as_bytes())
            .expect("http response should write");

        HttpCapture {
            request_line,
            headers,
            body,
        }
    });

    (format!("http://{address}/v1"), handle)
}

async fn start_ws_responses_stub(
    response_id: &'static str,
    output_text: &'static str,
) -> (String, tokio::task::JoinHandle<WsCapture>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("websocket listener should bind");
    let address = listener.local_addr().expect("websocket listener address");
    let capture = Arc::new(Mutex::new(None));
    let capture_for_server = capture.clone();
    let handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("websocket should accept");
        let mut ws_stream =
            accept_hdr_async(stream, move |request: &Request, response: Response| {
                let headers = request
                    .headers()
                    .iter()
                    .map(|(name, value)| {
                        (
                            name.as_str().to_owned(),
                            value.to_str().unwrap_or_default().to_owned(),
                        )
                    })
                    .collect::<Vec<_>>();
                *capture_for_server.lock().expect("capture lock") =
                    Some((request.uri().to_string(), headers));
                Ok(response)
            })
            .await
            .expect("websocket handshake should succeed");

        let request_frame = ws_stream
            .next()
            .await
            .expect("request frame should arrive")
            .expect("request frame should be readable");
        let request_text = request_frame
            .into_text()
            .expect("request frame should be text");
        let payload: Value = serde_json::from_str(&request_text).expect("payload should be json");

        ws_stream
            .send(Message::Text(
                json!({
                    "type": "response.completed",
                    "response": {
                        "id": response_id,
                        "output": [{
                            "type": "message",
                            "content": [{"type": "output_text", "text": output_text}]
                        }],
                        "usage": {"input_tokens": 2, "output_tokens": 3}
                    }
                })
                .to_string()
                .into(),
            ))
            .await
            .expect("completed websocket event should send");

        let (path_and_query, headers) = capture
            .lock()
            .expect("capture lock")
            .take()
            .expect("handshake capture should be present");
        WsCapture {
            path_and_query,
            headers,
            payload,
        }
    });

    (format!("ws://{address}"), handle)
}

#[tokio::test]
async fn openai_provider_applies_transport_choice_without_leaking_config() {
    let (http_base_url, http_server) = start_http_responses_stub(
        r#"{"id":"resp_http_123","output":[{"type":"message","content":[{"type":"output_text","text":"http-ok"}]}],"usage":{"input_tokens":1,"output_tokens":2}}"#,
    );
    let (ws_base_url_a, ws_server_a) = start_ws_responses_stub("resp_ws_a", "ws-a").await;
    let (ws_base_url_b, ws_server_b) = start_ws_responses_stub("resp_ws_b", "ws-b").await;

    let http_provider = OpenAIProvider::new()
        .with_api(OpenAIApi::Responses)
        .with_api_key("sk-http")
        .with_base_url(http_base_url.clone())
        .with_use_responses_websocket(false);
    let websocket_provider_a = OpenAIProvider::new()
        .with_api(OpenAIApi::Responses)
        .with_api_key("sk-ws-a")
        .with_base_url("http://127.0.0.1:1")
        .with_websocket_base_url(ws_base_url_a.clone())
        .with_use_responses_websocket(true);
    let websocket_provider_b = OpenAIProvider::new()
        .with_api(OpenAIApi::Responses)
        .with_api_key("sk-ws-b")
        .with_base_url("http://127.0.0.1:1")
        .with_websocket_base_url(ws_base_url_b.clone())
        .with_use_responses_websocket(true);

    assert_eq!(
        http_provider.responses_transport(),
        OpenAIResponsesTransport::Http
    );
    assert_eq!(
        websocket_provider_a.responses_transport(),
        OpenAIResponsesTransport::WebSocket
    );
    assert_eq!(
        websocket_provider_b.responses_transport(),
        OpenAIResponsesTransport::WebSocket
    );

    let http_response = http_provider
        .resolve(Some("gpt-5"))
        .generate(ModelRequest {
            model: Some("gpt-5".to_owned()),
            input: vec![InputItem::from("hello from http")],
            ..ModelRequest::default()
        })
        .await
        .expect("http provider request should succeed");
    let ws_response_a = websocket_provider_a
        .resolve(Some("gpt-5"))
        .generate(ModelRequest {
            model: Some("gpt-5".to_owned()),
            input: vec![InputItem::from("hello from ws a")],
            ..ModelRequest::default()
        })
        .await
        .expect("websocket provider a request should succeed");
    let ws_response_b = websocket_provider_b
        .resolve(Some("gpt-5-mini"))
        .generate(ModelRequest {
            model: Some("gpt-5-mini".to_owned()),
            input: vec![InputItem::from("hello from ws b")],
            ..ModelRequest::default()
        })
        .await
        .expect("websocket provider b request should succeed");

    let http_capture = http_server.join().expect("http server should finish");
    let ws_capture_a = ws_server_a.await.expect("ws server a should finish");
    let ws_capture_b = ws_server_b.await.expect("ws server b should finish");

    assert_eq!(http_capture.request_line, "POST /v1/responses HTTP/1.1");
    assert!(http_capture.headers.iter().any(|(name, value)| {
        name.eq_ignore_ascii_case("authorization") && value == "Bearer sk-http"
    }));
    assert_eq!(http_capture.body["model"], "gpt-5");
    assert!(
        matches!(http_response.output.first(), Some(OutputItem::Text { text }) if text == "http-ok")
    );
    assert_eq!(http_response.request_id.as_deref(), Some("req_http_123"));

    assert_eq!(ws_capture_a.path_and_query, "/responses");
    assert!(ws_capture_a.headers.iter().any(|(name, value)| {
        name.eq_ignore_ascii_case("authorization") && value == "Bearer sk-ws-a"
    }));
    assert_eq!(ws_capture_a.payload["model"], "gpt-5");
    assert!(
        matches!(ws_response_a.output.first(), Some(OutputItem::Text { text }) if text == "ws-a")
    );

    assert_eq!(ws_capture_b.path_and_query, "/responses");
    assert!(ws_capture_b.headers.iter().any(|(name, value)| {
        name.eq_ignore_ascii_case("authorization") && value == "Bearer sk-ws-b"
    }));
    assert_eq!(ws_capture_b.payload["model"], "gpt-5-mini");
    assert!(
        matches!(ws_response_b.output.first(), Some(OutputItem::Text { text }) if text == "ws-b")
    );
}

#[tokio::test]
async fn responses_requests_forward_metadata_and_conversation_settings() {
    let (ws_base_url, ws_server) = start_ws_responses_stub("resp_ws_meta", "done").await;
    let provider = OpenAIProvider::new()
        .with_api(OpenAIApi::Responses)
        .with_api_key("sk-meta")
        .with_base_url("http://127.0.0.1:1")
        .with_websocket_base_url(ws_base_url)
        .with_use_responses_websocket(true);

    let response = provider
        .resolve(Some("gpt-5"))
        .generate(ModelRequest {
            model: Some("gpt-5".to_owned()),
            previous_response_id: Some("resp_prev".to_owned()),
            conversation_id: Some("conv_123".to_owned()),
            settings: ModelSettings {
                metadata: std::collections::BTreeMap::from([("transport".to_owned(), json!("ws"))]),
                extra_headers: std::collections::BTreeMap::from([(
                    "X-Test-Header".to_owned(),
                    json!("present"),
                )]),
                extra_query: std::collections::BTreeMap::from([(
                    "tenant".to_owned(),
                    json!("acme"),
                )]),
                ..Default::default()
            },
            input: vec![InputItem::from("hello")],
            ..ModelRequest::default()
        })
        .await
        .expect("websocket metadata request should succeed");

    let capture = ws_server.await.expect("ws server should finish");
    assert_eq!(capture.path_and_query, "/responses?tenant=acme");
    assert!(capture.headers.iter().any(
        |(name, value)| name.eq_ignore_ascii_case("authorization") && value == "Bearer sk-meta"
    ));
    assert!(
        capture
            .headers
            .iter()
            .any(|(name, value)| name.eq_ignore_ascii_case("x-test-header") && value == "present")
    );
    assert_eq!(capture.payload["type"], "response.create");
    assert_eq!(capture.payload["stream"], true);
    assert_eq!(capture.payload["conversation"], "conv_123");
    assert_eq!(capture.payload["metadata"]["transport"], "ws");
    assert!(capture.payload.get("previous_response_id").is_none());
    assert_eq!(response.response_id.as_deref(), Some("resp_ws_meta"));
    assert!(matches!(response.output.first(), Some(OutputItem::Text { text }) if text == "done"));
}
