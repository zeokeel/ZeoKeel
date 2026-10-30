#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use axum::{routing::post, Json, Router};
use serde_json::{json, Value};
use zk_contract_llm::LlmProvider;
use zk_llm_gateway::GatewayLlm;

#[test]
fn calls_gateway_once_and_parses_text() {
    let app = Router::new().route(
        "/v1/responses",
        post(|Json(body): Json<Value>| async move {
            assert_eq!(body["model"], "deepseek-chat");
            Json(json!({ "output": [{ "content": [{ "type": "output_text", "text": "hello from gateway" }] }] }))
        }),
    );
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
    let provider = GatewayLlm::new(format!("http://{addr}"), "access-token".to_owned(), "deepseek-chat".to_owned());
    let text = provider.complete("hi").unwrap();
    assert_eq!(text, "hello from gateway");
}
