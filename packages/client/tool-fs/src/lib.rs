//! Read-only file tool for the minimal demo.
//!
//! @capability tool.fs

#![forbid(unsafe_code)]

use std::path::PathBuf;

use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

/// Name exposed to the model and UI.
const TOOL_NAME: &str = "fs.read";

/// Hard cap on file size; larger files are rejected instead of being read.
const MAX_BYTES: u64 = 64 * 1024;

/// Environment variable used by [`FsTool::with_default_root`].
const ROOT_ENV: &str = "ZEOKEEL_DEMO_ROOT";

/// Read-only file tool confined to a single root directory.
///
/// Only relative paths that resolve inside `root` are readable. Absolute
/// paths, `..` traversal and symlink escapes are denied.
#[derive(Debug, Clone)]
pub struct FsTool {
    root: PathBuf,
}

impl FsTool {
    /// Create a tool rooted at `root` (tests inject a temporary directory).
    #[must_use]
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Create a tool rooted at `ZEOKEEL_DEMO_ROOT`, falling back to the current directory.
    #[must_use]
    pub fn with_default_root() -> Self {
        let root = std::env::var_os(ROOT_ENV)
            .map(PathBuf::from)
            .filter(|path| !path.as_os_str().is_empty())
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        Self::new(root)
    }
}

/// Default factory used by the generated glue (role `tool.fs`).
#[must_use]
pub fn tool() -> FsTool {
    FsTool::with_default_root()
}

impl Tool for FsTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: TOOL_NAME.to_owned(),
            description: "只读读取 root 内的文本文件（相对路径，≤64KiB）".to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"],"additionalProperties":false}),
            mutating: false,
            requires_approval: false,
        }
    }

    fn call(&self, args: &str) -> Result<String, ToolError> {
        let context = TurnContext { conversation_id: "legacy".to_owned(), workspace_id: "legacy".to_owned(), workspace_root: self.root.display().to_string() };
        self.call_in(&context, args)
    }

    fn call_in(&self, context: &TurnContext, args: &str) -> Result<String, ToolError> {
        let path = zk_kit_tools::safe_path(context, args)?;
        let metadata = std::fs::metadata(&path).map_err(|error| ToolError::Io(error.to_string()))?;
        if !metadata.is_file() { return Err(ToolError::InvalidArgs("不是普通文件".to_owned())); }
        if metadata.len() > MAX_BYTES { return Err(ToolError::InvalidArgs(format!("文件超过 {MAX_BYTES} 字节上限"))); }
        let bytes = std::fs::read(&path).map_err(|error| ToolError::Io(error.to_string()))?;
        String::from_utf8(bytes).map_err(|error| ToolError::InvalidArgs(format!("不是 UTF-8 文本: {error}")))
    }

    fn call_json(&self, context: &TurnContext, args: serde_json::Value) -> Result<String, ToolError> {
        let path = args.get("path").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("path".to_owned()))?;
        self.call_in(context, path)
    }
}

