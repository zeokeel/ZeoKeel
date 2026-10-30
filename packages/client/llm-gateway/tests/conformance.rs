#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only

use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};
use zk_contract_llm::LlmProvider;
use zk_llm_gateway::GatewayLlm;

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

#[test]
fn completes_and_streams_through_gateway_contract() {
    let app = Router::new().route(
        "/v1/responses",
        post(|Json(body): Json<Value>| async move {
            assert_eq!(body["model"], "deepseek-chat");
            if body["stream"] == true {
                Json(json!({ "output": [{ "content": [{ "type": "output_text", "text": "unused" }] }] }))
            } else {
                Json(json!({ "output": [{ "content": [{ "type": "output_text", "text": "hello from gateway" }] }] }))
            }
        }),
    );
    let addr = serve(app);
    let provider = GatewayLlm::new(format!("http://{addr}"), "access-token".to_owned(), "deepseek-chat".to_owned());
    assert_eq!(provider.complete("hi").unwrap(), "hello from gateway");
    streams_sse_text_deltas();
}

fn streams_sse_text_deltas() {
    let app = Router::new().route(
        "/v1/responses",
        post(|_body: Json<Value>| async {
            (
                [("content-type", "text/event-stream")],
                "data: {\"delta\":\"gateway \"}\n\ndata: {\"delta\":\"hello\"}\n\ndata: [DONE]\n\n",
            )
        }),
    );
    let addr = serve(app);
    let provider = GatewayLlm::new(format!("http://{addr}"), "access-token".to_owned(), "deepseek-chat".to_owned());
    let mut deltas = Vec::new();
    let text = provider.complete_stream("hi", &mut |delta| deltas.push(delta.to_owned())).unwrap();
    assert_eq!(text, "gateway hello");
    assert_eq!(deltas, vec!["gateway ", "hello"]);
}
