#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_kit_wasm::validate_sql;

#[test]
fn allows_only_single_dml() {
    for sql in ["SELECT 1", "INSERT INTO t(a) VALUES (1)", "UPDATE t SET a=1 WHERE id=2", "DELETE FROM t WHERE id=1", "  select * from t  "] {
        assert!(validate_sql(sql).is_ok(), "{sql}");
    }
}

#[test]
fn rejects_ddl_attach_and_smuggling() {
    for sql in [
        "CREATE TABLE x(a)",
        "DROP TABLE x",
        "ALTER TABLE x ADD COLUMN b",
        "PRAGMA writable_schema=1",
        "ATTACH DATABASE '/tmp/x' AS y",
        "VACUUM INTO '/tmp/x'",
        "SELECT load_extension('/tmp/x')",
        "SELECT 1; DROP TABLE x",
        "SELECT 1 -- comment",
        "SELECT 1 /* c */",
        "CREATE VIRTUAL TABLE x USING fts5(a)",
        "CREATE TRIGGER t AFTER INSERT ON x BEGIN SELECT 1; END",
        "SELECT readfile('/etc/passwd')",
        "SELECT writefile('/tmp/x', 'x')",
    ] {
        assert!(validate_sql(sql).is_err(), "{sql}");
    }
}
