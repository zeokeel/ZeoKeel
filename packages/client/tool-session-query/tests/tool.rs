#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use std::sync::Arc;
use zk_contract_session::{Message, SessionStore, WorkspaceStore};
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn searches_session_history() {
    let dir = std::env::temp_dir().join(format!("zk-sess-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&dir).unwrap();
    let workspace_root = dir.join("ws"); std::fs::create_dir_all(&workspace_root).unwrap();
    let store = zk_store_session_sqlite::open_with_root(dir.join("sessions.db"), workspace_root).unwrap();
    let workspace = store.list_workspaces().unwrap().first().cloned().unwrap();
    let conversation = store.create_conversation_in(&workspace.id, "t").unwrap();
    store.append_message(&Message { id: "m1".to_owned(), conversation_id: conversation.id.clone(), role: "user".to_owned(), content: "needle".to_owned(), created_at_ms: 1, status: "done".to_owned() }).unwrap();
    zk_kit_session::set_global(Arc::new(store));
    let ctx = TurnContext { conversation_id: conversation.id, workspace_id: workspace.id, workspace_root: workspace.path };
    let out = zk_tool_session_query::tool().call_json(&ctx, serde_json::json!({"action":"search","query":"needle"})).unwrap();
    assert!(out.contains("needle"));
}
