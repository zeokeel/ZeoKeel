#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use std::path::PathBuf;
use zk_contract_tool::TurnContext;
use zk_kit_tools::{
    read_text, read_window, safe_path, search, truncate, workspace_root, write_text,
};

fn ctx() -> (PathBuf, TurnContext) {
    let dir = std::env::temp_dir().join(format!(
        "zk-kit-tools-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let context = TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: dir.display().to_string(),
    };
    (dir, context)
}

#[test]
fn reads_writes_and_windows() {
    let (dir, context) = ctx();
    let path = safe_path(&context, "a/b.txt").unwrap();
    write_text(&path, "one\ntwo\nthree\n").unwrap();
    assert_eq!(read_text(&path).unwrap(), "one\ntwo\nthree\n");
    assert_eq!(read_window("one\ntwo\nthree\n", 2, 1), "2\ttwo\n");
    assert_eq!(
        workspace_root(&context).unwrap(),
        dir.canonicalize().unwrap()
    );
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
    let second =
        zk_kit_tools::persistent::run(&context, "printf '%s:%s' \"$PWD\" \"$ZK_TEST\"", 5000)
            .unwrap();
    assert!(second.contains("/sub"));
    assert!(second.contains(":ok"));
    zk_kit_tools::persistent::close(&context).unwrap();
}

#[test]
fn persistent_sessions_are_independent() {
    use zk_kit_tools::persistent::ShellKind;
    let (dir, mut context) = ctx();
    context.conversation_id = "multi-session".to_owned();
    std::fs::create_dir_all(dir.join("alpha")).unwrap();
    std::fs::create_dir_all(dir.join("beta")).unwrap();
    let _ = zk_kit_tools::persistent::run_in_session(
        &context,
        ShellKind::Bash,
        "one",
        "cd alpha",
        5000,
    );
    let _ =
        zk_kit_tools::persistent::run_in_session(&context, ShellKind::Bash, "two", "cd beta", 5000);
    let one = zk_kit_tools::persistent::run_in_session(
        &context,
        ShellKind::Bash,
        "one",
        "printf '%s' \"$PWD\"",
        5000,
    )
    .unwrap();
    let two = zk_kit_tools::persistent::run_in_session(
        &context,
        ShellKind::Bash,
        "two",
        "printf '%s' \"$PWD\"",
        5000,
    )
    .unwrap();
    assert!(one.contains("/alpha"));
    assert!(two.contains("/beta"));
    let mut listed = zk_kit_tools::persistent::list_sessions(&context, ShellKind::Bash);
    listed.sort();
    assert_eq!(listed, vec!["one".to_owned(), "two".to_owned()]);
    assert!(zk_kit_tools::persistent::close_session(&context, ShellKind::Bash, "one").unwrap());
    assert!(!zk_kit_tools::persistent::is_session_open(
        &context,
        ShellKind::Bash,
        "one"
    ));
    let again = zk_kit_tools::persistent::run_in_session(
        &context,
        ShellKind::Bash,
        "two",
        "printf '%s' \"$PWD\"",
        5000,
    )
    .unwrap();
    assert!(again.contains("/beta"));
    zk_kit_tools::persistent::close_session(&context, ShellKind::Bash, "two").unwrap();
}

#[test]
fn persistent_session_cap_is_enforced() {
    use zk_kit_tools::persistent::ShellKind;
    let (_dir, context) = ctx();
    let mut open = Vec::new();
    for index in 0..zk_kit_tools::persistent::MAX_SESSIONS_PER_CONVERSATION {
        let session = format!("cap-{index}");
        zk_kit_tools::persistent::open_session(&context, ShellKind::Bash, &session).unwrap();
        open.push(session);
    }
    let error = zk_kit_tools::persistent::open_session(&context, ShellKind::Bash, "cap-over")
        .unwrap_err()
        .to_string();
    assert!(error.contains("session limit"), "unexpected error: {error}");
    for session in open {
        zk_kit_tools::persistent::close_session(&context, ShellKind::Bash, &session).unwrap();
    }
}

#[test]
fn pwsh_session_is_honest_about_availability() {
    use zk_kit_tools::persistent::ShellKind;
    let (_dir, context) = ctx();
    match zk_kit_tools::persistent::open_session(&context, ShellKind::Pwsh, "pw") {
        Ok(_) => {
            let out = zk_kit_tools::persistent::run_in_session(
                &context,
                ShellKind::Pwsh,
                "pw",
                "Write-Output 'zk-pwsh-ok'",
                10_000,
            )
            .unwrap();
            assert!(out.contains("zk-pwsh-ok"));
            zk_kit_tools::persistent::close_session(&context, ShellKind::Pwsh, "pw").unwrap();
        }
        Err(error) => {
            let message = error.to_string();
            // The contract is "honest about availability": a missing pwsh must report
            // TOOL_UNSUPPORTED_PLATFORM, while a hosted runner that ships a slow/broken
            // pwsh (readiness marker not printed within the 10s window) must surface the
            // timeout instead of pretending the session works.
            assert!(
                message.contains("TOOL_UNSUPPORTED_PLATFORM") || message.contains("timed out"),
                "unexpected pwsh error: {message}"
            );
        }
    }
}

#[test]
fn glob_and_grep_find_workspace_files() {
    let (_dir, context) = ctx();
    write_text(
        &safe_path(&context, "src/main.rs").unwrap(),
        "fn main() {}\nlet secret = 1;\n",
    )
    .unwrap();
    let globbed = zk_kit_tools::glob_files(&context, "**/*.rs", None).unwrap();
    assert!(globbed.contains("src/main.rs"));
    let grepped = zk_kit_tools::grep_files(&context, "secret", None, Some("*.rs")).unwrap();
    assert!(grepped.contains("src/main.rs:2"));
}
