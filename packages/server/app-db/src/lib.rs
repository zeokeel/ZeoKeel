//! Per-app SQLite lifecycle (`srv.app.db`).
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use rusqlite::Connection;
use zk_contract_app::AppMigration;

/// Max tables per app DB.
pub const MAX_TABLES: i64 = 16;
/// Max DB size (100 MiB).
pub const MAX_BYTES: u64 = 100 * 1024 * 1024;

/// Deterministic DB path; callers never accept a path from the guest.
#[must_use]
pub fn db_path(root: &Path, app_id: &str) -> PathBuf { root.join(format!("{app_id}.sqlite")) }

/// Open/create the app DB and enforce host-managed migrations.
pub fn ensure_schema(path: &Path, target_version: u32, migrations: &[AppMigration]) -> Result<u32, String> {
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;").map_err(|error| error.to_string())?;
    let current: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0)).map_err(|error| error.to_string())?;
    let current = u32::try_from(current).map_err(|_| "bad schema version".to_owned())?;
    if target_version < current { return Err("DB_SCHEMA_DOWNGRADE".to_owned()); }
    for migration in migrations {
        if migration.version <= current { continue; }
        if migration.version > target_version { return Err("DB_SCHEMA_TARGET".to_owned()); }
        validate_ddl(&migration.sql)?;
        connection.execute_batch(&format!("BEGIN; {} PRAGMA user_version={}; COMMIT;", migration.sql, migration.version)).map_err(|error| error.to_string())?;
    }
    let tables: i64 = connection.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'", [], |row| row.get(0)).map_err(|error| error.to_string())?;
    if tables > MAX_TABLES { return Err("APP_DB_TABLES".to_owned()); }
    Ok(target_version.max(current))
}

fn validate_ddl(sql: &str) -> Result<(), String> {
    let upper = sql.to_ascii_uppercase();
    if sql.len() > 65_536 { return Err("DB_SCHEMA_SIZE".to_owned()); }
    for token in ["ATTACH", "DETACH", "PRAGMA", "VACUUM", "TRIGGER", "VIRTUAL", "DROP", "ALTER", "LOAD_EXTENSION", "READFILE", "WRITEFILE"] {
        if upper.contains(token) { return Err(format!("DB_SCHEMA_FORBIDDEN:{token}")); }
    }
    for statement in sql.split(';').map(str::trim).filter(|value| !value.is_empty()) {
        if !statement.to_ascii_uppercase().starts_with("CREATE TABLE") && !statement.to_ascii_uppercase().starts_with("CREATE INDEX") && !statement.to_ascii_uppercase().starts_with("CREATE UNIQUE INDEX") {
            return Err("DB_SCHEMA_FORBIDDEN:statement".to_owned());
        }
    }
    Ok(())
}

/// Enforce file quota.
pub fn enforce_quota(path: &Path, quota: u64) -> Result<u64, String> {
    let size = std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
    if size > quota { Err("APP_DB_QUOTA".to_owned()) } else { Ok(size) }
}

/// Count app tables (diagnostics).
pub fn table_count(path: &Path) -> Result<i64, String> {
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    connection.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'", [], |row| row.get(0)).map_err(|error| error.to_string())
}
