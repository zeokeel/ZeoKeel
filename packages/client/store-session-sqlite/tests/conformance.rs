#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)] // test-only

use std::path::PathBuf;
use zk_contract_agent::AgentUsage;
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
    let base = std::env::temp_dir().join(format!("zeokeel-ws-{}", unique(name)));
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("sessions.db");
    let root = base.join("default-workspace");
    (open_with_root(db, root).unwrap(), base)
}

fn message(id: &str, conversation_id: &str, role: &str, content: &str) -> Message {
    Message {
        id: id.to_owned(),
        conversation_id: conversation_id.to_owned(),
        role: role.to_owned(),
        content: content.to_owned(),
        created_at_ms: 1,
        status: "done".to_owned(),
        usage: None,
        turn_id: "turn-1".to_owned(),
    }
}

#[test]
fn default_workspace_is_created_and_listed() {
    let (store, _) = open_store("default");
    let list = store.list_workspaces().unwrap();
    assert_eq!(list.len(), 1);
    assert!(list[0].is_default);
    assert!(list[0].path.ends_with("default-workspace"));
}

#[test]
fn create_workspace_and_isolate_conversations() {
    let (store, base) = open_store("isolate");
    let other_dir = base.join("other");
    std::fs::create_dir_all(&other_dir).unwrap();
    let other = store
        .create_workspace(other_dir.to_str().unwrap(), "其他")
        .unwrap();
    assert!(!other.is_default);
    let w1 = store
        .list_workspaces()
        .unwrap()
        .into_iter()
        .find(|w| w.is_default)
        .unwrap();
    let a = store.create_conversation_in(&w1.id, "会话 A").unwrap();
    assert_eq!(a.workspace_id, w1.id);
    let b = store.create_conversation_in(&other.id, "会话 B").unwrap();
    store
        .append_message(&message("m1", &a.id, "user", "你好"))
        .unwrap();
    store
        .append_message(&message("m2", &a.id, "assistant", "收到"))
        .unwrap();
    assert_eq!(store.messages_in(&w1.id, &a.id).unwrap().len(), 2);
    assert!(store.messages_in(&other.id, &b.id).unwrap().is_empty());
    assert_eq!(store.list_conversations_in(&w1.id).unwrap().len(), 1);
    assert_eq!(store.list_conversations_in(&other.id).unwrap().len(), 1);
    assert!(matches!(
        store.messages_in(&other.id, &a.id),
        Err(SessionError::WorkspaceMismatch { .. })
    ));
}

#[test]
fn workspace_crud_default_and_removal() {
    let (store, base) = open_store("crud");
    let dir = base.join("project");
    std::fs::create_dir_all(&dir).unwrap();
    let ws = store
        .create_workspace(dir.to_str().unwrap(), "项目")
        .unwrap();
    assert!(matches!(
        store.create_workspace(dir.to_str().unwrap(), "dup"),
        Err(SessionError::DuplicateWorkspacePath(_))
    ));
    let renamed = store.rename_workspace(&ws.id, "项目二").unwrap();
    assert_eq!(renamed.title, "项目二");
    let list = store.set_default_workspace(&ws.id).unwrap();
    assert_eq!(list.iter().filter(|w| w.is_default).count(), 1);
    assert!(list.iter().find(|w| w.id == ws.id).unwrap().is_default);

    let conv = store.create_conversation_in(&ws.id, "会话").unwrap();
    assert!(matches!(
        store.remove_workspace(&ws.id, None, true),
        Err(SessionError::DefaultWorkspace(_))
    ));
    let old_default = store
        .list_workspaces()
        .unwrap()
        .into_iter()
        .find(|w| !w.is_default)
        .unwrap();
    store.set_default_workspace(&old_default.id).unwrap();
    assert!(matches!(
        store.remove_workspace(&ws.id, Some(&old_default.id), false),
        Err(SessionError::ConfirmRequired(_))
    ));
    let res = store
        .remove_workspace(&ws.id, Some(&old_default.id), true)
        .unwrap();
    assert!(res.removed);
    assert_eq!(res.moved_sessions, 1);
    assert_eq!(
        store.messages_in(&old_default.id, &conv.id).unwrap().len(),
        0
    );

    let default_now = store
        .list_workspaces()
        .unwrap()
        .into_iter()
        .find(|w| w.is_default)
        .unwrap();
    assert!(matches!(
        store.remove_workspace(&default_now.id, None, true),
        Err(SessionError::DefaultWorkspace(_))
    ));
}

#[test]
fn legacy_v1_database_migrates_without_losing_messages() {
    let base = std::env::temp_dir().join(format!("zeokeel-legacy-{}", unique("migrate")));
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("sessions.db");
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(
            "CREATE TABLE conversations (id TEXT PRIMARY KEY, title TEXT NOT NULL, created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL);
             CREATE TABLE messages (id TEXT PRIMARY KEY, conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE, role TEXT NOT NULL, content TEXT NOT NULL, created_at_ms INTEGER NOT NULL);
             INSERT INTO conversations VALUES ('c1','旧会话',1,2);
             INSERT INTO messages VALUES ('m1','c1','user','旧消息',1);
             PRAGMA user_version=1;",
        )
        .unwrap();
    }
    let root = base.join("ws");
    let store = open_with_root(db, root).unwrap();
    let ws = store
        .list_workspaces()
        .unwrap()
        .into_iter()
        .find(|w| w.is_default)
        .unwrap();
    let list = store.list_conversations_in(&ws.id).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "c1");
    assert_eq!(list[0].workspace_id, ws.id);
    assert_eq!(
        store.messages_in(&ws.id, "c1").unwrap()[0].content,
        "旧消息"
    );
}

#[test]
fn reopens_persisted_workspace_and_conversation() {
    let (store, base) = open_store("reopen");
    let dir = base.join("persist");
    std::fs::create_dir_all(&dir).unwrap();
    let ws = store
        .create_workspace(dir.to_str().unwrap(), "持久化")
        .unwrap();
    let conv = store.create_conversation_in(&ws.id, "会话").unwrap();
    store
        .append_message(&message("m1", &conv.id, "user", "hi"))
        .unwrap();
    let db = base.join("sessions.db");
    drop(store);
    let reopened = open_with_root(db, base.join("default-workspace")).unwrap();
    assert_eq!(reopened.list_conversations_in(&ws.id).unwrap().len(), 1);
    assert_eq!(reopened.messages_in(&ws.id, &conv.id).unwrap().len(), 1);
}

#[test]
fn persists_message_status_for_cancel_markers() {
    let (store, _) = open_store("status");
    let ws = store
        .list_workspaces()
        .unwrap()
        .into_iter()
        .find(|w| w.is_default)
        .unwrap();
    let conv = store.create_conversation_in(&ws.id, "status").unwrap();
    let mut cancelled = message("c1", &conv.id, "assistant", "部分输出");
    cancelled.status = "cancelled".to_owned();
    store.append_message(&cancelled).unwrap();
    let loaded = store.messages_in(&ws.id, &conv.id).unwrap();
    assert_eq!(loaded[0].status, "cancelled");
}

#[test]
fn persists_message_usage_across_reopen() {
    let (store, base) = open_store("usage");
    let ws = store.list_workspaces().unwrap().into_iter().next().unwrap();
    let conv = store.create_conversation_in(&ws.id, "usage").unwrap();
    let usage = AgentUsage {
        input_tokens: 11,
        output_tokens: 5,
        total_tokens: 16,
        cached_tokens: 4,
        reasoning_tokens: 2,
        cost_micros: Some(42),
    };
    let mut assistant = message("m-a", &conv.id, "assistant", "answer");
    assistant.usage = Some(usage);
    store.append_message(&assistant).unwrap();
    store
        .append_message(&message("m-u", &conv.id, "user", "question"))
        .unwrap();
    drop(store);

    let reopened =
        open_with_root(base.join("sessions.db"), base.join("default-workspace")).unwrap();
    let messages = reopened.messages_in(&ws.id, &conv.id).unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].usage, Some(usage));
    assert!(messages[1].usage.is_none());

    // Schema is v5 and legacy NULL rows read back as None.
    let conn = rusqlite::Connection::open(base.join("sessions.db")).unwrap();
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 5);
    drop(conn);
    drop(reopened);
}

#[test]
fn corrupt_usage_json_degrades_to_none_without_panicking() {
    let (store, base) = open_store("usage-corrupt");
    let ws = store.list_workspaces().unwrap().into_iter().next().unwrap();
    let conv = store.create_conversation_in(&ws.id, "corrupt").unwrap();
    store
        .append_message(&message("m1", &conv.id, "assistant", "answer"))
        .unwrap();
    drop(store);

    let db = base.join("sessions.db");
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute(
        "UPDATE messages SET usage_json='{\"input_tokens\": ' WHERE id='m1'",
        [],
    )
    .unwrap();
    drop(conn);

    let reopened = open_with_root(db, base.join("default-workspace")).unwrap();
    let messages = reopened.messages_in(&ws.id, &conv.id).unwrap();
    assert_eq!(messages.len(), 1);
    assert!(
        messages[0].usage.is_none(),
        "corrupt usage must degrade to None"
    );
}

#[test]
fn legacy_v3_database_upgrades_to_v4_with_usage_null() {
    let base = std::env::temp_dir().join(format!("zeokeel-v3-{}", unique("v4")));
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("sessions.db");
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(
            "CREATE TABLE workspaces (id TEXT PRIMARY KEY, path TEXT NOT NULL UNIQUE, title TEXT NOT NULL, is_default INTEGER NOT NULL DEFAULT 0, created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL);
             CREATE TABLE conversations (id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL REFERENCES workspaces(id), title TEXT NOT NULL, created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL);
             CREATE TABLE messages (id TEXT PRIMARY KEY, conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE, role TEXT NOT NULL, content TEXT NOT NULL, created_at_ms INTEGER NOT NULL, status TEXT NOT NULL DEFAULT 'done');
             INSERT INTO workspaces VALUES ('w1', 'ROOT', 'ws', 1, 1, 1);
             INSERT INTO conversations VALUES ('c1', 'w1', 't', 1, 1);
             INSERT INTO messages VALUES ('m1', 'c1', 'assistant', 'old', 1, 'done');
             PRAGMA user_version=3;",
        )
        .unwrap();
    }
    let root = base.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let store = open_with_root(db.clone(), root.clone()).unwrap();
    let messages = store.messages_in("w1", "c1").unwrap();
    assert_eq!(messages.len(), 1);
    assert!(messages[0].usage.is_none());
    drop(store);
    let conn = rusqlite::Connection::open(&db).unwrap();
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 5, "v3 -> v5 upgrade path");
}

#[test]
fn turn_id_round_trips_and_preserves_write_order() {
    let (store, _) = open_store("turn-id");
    let workspace = store.list_workspaces().unwrap().first().cloned().unwrap();
    let conversation = store.create_conversation_in(&workspace.id, "t").unwrap();
    let rows = vec![
        ("user", "u1", "user", "", 1),
        ("tool", "t1", "tool", "{\"name\":\"fs.write\"}", 2),
        ("tool", "t2", "tool", "{\"name\":\"bash\"}", 3),
        ("assistant", "a1", "assistant", "done", 4),
    ];
    for (turn, id, role, content, created) in rows {
        let mut row = message(id, &conversation.id, role, content);
        row.created_at_ms = created;
        row.turn_id = turn.to_owned();
        store.append_message(&row).unwrap();
    }
    let read = store.messages_in(&workspace.id, &conversation.id).unwrap();
    assert_eq!(
        read.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
        vec!["u1", "t1", "t2", "a1"]
    );
    assert_eq!(read[1].turn_id, "tool");
    assert_eq!(read[3].turn_id, "assistant");
    assert_eq!(read[0].turn_id, "user");
}

#[test]
fn v4_database_is_migrated_to_v5_with_empty_turn_ids() {
    let base = std::env::temp_dir().join(format!("zeokeel-ws-v4-{}", unique("v4")));
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("sessions.db");
    let root = base.join("default-workspace");
    std::fs::create_dir_all(&root).unwrap();
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(
            "CREATE TABLE workspaces (
                id TEXT PRIMARY KEY, path TEXT NOT NULL UNIQUE, title TEXT NOT NULL,
                is_default INTEGER NOT NULL DEFAULT 0, created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL
            );
            CREATE TABLE conversations (
                id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL REFERENCES workspaces(id),
                title TEXT NOT NULL, created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL
            );
            CREATE TABLE messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
                role TEXT NOT NULL, content TEXT NOT NULL, created_at_ms INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'done', usage_json TEXT
            );
            INSERT INTO workspaces VALUES ('w1', 'ROOT', 't', 1, 1, 1);
            INSERT INTO conversations VALUES ('c1', 'w1', 't', 1, 1);
            INSERT INTO messages (id, conversation_id, role, content, created_at_ms, status)
                VALUES ('legacy-1', 'c1', 'user', 'hi', 1, 'done');
            PRAGMA user_version=4;"
                .replace("ROOT", &root.display().to_string())
                .as_str(),
        )
        .unwrap();
    }
    let store = open_with_root(db.clone(), root).unwrap();
    let version: i64 = {
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap()
    };
    assert_eq!(version, 5);
    let read = store.messages_in("w1", "c1").unwrap();
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].turn_id, "");
}
