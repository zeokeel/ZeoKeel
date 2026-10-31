#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};

fn ctx(tag: &str) -> TurnContext {
    let dir = std::env::temp_dir().join(format!(
        "zk-pwsh-{tag}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    TurnContext {
        conversation_id: format!("conv-{tag}"),
        workspace_id: "w".to_owned(),
        workspace_root: dir.display().to_string(),
    }
}

/// On a host without `pwsh` the tool must fail with TOOL_UNSUPPORTED_PLATFORM; on a Windows (or
/// pwsh-equipped) host the same call exercises a real long-lived session. No platform is faked.
#[test]
fn open_is_platform_gated_or_works() {
    let ctx = ctx("gate");
    let tool = zk_tool_pwsh_persistent::tool();
    match tool.call_json(&ctx, serde_json::json!({"action":"open","session":"pw1"})) {
        Ok(output) => {
            assert!(
                output.contains("pwsh_persistent open"),
                "unexpected output: {output}"
            );
            let sent = tool
                .call_json(
                    &ctx,
                    serde_json::json!({"action":"send","session":"pw1","command":"Write-Output 'zk-pwsh-ok'"}),
                )
                .unwrap();
            assert!(sent.contains("zk-pwsh-ok"), "unexpected output: {sent}");
            assert!(tool
                .call_json(&ctx, serde_json::json!({"action":"close","session":"pw1"}))
                .unwrap()
                .contains("closed=true"));
        }
        Err(error) => {
            let message = error.to_string();
            assert!(
                message.contains("TOOL_UNSUPPORTED_PLATFORM"),
                "unexpected pwsh error: {message}"
            );
        }
    }
}

#[test]
fn send_before_open_is_unknown_session() {
    let ctx = ctx("unknown");
    let tool = zk_tool_pwsh_persistent::tool();
    let error = tool
        .call_json(
            &ctx,
            serde_json::json!({"action":"send","session":"missing","command":"Write-Output x"}),
        )
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("unknown session"),
        "unexpected error: {error}"
    );
}

#[test]
fn missing_action_is_invalid() {
    let ctx = ctx("invalid");
    let tool = zk_tool_pwsh_persistent::tool();
    assert!(tool
        .call_json(&ctx, serde_json::json!({"command":"x"}))
        .is_err());
}
