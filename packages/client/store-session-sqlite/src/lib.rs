//! SQLite session/workspace store (M1 workspace-scoped implementation).
//!
//! @capability store.session.sqlite
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};
use zk_contract_session::{
    Conversation, Message, RemoveWorkspaceResult, SessionError, SessionStore, Workspace,
    WorkspaceStore,
};

/// SQLite-backed workspace + session store (WAL).
pub struct SqliteSessionStore {
    conn: Mutex<Option<Connection>>,
}

/// Default store location: `$ZEOKEEL_HOME/sessions.db` or the platform data dir.
pub fn store() -> Result<SqliteSessionStore, SessionError> {
    let path = default_path();
    let root = default_workspace_root()?;
    open_with_root(path, root)
}

/// Open (and migrate) a store at `path`; the default workspace is the user home (`~`).
pub fn open(path: PathBuf) -> Result<SqliteSessionStore, SessionError> {
    let root = default_workspace_root()?;
    open_with_root(path, root)
}

/// Open a store with an explicit default workspace root (tests / embedded use).
pub fn open_with_root(
    path: PathBuf,
    workspace_root: PathBuf,
) -> Result<SqliteSessionStore, SessionError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(storage)?;
    }
    let mut conn = Connection::open(&path).map_err(storage)?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(storage)?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(storage)?;
    let version = schema_version(&conn)?;
    if version < 3 && path.exists() {
        backup(&conn, &path)?;
    }
    migrate(&mut conn, &workspace_root)?;
    Ok(SqliteSessionStore {
        conn: Mutex::new(Some(conn)),
    })
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

fn default_workspace_root() -> Result<PathBuf, SessionError> {
    // Per product decision: the default workspace is the user's home (`~`).
    // A custom ZEOKEEL_HOME is a test/embedding escape hatch, not the UI default.
    if let Ok(home) = std::env::var("ZEOKEEL_HOME") {
        if !home.is_empty() {
            return Ok(PathBuf::from(home));
        }
    }
    dirs::home_dir().ok_or_else(|| {
        SessionError::WorkspacePathUnavailable(
            "home directory unavailable; choose a workspace directory".to_owned(),
        )
    })
}

fn storage(error: impl std::fmt::Display) -> SessionError {
    SessionError::Storage(error.to_string())
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

fn schema_version(conn: &Connection) -> Result<i64, SessionError> {
    conn.query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(storage)
}

fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, SessionError> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(storage)?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(storage)?;
    for name in rows {
        if name.map_err(storage)? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn has_table(conn: &Connection, table: &str) -> Result<bool, SessionError> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(1) FROM sqlite_master WHERE type='table' AND name=?1",
            params![table],
            |row| row.get(0),
        )
        .map_err(storage)?;
    Ok(count > 0)
}

fn backup(conn: &Connection, path: &Path) -> Result<(), SessionError> {
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return Ok(());
    };
    let bak = path.with_file_name(format!("{name}.bak-v1"));
    std::fs::copy(path, &bak).map_err(storage)?;
    for suffix in ["-wal", "-shm"] {
        let from = path.with_file_name(format!("{name}{suffix}"));
        if from.exists() {
            std::fs::copy(&from, path.with_file_name(format!("{name}.bak-v1{suffix}")))
                .map_err(storage)?;
        }
    }
    Ok(())
}

fn create_schema_v2(conn: &Connection) -> Result<(), SessionError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS workspaces (
            id TEXT PRIMARY KEY,
            path TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            is_default INTEGER NOT NULL DEFAULT 0,
            created_at_ms INTEGER NOT NULL,
            updated_at_ms INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS conversations (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL REFERENCES workspaces(id),
            title TEXT NOT NULL,
            created_at_ms INTEGER NOT NULL,
            updated_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_conversations_workspace
            ON conversations(workspace_id, updated_at_ms DESC);
        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at_ms INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'done'
        );
        CREATE INDEX IF NOT EXISTS idx_messages_conversation
            ON messages(conversation_id, created_at_ms);",
    )
    .map_err(storage)
}

fn normalize_workspace_root(root: &Path) -> Result<PathBuf, SessionError> {
    std::fs::create_dir_all(root).map_err(|error| {
        SessionError::WorkspacePathUnavailable(format!("{}: {error}", root.display()))
    })?;
    std::fs::canonicalize(root).map_err(|error| {
        SessionError::WorkspacePathUnavailable(format!("{}: {error}", root.display()))
    })
}

fn migrate(conn: &mut Connection, workspace_root: &Path) -> Result<(), SessionError> {
    let version = schema_version(conn)?;
    let default_path = normalize_workspace_root(workspace_root)?;
    let default_id = existing_default_id(conn)?.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let now = now_ms();
    if !has_table(conn, "conversations")? {
        create_schema_v2(conn)?;
        conn.execute(
            "INSERT OR IGNORE INTO workspaces (id, path, title, is_default, created_at_ms, updated_at_ms)
             VALUES (?1, ?2, ?3, 1, ?4, ?4)",
            params![default_id, default_path.to_string_lossy(), "默认工作区", now],
        )
        .map_err(storage)?;
        conn.pragma_update(None, "user_version", 2)
            .map_err(storage)?;
        return Ok(());
    }
    let legacy = !has_column(conn, "conversations", "workspace_id")?;
    if legacy || version < 2 {
        conn.execute_batch("PRAGMA foreign_keys=OFF;")
            .map_err(storage)?;
        let tx = conn.transaction().map_err(storage)?;
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS workspaces (
                id TEXT PRIMARY KEY, path TEXT NOT NULL UNIQUE, title TEXT NOT NULL,
                is_default INTEGER NOT NULL DEFAULT 0, created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL
            );",
        )
        .map_err(storage)?;
        tx.execute(
            "INSERT OR IGNORE INTO workspaces (id, path, title, is_default, created_at_ms, updated_at_ms)
             VALUES (?1, ?2, ?3, 1, ?4, ?4)",
            params![default_id, default_path.to_string_lossy(), "默认工作区", now],
        )
        .map_err(storage)?;
        tx.execute_batch(
            "CREATE TABLE conversations_new (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL REFERENCES workspaces(id),
                title TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL
            );
            CREATE TABLE messages_new (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL REFERENCES conversations_new(id) ON DELETE CASCADE,
                role TEXT NOT NULL, content TEXT NOT NULL, created_at_ms INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'done'
            );",
        )
        .map_err(storage)?;
        let has_workspace = has_column(&tx, "conversations", "workspace_id")?;
        if has_workspace {
            tx.execute(
                "INSERT INTO conversations_new SELECT id, workspace_id, title, created_at_ms, updated_at_ms FROM conversations",
                [],
            )
            .map_err(storage)?;
        } else {
            tx.execute(
                "INSERT INTO conversations_new (id, workspace_id, title, created_at_ms, updated_at_ms)
                 SELECT id, ?1, title, created_at_ms, updated_at_ms FROM conversations",
                params![default_id],
            )
            .map_err(storage)?;
        }
        if has_table(&tx, "messages")? {
            let has_status = has_column(&tx, "messages", "status")?;
            let sql = if has_status {
                "INSERT INTO messages_new (id, conversation_id, role, content, created_at_ms, status)
                 SELECT id, conversation_id, role, content, created_at_ms, COALESCE(status, 'done') FROM messages"
            } else {
                "INSERT INTO messages_new (id, conversation_id, role, content, created_at_ms, status)
                 SELECT id, conversation_id, role, content, created_at_ms, 'done' FROM messages"
            };
            tx.execute(sql, []).map_err(storage)?;
        }
        tx.execute_batch(
            "DROP TABLE IF EXISTS messages;
             DROP TABLE conversations;
             ALTER TABLE conversations_new RENAME TO conversations;
             ALTER TABLE messages_new RENAME TO messages;
             CREATE INDEX idx_conversations_workspace ON conversations(workspace_id, updated_at_ms DESC);
             CREATE INDEX idx_messages_conversation ON messages(conversation_id, created_at_ms);
             PRAGMA user_version=3;",
        )
        .map_err(storage)?;
        tx.commit().map_err(storage)?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")
            .map_err(storage)?;
    }
    if !has_column(conn, "messages", "status")? {
        conn.execute(
            "ALTER TABLE messages ADD COLUMN status TEXT NOT NULL DEFAULT 'done'",
            [],
        )
        .map_err(storage)?;
        conn.pragma_update(None, "user_version", 3)
            .map_err(storage)?;
    }
    conn.execute(
        "UPDATE workspaces SET is_default = CASE WHEN id = (SELECT id FROM workspaces ORDER BY is_default DESC, created_at_ms ASC LIMIT 1) THEN 1 ELSE 0 END",
        [],
    )
    .map_err(storage)?;
    Ok(())
}

fn existing_default_id(conn: &Connection) -> Result<Option<String>, SessionError> {
    if !has_table(conn, "workspaces")? {
        return Ok(None);
    }
    conn.query_row(
        "SELECT id FROM workspaces WHERE is_default=1 ORDER BY created_at_ms LIMIT 1",
        [],
        |row| row.get(0),
    )
    .optional()
    .map_err(storage)
}

fn is_writable_dir(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_dir() || metadata.permissions().readonly() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o200 == 0 {
            return false;
        }
    }
    true
}

fn row_to_workspace(row: &rusqlite::Row<'_>) -> rusqlite::Result<Workspace> {
    Ok(Workspace {
        id: row.get(0)?,
        path: row.get(1)?,
        title: row.get(2)?,
        is_default: row.get::<_, i64>(3)? != 0,
        created_at_ms: row.get::<_, i64>(4)?.max(0).try_into().unwrap_or(0),
        updated_at_ms: row.get::<_, i64>(5)?.max(0).try_into().unwrap_or(0),
    })
}

fn row_to_conversation(row: &rusqlite::Row<'_>) -> rusqlite::Result<Conversation> {
    Ok(Conversation {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        title: row.get(2)?,
        created_at_ms: row.get::<_, i64>(3)?.max(0).try_into().unwrap_or(0),
        updated_at_ms: row.get::<_, i64>(4)?.max(0).try_into().unwrap_or(0),
    })
}

impl SqliteSessionStore {
    fn with_conn<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, SessionError>,
    ) -> Result<T, SessionError> {
        let guard = self
            .conn
            .lock()
            .map_err(|_| storage("session store poisoned"))?;
        let conn = guard
            .as_ref()
            .ok_or_else(|| storage("session store unavailable"))?;
        f(conn)
    }

    fn get_workspace(conn: &Connection, id: &str) -> Result<Workspace, SessionError> {
        conn.query_row(
            "SELECT id, path, title, is_default, created_at_ms, updated_at_ms FROM workspaces WHERE id=?1",
            params![id],
            row_to_workspace,
        )
        .optional()
        .map_err(storage)?
        .ok_or_else(|| SessionError::WorkspaceNotFound(id.to_owned()))
    }
}

impl WorkspaceStore for SqliteSessionStore {
    fn list_workspaces(&self) -> Result<Vec<Workspace>, SessionError> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare("SELECT id, path, title, is_default, created_at_ms, updated_at_ms FROM workspaces ORDER BY is_default DESC, created_at_ms ASC, id ASC")
                .map_err(storage)?;
            let rows = stmt.query_map([], row_to_workspace).map_err(storage)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(storage)
        })
    }

    fn create_workspace(&self, path: &str, title: &str) -> Result<Workspace, SessionError> {
        let path = path.trim();
        if path.is_empty() {
            return Err(SessionError::WorkspacePathUnavailable(
                "empty path".to_owned(),
            ));
        }
        let canonical = std::fs::canonicalize(path).map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => SessionError::WorkspacePathNotFound(path.to_owned()),
            std::io::ErrorKind::PermissionDenied => {
                SessionError::WorkspaceNotWritable(path.to_owned())
            }
            _ => SessionError::WorkspacePathUnavailable(format!("{path}: {error}")),
        })?;
        let metadata = std::fs::metadata(&canonical).map_err(storage)?;
        if !metadata.is_dir() {
            return Err(SessionError::WorkspaceNotADirectory(path.to_owned()));
        }
        if !is_writable_dir(&canonical) {
            return Err(SessionError::WorkspaceNotWritable(path.to_owned()));
        }
        let path_text = canonical.to_string_lossy().to_string();
        let title = if title.trim().is_empty() {
            canonical.file_name().map_or_else(
                || "Workspace".to_owned(),
                |n| n.to_string_lossy().to_string(),
            )
        } else {
            title.trim().to_owned()
        };
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_ms();
        self.with_conn(|conn| {
            let exists: i64 = conn
                .query_row(
                    "SELECT COUNT(1) FROM workspaces WHERE path=?1",
                    params![path_text],
                    |row| row.get(0),
                )
                .map_err(storage)?;
            if exists > 0 {
                return Err(SessionError::DuplicateWorkspacePath(path_text));
            }
            conn.execute(
                "INSERT INTO workspaces (id, path, title, is_default, created_at_ms, updated_at_ms)
                 VALUES (?1, ?2, ?3, 0, ?4, ?4)",
                params![id, path_text, title, now],
            )
            .map_err(storage)?;
            SqliteSessionStore::get_workspace(conn, &id)
        })
    }

    fn rename_workspace(&self, workspace_id: &str, title: &str) -> Result<Workspace, SessionError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(SessionError::InvalidWorkspace("empty title".to_owned()));
        }
        self.with_conn(|conn| {
            SqliteSessionStore::get_workspace(conn, workspace_id)?;
            conn.execute(
                "UPDATE workspaces SET title=?2, updated_at_ms=?3 WHERE id=?1",
                params![workspace_id, title, now_ms()],
            )
            .map_err(storage)?;
            SqliteSessionStore::get_workspace(conn, workspace_id)
        })
    }

    fn set_default_workspace(&self, workspace_id: &str) -> Result<Vec<Workspace>, SessionError> {
        self.with_conn(|conn| {
            SqliteSessionStore::get_workspace(conn, workspace_id)?;
            let tx = conn.unchecked_transaction().map_err(storage)?;
            tx.execute("UPDATE workspaces SET is_default=0", []).map_err(storage)?;
            tx.execute(
                "UPDATE workspaces SET is_default=1, updated_at_ms=?2 WHERE id=?1",
                params![workspace_id, now_ms()],
            )
            .map_err(storage)?;
            tx.commit().map_err(storage)?;
            let mut stmt = conn
                .prepare("SELECT id, path, title, is_default, created_at_ms, updated_at_ms FROM workspaces ORDER BY is_default DESC, created_at_ms ASC, id ASC")
                .map_err(storage)?;
            let rows = stmt.query_map([], row_to_workspace).map_err(storage)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(storage)
        })
    }

    fn remove_workspace(
        &self,
        workspace_id: &str,
        reassign_to: Option<&str>,
        confirm: bool,
    ) -> Result<RemoveWorkspaceResult, SessionError> {
        if !confirm {
            return Err(SessionError::ConfirmRequired(
                "workspace.remove requires confirm=true".to_owned(),
            ));
        }
        self.with_conn(|conn| {
            let workspace = SqliteSessionStore::get_workspace(conn, workspace_id)?;
            if workspace.is_default {
                return Err(SessionError::DefaultWorkspace(
                    "change default with workspace.set_default first".to_owned(),
                ));
            }
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(1) FROM conversations WHERE workspace_id=?1",
                    params![workspace_id],
                    |row| row.get(0),
                )
                .map_err(storage)?;
            let moved = u64::try_from(count).unwrap_or(0);
            if count > 0 {
                let Some(target) = reassign_to else {
                    return Err(SessionError::WorkspaceInUse(format!(
                        "{workspace_id} has {count} conversations"
                    )));
                };
                if target == workspace_id {
                    return Err(SessionError::WorkspaceReassignTargetInvalid(
                        "reassign_to must differ from removed workspace".to_owned(),
                    ));
                }
                let target_workspace = match SqliteSessionStore::get_workspace(conn, target) {
                    Ok(workspace) => workspace,
                    Err(SessionError::WorkspaceNotFound(_)) => {
                        return Err(SessionError::WorkspaceReassignTargetInvalid(format!(
                            "target not found: {target}"
                        )));
                    }
                    Err(error) => return Err(error),
                };
                if !is_writable_dir(Path::new(&target_workspace.path)) {
                    return Err(SessionError::WorkspaceReassignTargetInvalid(format!(
                        "target path unavailable: {}",
                        target_workspace.path
                    )));
                }
                let tx = conn.unchecked_transaction().map_err(storage)?;
                tx.execute(
                    "UPDATE conversations SET workspace_id=?2 WHERE workspace_id=?1",
                    params![workspace_id, target],
                )
                .map_err(storage)?;
                tx.execute("DELETE FROM workspaces WHERE id=?1", params![workspace_id])
                    .map_err(storage)?;
                tx.commit().map_err(storage)?;
            } else {
                conn.execute("DELETE FROM workspaces WHERE id=?1", params![workspace_id])
                    .map_err(storage)?;
            }
            Ok(RemoveWorkspaceResult {
                removed: true,
                moved_sessions: moved,
            })
        })
    }
}

impl SessionStore for SqliteSessionStore {
    fn create_conversation_in(
        &self,
        workspace_id: &str,
        title: &str,
    ) -> Result<Conversation, SessionError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(SessionError::Invalid("empty conversation title".to_owned()));
        }
        self.with_conn(|conn| {
            SqliteSessionStore::get_workspace(conn, workspace_id)?;
            let id = uuid::Uuid::new_v4().to_string();
            let now = now_ms();
            conn.execute(
                "INSERT INTO conversations (id, workspace_id, title, created_at_ms, updated_at_ms) VALUES (?1, ?2, ?3, ?4, ?4)",
                params![id, workspace_id, title, now],
            )
            .map_err(storage)?;
            conn.query_row(
                "SELECT id, workspace_id, title, created_at_ms, updated_at_ms FROM conversations WHERE id=?1",
                params![id],
                row_to_conversation,
            )
            .map_err(storage)
        })
    }

    fn list_conversations_in(&self, workspace_id: &str) -> Result<Vec<Conversation>, SessionError> {
        self.with_conn(|conn| {
            SqliteSessionStore::get_workspace(conn, workspace_id)?;
            let mut stmt = conn
                .prepare("SELECT id, workspace_id, title, created_at_ms, updated_at_ms FROM conversations WHERE workspace_id=?1 ORDER BY updated_at_ms DESC, id DESC")
                .map_err(storage)?;
            let rows = stmt.query_map(params![workspace_id], row_to_conversation).map_err(storage)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(storage)
        })
    }

    fn append_message(&self, message: &Message) -> Result<(), SessionError> {
        if message.id.trim().is_empty() || message.conversation_id.trim().is_empty() {
            return Err(SessionError::Invalid(
                "empty message id/conversation id".to_owned(),
            ));
        }
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO messages (id, conversation_id, role, content, created_at_ms, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![message.id, message.conversation_id, message.role, message.content, i64::try_from(message.created_at_ms).unwrap_or(i64::MAX), message.status],
            )
            .map_err(storage)?;
            conn.execute(
                "UPDATE conversations SET updated_at_ms=?2 WHERE id=?1",
                params![message.conversation_id, i64::try_from(now_ms()).unwrap_or(i64::MAX)],
            )
            .map_err(storage)?;
            Ok(())
        })
    }

    fn messages_in(
        &self,
        workspace_id: &str,
        conversation_id: &str,
    ) -> Result<Vec<Message>, SessionError> {
        self.with_conn(|conn| {
            SqliteSessionStore::get_workspace(conn, workspace_id)?;
            let owner: Option<String> = conn
                .query_row("SELECT workspace_id FROM conversations WHERE id=?1", params![conversation_id], |row| row.get(0))
                .optional()
                .map_err(storage)?;
            let owner = owner.ok_or_else(|| SessionError::NotFound(conversation_id.to_owned()))?;
            if owner != workspace_id {
                return Err(SessionError::WorkspaceMismatch {
                    conversation: conversation_id.to_owned(),
                    workspace: workspace_id.to_owned(),
                });
            }
            let mut stmt = conn
                .prepare("SELECT id, conversation_id, role, content, created_at_ms, status FROM messages WHERE conversation_id=?1 ORDER BY created_at_ms ASC, id ASC")
                .map_err(storage)?;
            let rows = stmt.query_map(params![conversation_id], |row| {
                Ok(Message {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    created_at_ms: row.get::<_, i64>(4)?.max(0).try_into().unwrap_or(0),
                    status: row.get(5)?,
                })
            })
            .map_err(storage)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(storage)
        })
    }
}
