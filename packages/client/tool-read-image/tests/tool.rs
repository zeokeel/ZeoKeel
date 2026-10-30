#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn reads_png_data_url() {
    let dir = std::env::temp_dir().join(format!("zk-img-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.png"), [0x89, 0x50, 0x4e, 0x47]).unwrap();
    let ctx = TurnContext { conversation_id: "c".to_owned(), workspace_id: "w".to_owned(), workspace_root: dir.display().to_string() };
    let out = zk_tool_read_image::tool().call_json(&ctx, serde_json::json!({"path":"a.png"})).unwrap();
    assert!(out.starts_with("data:image/png;base64,"));
}
