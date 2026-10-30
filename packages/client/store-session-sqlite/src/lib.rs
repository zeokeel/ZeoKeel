//! SQLite session store (minimal M1 implementation).
//!
//! @capability store.session.sqlite
#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::{params, Connection};
use zk_contract_session::{Conversation, Message, SessionError, SessionStore};

/// SQLite-backed session store (WAL).
pub struct SqliteSessionStore {
    conn: Mutex<Option<Connection>>,
}

/// Default store location: `$ZEOKEEL_HOME/sessions.db` or the platform data dir.
#[must_use]
pub fn store() -> SqliteSessionStore {
    match open(default_path()) {
        Ok(store) => store,
        Err(_) => SqliteSessionStore { conn: Mutex::new(None) },
    }
}

/// Open (and migrate) a store at `path`.
pub fn open(path: PathBuf) -> Result<SqliteSessionStore, SessionError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(storage)?;
    }
    let conn = Connection::open(path).map_err(storage)?;
    conn.pragma_update(None, "journal_mode", "WAL").map_err(storage)?;
    conn.pragma_update(None, "foreign_keys", "ON").map_err(storage)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS conversations (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            created_at_ms INTEGER NOT NULL,
            updated_at_ms INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_messages_conversation
            ON messages(conversation_id, created_at_ms);",
    )
    .map_err(storage)?;
    Ok(SqliteSessionStore { conn: Mutex::new(Some(conn)) })
}

fn default_path() -> PathBuf {
    if let Ok(home) = std::env::var("ZEOKEEL_HOME") {
        if !home.is_empty() {
            return PathBuf::from(home).join("sessions.db");
        }
    }
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("zeokeel").join("sessions.db")
}

fn storage(error: impl std::fmt::Display) -> SessionError {
    SessionError::Storage(error.to_string())
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

impl SqliteSessionStore {
    fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> Result<T, SessionError>) -> Result<T, SessionError> {
        let guard = self.conn.lock().map_err(|_| storage("session store poisoned"))?;
        let conn = guard.as_ref().ok_or_else(|| storage("session store unavailable"))?;
        f(conn)
    }
}

impl SessionStore for SqliteSessionStore {
    fn create_conversation(&self, title: &str) -> Result<Conversation, SessionError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(SessionError::Invalid("empty conversation title".to_owned()));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_ms();
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO conversations (id, title, created_at_ms, updated_at_ms) VALUES (?1, ?2, ?3, ?3)",
                params![id, title, now],
            )
            .map_err(storage)
        })?;
        Ok(Conversation { id, title: title.to_owned(), created_at_ms: now, updated_at_ms: now })
    }

    fn list_conversations(&self) -> Result<Vec<Conversation>, SessionError> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare("SELECT id, title, created_at_ms, updated_at_ms FROM conversations ORDER BY updated_at_ms DESC, id DESC")
                .map_err(storage)?;
            let rows = stmt
                .query_map([], |row| {
                    Ok(Conversation {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        created_at_ms: row.get::<_, i64>(2)?.max(0).try_into().unwrap_or(0),
                        updated_at_ms: row.get::<_, i64>(3)?.max(0).try_into().unwrap_or(0),
                    })
                })
                .map_err(storage)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(storage)
        })
    }

    fn append_message(&self, message: &Message) -> Result<(), SessionError> {
        if message.id.trim().is_empty() || message.conversation_id.trim().is_empty() {
            return Err(SessionError::Invalid("empty message id/conversation id".to_owned()));
        }
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO messages (id, conversation_id, role, content, created_at_ms) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![message.id, message.conversation_id, message.role, message.content, message.created_at_ms as i64],
            )
            .map_err(|error| match error {
                rusqlite::Error::SqliteFailure(_, _) => SessionError::Storage(error.to_string()),
                other => SessionError::Storage(other.to_string()),
            })?;
            conn.execute(
                "UPDATE conversations SET updated_at_ms = ?2 WHERE id = ?1",
                params![message.conversation_id, now_ms() as i64],
            )
            .map_err(storage)?;
            Ok(())
        })
    }

    fn messages(&self, conversation_id: &str) -> Result<Vec<Message>, SessionError> {
        self.with_conn(|conn| {
            let exists: i64 = conn
                .query_row("SELECT COUNT(1) FROM conversations WHERE id = ?1", params![conversation_id], |row| row.get(0))
                .map_err(storage)?;
            if exists == 0 {
                return Err(SessionError::NotFound(conversation_id.to_owned()));
            }
            let mut stmt = conn
                .prepare("SELECT id, conversation_id, role, content, created_at_ms FROM messages WHERE conversation_id = ?1 ORDER BY created_at_ms ASC, id ASC")
                .map_err(storage)?;
            let rows = stmt
                .query_map(params![conversation_id], |row| {
                    Ok(Message {
                        id: row.get(0)?,
                        conversation_id: row.get(1)?,
                        role: row.get(2)?,
                        content: row.get(3)?,
                        created_at_ms: row.get::<_, i64>(4)?.max(0).try_into().unwrap_or(0),
                    })
                })
                .map_err(storage)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(storage)
        })
    }
}
