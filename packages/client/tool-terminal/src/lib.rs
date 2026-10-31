//! `terminal` tool: named persistent bash sessions (implementation in `zk-kit-tools`).
#![forbid(unsafe_code)]
use zk_kit_tools::persistent::{SessionTool, ShellKind};

/// Construct the `terminal` tool (bash sessions, multi-session per conversation).
#[must_use]
pub fn tool() -> SessionTool {
    SessionTool::new(ShellKind::Bash)
}
