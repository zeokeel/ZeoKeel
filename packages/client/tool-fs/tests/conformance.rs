//! Conformance + behavior tests for `zk-tool-fs` (role `tool.fs`).
#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use zk_contract_tool::{Tool, ToolError};
use zk_tool_fs::{tool, FsTool};

/// Unique temporary directory, removed on drop (no `tempfile` dependency).
struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(tag: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!(
            "zk-tool-fs-{}-{tag}-{nanos}-{seq}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn write(&self, relative: &str, contents: &[u8]) {
        std::fs::write(self.path.join(relative), contents).unwrap();
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn fs_tool(root: &TempRoot) -> FsTool {
    FsTool::new(root.path().to_path_buf())
}

#[test]
fn spec_is_fs_read() {
    let root = TempRoot::new("spec");
    let spec = fs_tool(&root).spec();
    assert_eq!(spec.name, "fs.read");
    assert!(!spec.description.is_empty());
}

#[test]
fn reads_file_inside_root() {
    let root = TempRoot::new("read");
    root.write(
        "a.txt",
        "Hello ZeoKeel!\n这是一个最小 demo 文件。\n".as_bytes(),
    );

    // Object-safe through the contract trait, and argument whitespace tolerated.
    let tool: Arc<dyn Tool> = Arc::new(fs_tool(&root));
    let contents = tool.call("  a.txt  ").unwrap();
    assert_eq!(contents, "Hello ZeoKeel!\n这是一个最小 demo 文件。\n");
}

#[test]
fn denies_parent_directory_traversal() {
    let root = TempRoot::new("parent");
    root.write("a.txt", b"inside");

    assert!(matches!(
        fs_tool(&root).call("../x"),
        Err(ToolError::Denied(_))
    ));
    // `..` is rejected even when it would resolve back inside the root.
    assert!(matches!(
        fs_tool(&root).call("sub/../a.txt"),
        Err(ToolError::Denied(_))
    ));
}

#[test]
fn denies_absolute_paths() {
    let root = TempRoot::new("absolute");
    root.write("a.txt", b"inside");
    let absolute = root.path().join("a.txt");

    assert!(matches!(
        fs_tool(&root).call(&absolute.to_string_lossy()),
        Err(ToolError::Denied(_))
    ));
}

#[test]
fn missing_file_is_io_error() {
    let root = TempRoot::new("missing");
    assert!(matches!(
        fs_tool(&root).call("missing.txt"),
        Err(ToolError::Io(_))
    ));
}

#[test]
fn empty_argument_is_invalid() {
    let root = TempRoot::new("empty");
    assert!(matches!(
        fs_tool(&root).call("   "),
        Err(ToolError::InvalidArgs(_))
    ));
}

#[test]
fn rejects_files_above_64_kib() {
    let root = TempRoot::new("large");
    root.write("big.txt", &vec![b'a'; 70 * 1024]);
    assert!(matches!(
        fs_tool(&root).call("big.txt"),
        Err(ToolError::InvalidArgs(_))
    ));
}

#[test]
fn rejects_non_utf8_and_directories() {
    let root = TempRoot::new("shape");
    root.write("binary.bin", &[0xff, 0xfe, 0xfd]);
    std::fs::create_dir_all(root.path().join("sub")).unwrap();

    assert!(matches!(
        fs_tool(&root).call("binary.bin"),
        Err(ToolError::InvalidArgs(_))
    ));
    assert!(matches!(
        fs_tool(&root).call("sub"),
        Err(ToolError::InvalidArgs(_))
    ));
}

#[cfg(unix)]
#[test]
fn denies_symlink_escape() {
    let root = TempRoot::new("symlink");
    let outside = TempRoot::new("symlink-outside");
    outside.write("secret.txt", b"outside");

    std::os::unix::fs::symlink(outside.path().join("secret.txt"), root.path().join("link"))
        .unwrap();

    assert!(matches!(
        fs_tool(&root).call("link"),
        Err(ToolError::Denied(_))
    ));
}

#[test]
fn default_factory_is_fs_read() {
    assert_eq!(tool().spec().name, "fs.read");
}

#[test]
fn call_in_uses_workspace_root_not_tool_root() {
    let tool_root = TempRoot::new("tool-root");
    let workspace = TempRoot::new("workspace");
    workspace.write("notes.txt", b"from workspace");
    let ctx = zk_contract_tool::TurnContext {
        conversation_id: "c1".to_owned(),
        workspace_id: "w1".to_owned(),
        workspace_root: workspace.path().to_string_lossy().to_string(),
    };
    let result = fs_tool(&tool_root).call_in(&ctx, "notes.txt").unwrap();
    assert_eq!(result, "from workspace");
    assert!(fs_tool(&tool_root).call_in(&ctx, "../escape").is_err());
    let empty = zk_contract_tool::TurnContext { workspace_root: String::new(), ..ctx };
    assert!(fs_tool(&tool_root).call_in(&empty, "notes.txt").is_err());
}
