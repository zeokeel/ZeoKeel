#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use std::collections::HashMap;
use zk_kit_wasm::{run, Limits, RunInput, WasmError};

#[test]
fn fuel_exhaustion_is_fuel_error() {
    let dir = std::env::temp_dir().join(format!("zk-wasm-fuel-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("db.sqlite");
    let wasm = wat::parse_str(r#"(module (func (export "_start") (loop br 0)))"#).unwrap();
    let result = run(RunInput { wasm: &wasm, db_path: &db, request: serde_json::json!({}), limits: Limits { fuel: 1_000, ..Limits::default() }, assets: &HashMap::new() });
    assert!(matches!(result, Err(WasmError::Fuel)), "{result:?}");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn memory_limit_is_memory_error() {
    let dir = std::env::temp_dir().join(format!("zk-wasm-mem-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("db.sqlite");
    let wasm = wat::parse_str(r#"(module (memory 1) (func (export "_start") (loop (memory.grow (i32.const 1)) (br 0))))"#).unwrap();
    let result = run(RunInput { wasm: &wasm, db_path: &db, request: serde_json::json!({}), limits: Limits { memory_bytes: 2 * 1024 * 1024, timeout: std::time::Duration::from_secs(2), ..Limits::default() }, assets: &HashMap::new() });
    assert!(matches!(result, Err(WasmError::Memory | WasmError::Fuel | WasmError::Timeout)), "{result:?}");
    std::fs::remove_dir_all(dir).unwrap();
}
