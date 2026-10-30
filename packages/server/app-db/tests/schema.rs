#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_app::AppMigration;
use zk_srv_app_db::{ensure_schema, table_count, MAX_TABLES};

fn migration(version: u32, sql: &str) -> AppMigration { AppMigration { version, sql: sql.to_owned() } }

#[test]
fn host_managed_schema_and_downgrade_guard() {
    let dir = std::env::temp_dir().join(format!("zk-app-db-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("a.sqlite");
    let migrations = vec![
        migration(1, "CREATE TABLE t1(id INTEGER PRIMARY KEY);"),
        migration(2, "CREATE INDEX idx_t1 ON t1(id);"),
    ];
    assert_eq!(ensure_schema(&path, 2, &migrations).unwrap(), 2);
    assert_eq!(table_count(&path).unwrap(), 1);
    assert!(ensure_schema(&path, 1, &migrations).is_err(), "downgrade must fail");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn guest_ddl_and_forbidden_migrations_rejected() {
    let dir = std::env::temp_dir().join(format!("zk-app-db-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("b.sqlite");
    for (version, sql) in [
        (1, "DROP TABLE x;"),
        (1, "ATTACH DATABASE '/tmp/x' AS y;"),
        (1, "PRAGMA writable_schema=1;"),
        (1, "CREATE TRIGGER t AFTER INSERT ON x BEGIN SELECT 1; END;"),
    ] {
        assert!(ensure_schema(&path, 1, &[migration(version, sql)]).is_err(), "{sql}");
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn table_limit_enforced() {
    let dir = std::env::temp_dir().join(format!("zk-app-db-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("c.sqlite");
    let mut sql = String::new();
    for index in 0..=MAX_TABLES { sql.push_str(&format!("CREATE TABLE t{index}(id INTEGER);")); }
    assert!(ensure_schema(&path, 1, &[migration(1, &sql)]).is_err());
    std::fs::remove_dir_all(dir).unwrap();
}
