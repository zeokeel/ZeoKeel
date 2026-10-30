#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use std::path::PathBuf;
use zk_contract_tool::TurnContext;
use zk_kit_tools::{read_text, read_window, safe_path, search, truncate, write_text, workspace_root};

fn ctx() -> (PathBuf, TurnContext) {
    let dir = std::env::temp_dir().join(format!("zk-kit-tools-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&dir).unwrap();
    let context = TurnContext { conversation_id: "c".to_owned(), workspace_id: "w".to_owned(), workspace_root: dir.display().to_string() };
    (dir, context)
}

#[test]
fn reads_writes_and_windows() {
    let (dir, context) = ctx();
    let path = safe_path(&context, "a/b.txt").unwrap();
    write_text(&path, "one\ntwo\nthree\n").unwrap();
    assert_eq!(read_text(&path).unwrap(), "one\ntwo\nthree\n");
    assert_eq!(read_window("one\ntwo\nthree\n", 2, 1), "2\ttwo\n");
    assert_eq!(workspace_root(&context).unwrap(), dir.canonicalize().unwrap());
}

#[test]
fn rejects_traversal_and_absolute() {
    let (_dir, context) = ctx();
    assert!(safe_path(&context, "../escape.txt").is_err());
    assert!(safe_path(&context, "/etc/passwd").is_err());
    assert!(safe_path(&context, "a/../../x").is_err());
}

#[test]
fn searches_names_and_content() {
    let (_dir, context) = ctx();
    write_text(&safe_path(&context, "hello.txt").unwrap(), "alpha\nbeta\n").unwrap();
    let result = search(&context, "beta", None).unwrap();
    assert!(result.contains("hello.txt:2"));
}

#[test]
fn truncates_with_marker() {
    let long = "x".repeat(zk_kit_tools::MAX_OUTPUT_BYTES + 10);
    let out = truncate(long);
    assert!(out.ends_with("[truncated]"));
}

#[test]
fn bash_runs_in_workspace() {
    let (_dir, context) = ctx();
    let out = zk_kit_tools::run_bash(&context, "printf hi", None, Some(5000)).unwrap();
    assert_eq!(out, "hi");
}

#[test]
fn persistent_shell_keeps_state() {
    let (dir, context) = ctx();
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    let first = zk_kit_tools::persistent::run(&context, "cd sub; export ZK_TEST=ok", 5000).unwrap();
    let _ = first;
    let second = zk_kit_tools::persistent::run(&context, "printf '%s:%s' \"$PWD\" \"$ZK_TEST\"", 5000).unwrap();
    assert!(second.contains("/sub"));
    assert!(second.contains(":ok"));
    zk_kit_tools::persistent::close_all();
}

#[test]
fn glob_and_grep_find_workspace_files() {
    let (_dir, context) = ctx();
    write_text(&safe_path(&context, "src/main.rs").unwrap(), "fn main() {}\nlet secret = 1;\n").unwrap();
    let globbed = zk_kit_tools::glob_files(&context, "**/*.rs", None).unwrap();
    assert!(globbed.contains("src/main.rs"));
    let grepped = zk_kit_tools::grep_files(&context, "secret", None, Some("*.rs")).unwrap();
    assert!(grepped.contains("src/main.rs:2"));
}
