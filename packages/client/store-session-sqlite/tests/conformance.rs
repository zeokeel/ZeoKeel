#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use std::path::PathBuf;
use zk_contract_session::{Message, SessionStore};
use zk_store_session_sqlite::open;

fn temp_db(name: &str) -> PathBuf {
    let unique = format!("{}-{}-{}", std::process::id(), name, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
    std::env::temp_dir().join(format!("zeokeel-test-{unique}.db"))
}

fn message(id: &str, conversation_id: &str, role: &str, content: &str) -> Message {
    Message { id: id.to_owned(), conversation_id: conversation_id.to_owned(), role: role.to_owned(), content: content.to_owned(), created_at_ms: 1 }
}

#[test]
fn create_list_append_read_and_isolate() {
    let path = temp_db("sessions");
    let store = open(path.clone()).unwrap();
    let a = store.create_conversation("会话 A").unwrap();
    let b = store.create_conversation("会话 B").unwrap();
    store.append_message(&message("m1", &a.id, "user", "你好")).unwrap();
    store.append_message(&message("m2", &a.id, "assistant", "收到")).unwrap();
    assert_eq!(store.messages(&a.id).unwrap().len(), 2);
    assert!(store.messages(&b.id).unwrap().is_empty());
    let list = store.list_conversations().unwrap();
    assert_eq!(list.len(), 2);
    assert!(store.messages("missing").is_err());
}

#[test]
fn reopens_persisted_data() {
    let path = temp_db("reopen");
    let id = {
        let store = open(path.clone()).unwrap();
        let c = store.create_conversation("持久化").unwrap();
        store.append_message(&message("m1", &c.id, "user", "hi")).unwrap();
        c.id
    };
    let store = open(path).unwrap();
    assert_eq!(store.messages(&id).unwrap().len(), 1);
    assert_eq!(store.list_conversations().unwrap().len(), 1);
}
