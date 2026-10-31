#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};

fn ctx(tag: &str) -> TurnContext {
    let dir = std::env::temp_dir().join(format!(
        "zk-terminal-{tag}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(dir.join("alpha")).unwrap();
    std::fs::create_dir_all(dir.join("beta")).unwrap();
    TurnContext {
        conversation_id: format!("conv-{tag}"),
        workspace_id: "w".to_owned(),
        workspace_root: dir.display().to_string(),
    }
}

#[test]
fn default_session_open_send_close() {
    let ctx = ctx("default");
    let tool = zk_tool_terminal::tool();
    assert!(tool
        .call_json(&ctx, serde_json::json!({"action":"open"}))
        .unwrap()
        .contains("terminal open"));
    assert!(tool
        .call_json(
            &ctx,
            serde_json::json!({"action":"send","command":"printf hi"})
        )
        .unwrap()
        .contains("hi"));
    assert!(tool
        .call_json(&ctx, serde_json::json!({"action":"close"}))
        .unwrap()
        .contains("closed=true"));
}

#[test]
fn sessions_are_independent_and_listed() {
    let ctx = ctx("multi");
    let tool = zk_tool_terminal::tool();
    tool.call_json(&ctx, serde_json::json!({"action":"open","session":"s1"}))
        .unwrap();
    tool.call_json(&ctx, serde_json::json!({"action":"open","session":"s2"}))
        .unwrap();
    tool.call_json(
        &ctx,
        serde_json::json!({"action":"send","session":"s1","command":"cd alpha"}),
    )
    .unwrap();
    tool.call_json(
        &ctx,
        serde_json::json!({"action":"send","session":"s2","command":"cd beta"}),
    )
    .unwrap();
    assert!(tool
        .call_json(
            &ctx,
            serde_json::json!({"action":"send","session":"s1","command":"pwd"})
        )
        .unwrap()
        .contains("alpha"));
    assert!(tool
        .call_json(
            &ctx,
            serde_json::json!({"action":"send","session":"s2","command":"pwd"})
        )
        .unwrap()
        .contains("beta"));
    let listed = tool
        .call_json(&ctx, serde_json::json!({"action":"status"}))
        .unwrap();
    assert!(listed.contains("\"s1\""));
    assert!(listed.contains("\"s2\""));
    assert!(tool
        .call_json(&ctx, serde_json::json!({"action":"close","session":"s1"}))
        .unwrap()
        .contains("closed=true"));
    let listed = tool
        .call_json(&ctx, serde_json::json!({"action":"status"}))
        .unwrap();
    assert!(!listed.contains("\"s1\""));
    assert!(listed.contains("\"s2\""));
    // Closing one session leaves the other usable.
    assert!(tool
        .call_json(
            &ctx,
            serde_json::json!({"action":"send","session":"s2","command":"pwd"})
        )
        .unwrap()
        .contains("beta"));
    tool.call_json(&ctx, serde_json::json!({"action":"close","session":"s2"}))
        .unwrap();
}

#[test]
fn send_requires_open_session() {
    let ctx = ctx("unknown");
    let tool = zk_tool_terminal::tool();
    let error = tool
        .call_json(
            &ctx,
            serde_json::json!({"action":"send","session":"missing","command":"printf x"}),
        )
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("unknown session"),
        "unexpected error: {error}"
    );
}

#[test]
fn invalid_session_id_is_rejected() {
    let ctx = ctx("id");
    let tool = zk_tool_terminal::tool();
    let error = tool
        .call_json(
            &ctx,
            serde_json::json!({"action":"open","session":"bad id!"}),
        )
        .unwrap_err()
        .to_string();
    assert!(error.contains("session id"), "unexpected error: {error}");
}

#[test]
fn unknown_action_is_rejected() {
    let ctx = ctx("action");
    let tool = zk_tool_terminal::tool();
    let error = tool
        .call_json(&ctx, serde_json::json!({"action":"detach"}))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("unknown action"),
        "unexpected error: {error}"
    );
}
