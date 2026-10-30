#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use std::sync::Arc;
use std::time::Duration;
use zk_contract_tool::{Tool, TurnContext};

#[test]
fn asks_and_receives_answer() {
    let broker = zk_kit_questions::QuestionBroker::new();
    zk_kit_questions::set_global(Arc::clone(&broker));
    let handle = std::thread::spawn(move || {
        let ctx = TurnContext {
            conversation_id: "c".to_owned(),
            workspace_id: "w".to_owned(),
            workspace_root: ".".to_owned(),
        };
        zk_tool_ask_user::tool().call_json(
            &ctx,
            serde_json::json!({"questions":[{"id":"q1","question":"ok?"}]}),
        )
    });
    std::thread::sleep(Duration::from_millis(50));
    assert!(broker.answer("question-1", serde_json::json!({"q1":"yes"})));
    let out = handle.join().unwrap().unwrap();
    assert!(out.contains("yes"));
}
