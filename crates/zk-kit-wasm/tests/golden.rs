#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only
use std::collections::HashMap;
use std::path::PathBuf;
use zk_kit_wasm::{run, Limits, RunInput};

fn root() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../experiments/wasm-poc") }

#[test]
fn go_and_rust_guests_share_frame_semantics() {
    if !root().join("guest-go/guest-go.wasm").exists() || !root().join("guest-rust/target/wasm32-wasip1/release/zk-guest-rust.wasm").exists() {
        eprintln!("skipping golden: run experiments/wasm-poc/build.sh first");
        return;
    }
    let dir = std::env::temp_dir().join(format!("zk-kit-wasm-golden-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("app.sqlite");
    let connection = rusqlite::Connection::open(&db).unwrap();
    connection.execute_batch("CREATE TABLE items(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL);").unwrap();
    drop(connection);
    let guests = [
        ("go", root().join("guest-go/guest-go.wasm")),
        ("rust", root().join("guest-rust/target/wasm32-wasip1/release/zk-guest-rust.wasm")),
    ];
    for (name, path) in &guests {
        let wasm = std::fs::read(path).unwrap();
        let request = serde_json::json!({"abi":"zeokeel-app/1","body":{"op":"add","name":name}});
        let output = run(RunInput { wasm: &wasm, db_path: &db, request, limits: Limits::default(), assets: &HashMap::new(), db_quota_bytes: 0 }).unwrap();
        assert_eq!(output.response["status"], serde_json::Value::from(200), "{name}");
    }
    let go_wasm = std::fs::read(root().join("guest-go/guest-go.wasm")).unwrap();
    let output = run(RunInput { wasm: &go_wasm, db_path: &db, request: serde_json::json!({"abi":"zeokeel-app/1","body":{"op":"list"}}), limits: Limits::default(), assets: &HashMap::new(), db_quota_bytes: 0 }).unwrap();
    let names: Vec<String> = output.response["body"]["rows"].as_array().unwrap().iter().filter_map(|row| row["name"].as_str().map(str::to_owned)).collect();
    assert_eq!(names, vec!["go".to_owned(), "rust".to_owned()]);
    let _ = std::fs::remove_dir_all(dir);
}
