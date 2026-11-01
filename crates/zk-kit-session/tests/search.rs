#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use zk_contract_session::{Message, SessionStore, WorkspaceStore};
use zk_store_session_sqlite::open_with_root;

fn message(id: &str, conversation_id: &str, role: &str, content: &str) -> Message {
    Message {
        id: id.to_owned(),
        conversation_id: conversation_id.to_owned(),
        role: role.to_owned(),
        content: content.to_owned(),
        created_at_ms: 1,
        status: "done".to_owned(),
        usage: None,
        turn_id: String::new(),
    }
}

#[test]
fn search_is_case_insensitive_snippet_bounded_and_workspace_scoped() {
    let base = std::env::temp_dir().join(format!(
        "zk-kit-search-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&base).unwrap();
    let store = open_with_root(base.join("sessions.db"), base.join("ws")).unwrap();
    let workspace = store.list_workspaces().unwrap().first().cloned().unwrap();
    let conversation = store.create_conversation_in(&workspace.id, "conv").unwrap();
    store
        .append_message(&message(
            "m1",
            &conversation.id,
            "user",
            &format!("prefix {} needle suffix", "x".repeat(200)),
        ))
        .unwrap();
    store
        .append_message(&message("m2", &conversation.id, "assistant", "NO MATCH"))
        .unwrap();
    let hits = zk_kit_session::search(&store, &workspace.id, "NEEDLE", 20).unwrap();
    assert_eq!(hits.items.len(), 1);
    let hit = hits.items.first().unwrap();
    assert_eq!(hit.conversation_id, conversation.id);
    assert_eq!(hit.message_id, "m1");
    assert!(hit.snippet.to_lowercase().contains("needle"));
    assert!(hit.snippet.chars().count() <= 60 + "needle".len() + 60);
    // Empty query short-circuits; limit is bounded.
    assert!(zk_kit_session::search(&store, &workspace.id, "   ", 20)
        .unwrap()
        .items
        .is_empty());
    let bounded = zk_kit_session::search(&store, &workspace.id, "needle", 500).unwrap();
    assert_eq!(bounded.items.len(), 1);
    std::fs::remove_dir_all(base).ok();
}
