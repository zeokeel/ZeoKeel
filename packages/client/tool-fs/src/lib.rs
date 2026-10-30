//! Read-only file tool for the minimal demo.
//!
//! @capability tool.fs

#![forbid(unsafe_code)]

use std::path::{Component, Path, PathBuf};

use zk_contract_tool::{Tool, ToolError, ToolSpec};

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
        }
    }

    fn call(&self, args: &str) -> Result<String, ToolError> {
        let relative = args.trim();
        if relative.is_empty() {
            return Err(ToolError::InvalidArgs("路径为空".to_owned()));
        }

        let requested = Path::new(relative);
        if requested.is_absolute() {
            return Err(ToolError::Denied("不允许绝对路径".to_owned()));
        }
        if requested
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(ToolError::Denied("不允许 .. 路径穿越".to_owned()));
        }

        let root = std::fs::canonicalize(&self.root)
            .map_err(|err| ToolError::Io(format!("root 不可用: {err}")))?;
        let candidate = root.join(requested);
        let canonical = std::fs::canonicalize(&candidate)
            .map_err(|err| ToolError::Io(format!("文件不可用: {err}")))?;
        if !canonical.starts_with(&root) {
            return Err(ToolError::Denied("路径越出 root".to_owned()));
        }

        let meta = std::fs::metadata(&canonical)
            .map_err(|err| ToolError::Io(format!("文件不可用: {err}")))?;
        if !meta.is_file() {
            return Err(ToolError::InvalidArgs("不是普通文件".to_owned()));
        }
        if meta.len() > MAX_BYTES {
            return Err(ToolError::InvalidArgs(format!(
                "文件超过 {MAX_BYTES} 字节上限"
            )));
        }

        let bytes =
            std::fs::read(&canonical).map_err(|err| ToolError::Io(format!("读取失败: {err}")))?;
        if u64::try_from(bytes.len()).is_ok_and(|len| len > MAX_BYTES) {
            return Err(ToolError::InvalidArgs(format!(
                "文件超过 {MAX_BYTES} 字节上限"
            )));
        }
        String::from_utf8(bytes)
            .map_err(|err| ToolError::InvalidArgs(format!("不是 UTF-8 文本: {err}")))
    }
}
