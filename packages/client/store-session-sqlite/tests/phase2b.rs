#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)] // test-only

use std::path::PathBuf;
use zk_contract_session::{Message, SessionError, SessionStore, WorkspaceStore};
use zk_store_session_sqlite::{open_with_root, SqliteSessionStore};

fn unique(name: &str) -> String {
    format!(
        "{}-{}-{}",
        std::process::id(),
        name,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}

fn open_store(name: &str) -> (SqliteSessionStore, PathBuf) {
    let base = std::env::temp_dir().join(format!("zeokeel-p2b-{}", unique(name)));
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("sessions.db");
    let root = base.join("default-workspace");
    (open_with_root(db, root).unwrap(), base)
}

fn message(id: &str, conversation_id: &str, content: &str) -> Message {
    Message {
        id: id.to_owned(),
        conversation_id: conversation_id.to_owned(),
        role: "user".to_owned(),
        content: content.to_owned(),
        created_at_ms: 1,
        status: "done".to_owned(),
        usage: None,
        turn_id: String::new(),
    }
}

#[test]
fn migration_v6_is_idempotent_and_defaults_flags() {
    let (store, base) = open_store("migrate");
    let workspace = store.list_workspaces().unwrap()[0].clone();
    let conversation = store.create_conversation_in(&workspace.id, "conv").unwrap();
    assert!(!conversation.pinned);
    assert!(!conversation.archived);
    // Re-open the same database: migration must be a no-op (no error, flags kept).
    drop(store);
    let reopened =
        open_with_root(base.join("sessions.db"), base.join("default-workspace")).unwrap();
    let rows = reopened.list_conversations_in(&workspace.id).unwrap();
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].pinned);
    std::fs::remove_dir_all(base).ok();
}

#[test]
fn rename_pin_archive_and_pinned_ordering() {
    let (store, base) = open_store("ordering");
    let workspace = store.list_workspaces().unwrap()[0].clone();
    let first = store
        .create_conversation_in(&workspace.id, "first")
        .unwrap();
    let second = store
        .create_conversation_in(&workspace.id, "second")
        .unwrap();
    let renamed = store
        .rename_conversation(&workspace.id, &first.id, "  renamed  ")
        .unwrap();
    assert_eq!(renamed.title, "renamed");
    assert!(matches!(
        store.rename_conversation(&workspace.id, &first.id, "   "),
        Err(SessionError::Invalid(_))
    ));
    let pinned = store.set_pinned(&workspace.id, &first.id, true).unwrap();
    assert!(pinned.pinned);
    let archived = store.set_archived(&workspace.id, &second.id, true).unwrap();
    assert!(archived.archived);
    let rows = store.list_conversations_in(&workspace.id).unwrap();
    assert_eq!(rows[0].id, first.id, "pinned conversation sorts first");
    assert!(rows.iter().any(|row| row.id == second.id && row.archived));
    // Cross-workspace operations are rejected (C18).
    let other = store.create_workspace(&format!("/tmp/{}", unique("other")), "other");
    if let Ok(other) = other {
        assert!(matches!(
            store.set_pinned(&other.id, &first.id, true),
            Err(SessionError::WorkspaceMismatch { .. })
        ));
    }
    std::fs::remove_dir_all(base).ok();
}

#[test]
fn fork_copies_messages_in_the_same_workspace_only() {
    let (store, base) = open_store("fork");
    let workspace = store.list_workspaces().unwrap()[0].clone();
    let source = store
        .create_conversation_in(&workspace.id, "source")
        .unwrap();
    store.set_pinned(&workspace.id, &source.id, true).unwrap();
    store.set_archived(&workspace.id, &source.id, true).unwrap();
    store
        .append_message(&message("m1", &source.id, "hello"))
        .unwrap();
    store
        .append_message(&message("m2", &source.id, "world"))
        .unwrap();
    let forked = store
        .fork_conversation(&workspace.id, &source.id, None)
        .unwrap();
    assert_ne!(forked.id, source.id);
    assert_eq!(forked.workspace_id, workspace.id);
    assert_eq!(forked.title, "source 副本");
    assert!(!forked.pinned);
    assert!(!forked.archived);
    let messages = store.messages_in(&workspace.id, &forked.id).unwrap();
    assert_eq!(messages.len(), 2);
    // Same-created_at rows order by id, so assert the copied set rather than the order.
    let contents: Vec<&str> = messages
        .iter()
        .map(|message| message.content.as_str())
        .collect();
    assert!(contents.contains(&"hello"));
    assert!(contents.contains(&"world"));
    // Copied rows get new message ids (no primary-key collision).
    assert!(messages
        .iter()
        .all(|message| message.id != "m1" && message.id != "m2"));
    // Unknown source is a not-found error.
    assert!(matches!(
        store.fork_conversation(&workspace.id, "missing", None),
        Err(SessionError::NotFound(_))
    ));
    // Trait default implementation keeps third-party stores compiling.
    std::fs::remove_dir_all(base).ok();
}
