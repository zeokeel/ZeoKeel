#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::TurnContext;

#[test]
fn background_job_roundtrip() {
    let dir = std::env::temp_dir().join(format!(
        "zk-jobs-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let ctx = TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: dir.display().to_string(),
    };
    let id = zk_kit_jobs::start(&ctx, "printf hello", None).unwrap();
    // Poll instead of a fixed sleep: a loaded CI runner may take longer than 200ms
    // to schedule the background process, which made this test flaky.
    let mut out = zk_kit_jobs::output(&id).unwrap();
    for _ in 0..200 {
        if out.get("stdout").and_then(serde_json::Value::as_str) == Some("hello") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
        out = zk_kit_jobs::output(&id).unwrap();
    }
    assert_eq!(
        out.get("stdout").and_then(serde_json::Value::as_str),
        Some("hello")
    );
    let list = zk_kit_jobs::list().unwrap();
    assert!(list
        .as_array()
        .map(|items| !items.is_empty())
        .unwrap_or(false));
}
