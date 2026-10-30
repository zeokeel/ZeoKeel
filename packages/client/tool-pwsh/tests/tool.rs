#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn pwsh_has_platform_error_or_runs() {
    let dir = std::env::temp_dir();
    let ctx = TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: dir.display().to_string(),
    };
    let result =
        zk_tool_pwsh::tool().call_json(&ctx, serde_json::json!({"command":"Write-Output hi"}));
    if let Err(error) = result {
        assert!(
            error.to_string().contains("TOOL_UNSUPPORTED_PLATFORM")
                || error.to_string().contains("io")
        );
    }
}
