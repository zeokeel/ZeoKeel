#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)] // test-only

//! End-to-end tests for the rig-native Responses model over a local fake
//! gateway. No network: an axum server on `127.0.0.1:0` plays the enterprise
//! gateway, and the adapter must post `{server}/v1/responses` with the login
//! bearer token, then decode streaming text, tool calls and usage.

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::extract::Json;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::post;
use axum::Router;
use futures::StreamExt;
use rig_core::completion::{CompletionRequest, FinishReason, ToolDefinition};
use rig_core::message::{AssistantContent, ToolName};
use rig_core::streaming::{Item, StreamEvent};
use serde_json::{json, Value};
use zk_llm_gateway::{model, model_from_config, set_credentials, set_model};

static CONFIG_LOCK: Mutex<()> = Mutex::new(());

/// Serialize tests that mutate the process-global gateway config, and keep
/// running after a sibling test panicked (a poisoned lock would otherwise hide
/// the real failure behind `PoisonError`).
fn config_guard() -> std::sync::MutexGuard<'static, ()> {
    CONFIG_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// `(addr, seen request body, seen authorization header)`.
type CapturedGateway = (
    std::net::SocketAddr,
    Arc<Mutex<Option<Value>>>,
    Arc<Mutex<Option<String>>>,
);

fn serve(app: Router) -> std::net::SocketAddr {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(listener).unwrap();
            axum::serve(listener, app).await.unwrap();
        });
    });
    addr
}

fn response_with(content_type: &str, body: String) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(body))
        .unwrap()
}

fn sse(events: &[Value]) -> Response {
    let body = events
        .iter()
        .map(|event| format!("data: {}\n\n", serde_json::to_string(event).unwrap()))
        .collect::<String>();
    response_with("text/event-stream", body)
}

fn response_object(status: &str, output: Value, usage: Option<Value>) -> Value {
    let mut object = json!({
        "id": "resp_test",
        "object": "response",
        "created_at": 0,
        "status": status,
        "model": "gateway-model",
        "output": output,
        "tools": [],
    });
    if let Some(usage) = usage {
        object["usage"] = usage;
    }
    object
}

fn message_item(text: &str) -> Value {
    json!({
        "type": "message",
        "id": "msg_1",
        "role": "assistant",
        "status": "completed",
        "content": [{ "type": "output_text", "annotations": [], "logprobs": [], "text": text }],
    })
}

fn function_call_item(arguments: &str) -> Value {
    json!({
        "type": "function_call",
        "id": "fc_1",
        "call_id": "call_1",
        "name": "lookup",
        "arguments": arguments,
        "status": "completed",
    })
}

fn text_stream() -> Vec<Value> {
    vec![
        json!({
            "type": "response.created",
            "sequence_number": 0,
            "response": response_object("in_progress", json!([]), None),
        }),
        json!({
            "type": "response.output_item.added",
            "output_index": 0,
            "sequence_number": 1,
            "item": {
                "type": "message",
                "id": "msg_1",
                "role": "assistant",
                "status": "in_progress",
                "content": [],
            },
        }),
        json!({
            "type": "response.output_text.delta",
            "item_id": "msg_1",
            "output_index": 0,
            "content_index": 0,
            "delta": "gateway ",
            "sequence_number": 2,
        }),
        json!({
            "type": "response.output_text.delta",
            "item_id": "msg_1",
            "output_index": 0,
            "content_index": 0,
            "delta": "hello",
            "sequence_number": 3,
        }),
        json!({
            "type": "response.output_item.done",
            "output_index": 0,
            "sequence_number": 4,
            "item": message_item("gateway hello"),
        }),
        json!({
            "type": "response.completed",
            "sequence_number": 5,
            "response": response_object(
                "completed",
                json!([message_item("gateway hello")]),
                Some(json!({ "input_tokens": 11, "output_tokens": 5, "total_tokens": 16 })),
            ),
        }),
    ]
}

fn tool_call_stream() -> Vec<Value> {
    vec![
        json!({
            "type": "response.created",
            "sequence_number": 0,
            "response": response_object("in_progress", json!([]), None),
        }),
        json!({
            "type": "response.output_item.added",
            "output_index": 0,
            "sequence_number": 1,
            "item": {
                "type": "function_call",
                "id": "fc_1",
                "call_id": "call_1",
                "name": "lookup",
                "arguments": "",
            },
        }),
        json!({
            "type": "response.function_call_arguments.delta",
            "item_id": "fc_1",
            "output_index": 0,
            "delta": "{\"q\":",
            "sequence_number": 2,
        }),
        json!({
            "type": "response.function_call_arguments.delta",
            "item_id": "fc_1",
            "output_index": 0,
            "delta": "\"rig\"}",
            "sequence_number": 3,
        }),
        json!({
            "type": "response.output_item.done",
            "output_index": 0,
            "sequence_number": 4,
            "item": function_call_item("{\"q\":\"rig\"}"),
        }),
        json!({
            "type": "response.completed",
            "sequence_number": 5,
            "response": response_object(
                "completed",
                json!([function_call_item("{\"q\":\"rig\"}")]),
                Some(json!({ "input_tokens": 20, "output_tokens": 3, "total_tokens": 23 })),
            ),
        }),
    ]
}

/// Serve one fake gateway that records the last request body and the
/// authorization header, and answers with `stream_events` for streaming
/// requests or `unary_body` for unary ones.
fn fake_gateway(stream_events: Vec<Value>, unary_body: Value) -> CapturedGateway {
    let seen_body = Arc::new(Mutex::new(None::<Value>));
    let seen_auth = Arc::new(Mutex::new(None::<String>));
    let body_slot = Arc::clone(&seen_body);
    let auth_slot = Arc::clone(&seen_auth);
    let app = Router::new().route(
        "/v1/responses",
        post(move |headers: HeaderMap, Json(body): Json<Value>| {
            let body_slot = Arc::clone(&body_slot);
            let auth_slot = Arc::clone(&auth_slot);
            async move {
                *body_slot.lock().unwrap() = Some(body.clone());
                *auth_slot.lock().unwrap() = headers
                    .get(header::AUTHORIZATION)
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_owned);
                if body.get("stream").and_then(Value::as_bool) == Some(true) {
                    sse(&stream_events)
                } else {
                    response_with(
                        "application/json",
                        serde_json::to_string(&unary_body).unwrap(),
                    )
                }
            }
        }),
    );
    (serve(app), seen_body, seen_auth)
}

fn gateway_credentials(addr: std::net::SocketAddr) {
    set_credentials(&format!("http://{addr}"), "access-token");
    set_model("gateway-model");
}

#[test]
fn boots_without_credentials_using_placeholder_and_fails_at_call() {
    let _guard = config_guard();
    set_credentials("", "");
    let placeholder = model().unwrap();
    assert_eq!(
        placeholder.id(),
        Some("unconfigured"),
        "pre-login build must get the ADR-064 boot placeholder"
    );
    assert_eq!(placeholder.name(), "openai");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let call = runtime.block_on(async {
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            placeholder.call(CompletionRequest::new("hello")),
        )
        .await
    });
    let call = match call {
        Ok(call) => call,
        Err(_) => panic!("placeholder call must fail fast at the transport layer, not hang"),
    };
    match call {
        Ok(_) => panic!("placeholder must never silently answer like a mock"),
        Err(error) => assert!(
            !error.to_string().trim().is_empty(),
            "placeholder failure must surface a transport error"
        ),
    }
}

#[test]
fn model_from_config_requires_login_and_rejects_local_providers() {
    let _guard = config_guard();
    set_credentials("", "");
    let error = model_from_config("gateway", "", "", "").unwrap_err();
    assert!(
        error.to_string().contains("login required"),
        "unauthenticated model_from_config() must fail clearly: {error}"
    );
    let error = model_from_config("mock", "http://127.0.0.1:1", "token", "model").unwrap_err();
    assert!(
        error.to_string().contains("does not support provider"),
        "local providers must be rejected in enterprise: {error}"
    );
    let error = model_from_config("gateway", "http://127.0.0.1:1", "", "model").unwrap_err();
    assert!(
        error.to_string().contains("login required"),
        "missing token must be reported: {error}"
    );
}

#[test]
fn streams_text_and_usage_through_gateway() {
    let _guard = config_guard();
    let (addr, seen_body, seen_auth) = fake_gateway(
        text_stream(),
        response_object(
            "completed",
            json!([message_item("gateway hello")]),
            Some(json!({ "input_tokens": 11, "output_tokens": 5, "total_tokens": 16 })),
        ),
    );
    gateway_credentials(addr);
    let gateway_model = model().unwrap();
    assert_eq!(
        gateway_model.id(),
        Some("gateway-model"),
        "after set_credentials+set_model, model() must build the real gateway model"
    );
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (streamed_text, response) = runtime.block_on(async move {
        let mut stream = gateway_model
            .stream(CompletionRequest::new("hello"))
            .unwrap();
        let mut streamed_text = String::new();
        while let Some(item) = stream.next().await {
            if let Item::Event(StreamEvent::Text { text, .. }) = item.unwrap() {
                streamed_text.push_str(&text);
            }
        }
        let response = stream.finish().await.unwrap();
        (streamed_text, response)
    });
    assert_eq!(streamed_text, "gateway hello");
    assert_eq!(response.text(), "gateway hello");
    assert_eq!(response.usage.input_tokens, Some(11));
    assert_eq!(response.usage.output_tokens, Some(5));
    assert_eq!(response.usage.total_tokens, Some(16));

    let body = seen_body
        .lock()
        .unwrap()
        .clone()
        .expect("request body captured");
    assert_eq!(body["model"], "gateway-model");
    assert_eq!(body["stream"], true);
    assert!(
        body.get("input").is_some(),
        "request must carry input: {body}"
    );
    assert_eq!(
        seen_auth.lock().unwrap().as_deref(),
        Some("Bearer access-token"),
        "the rig Responses wire must send the login token as bearer auth"
    );
}

#[test]
fn round_trips_tool_calls_and_usage_through_gateway() {
    let _guard = config_guard();
    let (addr, seen_body, _seen_auth) = fake_gateway(
        tool_call_stream(),
        response_object(
            "completed",
            json!([function_call_item("{\"q\":\"rig\"}")]),
            Some(json!({ "input_tokens": 20, "output_tokens": 3, "total_tokens": 23 })),
        ),
    );
    gateway_credentials(addr);
    let gateway_model = model().unwrap();
    let request = CompletionRequest::new("look up rig").tool(ToolDefinition::new(
        ToolName::new("lookup").unwrap(),
        "Look up a term in the docs",
        json!({
            "type": "object",
            "properties": { "q": { "type": "string" } },
            "required": ["q"],
            "additionalProperties": false,
        }),
    ));
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let response = runtime
        .block_on(async move { gateway_model.stream(request).unwrap().finish().await })
        .unwrap();

    assert_eq!(response.usage.input_tokens, Some(20));
    assert_eq!(response.usage.output_tokens, Some(3));
    assert_eq!(response.finish_reason(), Some(FinishReason::ToolCalls));
    match response.choice.as_slice() {
        [AssistantContent::ToolCall(call)] => {
            assert_eq!(call.function.name.to_string(), "lookup");
            assert_eq!(call.function.arguments.get("q"), Some(&json!("rig")));
        }
        other => panic!("expected one tool call, got {other:?}"),
    }

    let body = seen_body
        .lock()
        .unwrap()
        .clone()
        .expect("request body captured");
    assert_eq!(body["stream"], true);
    assert_eq!(body["tools"][0]["name"], "lookup");
    assert_eq!(body["tools"][0]["type"], "function");
    assert_eq!(
        body["tools"][0]["parameters"]["required"][0], "q",
        "tool JSON Schema must reach the gateway: {body}"
    );
}

#[test]
fn after_login_model_from_config_builds_real_model() {
    let _guard = config_guard();
    let (addr, seen_body, _seen_auth) = fake_gateway(
        text_stream(),
        response_object(
            "completed",
            json!([message_item("unary hello")]),
            Some(json!({ "input_tokens": 4, "output_tokens": 2, "total_tokens": 6 })),
        ),
    );
    gateway_credentials(addr);
    let gateway_model = model_from_config("gateway", "", "", "gateway-model").unwrap();
    assert_eq!(
        gateway_model.id(),
        Some("gateway-model"),
        "post-login factory must build the real gateway model, not the placeholder"
    );
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let response = runtime
        .block_on(async move { gateway_model.call(CompletionRequest::new("hello")).await })
        .unwrap();
    assert_eq!(response.text(), "unary hello");
    assert_eq!(response.usage.input_tokens, Some(4));
    let body = seen_body
        .lock()
        .unwrap()
        .clone()
        .expect("request body captured");
    assert_eq!(body["model"], "gateway-model");
}
