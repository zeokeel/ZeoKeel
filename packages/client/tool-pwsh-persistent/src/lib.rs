//! `pwsh_persistent` tool: named persistent PowerShell 7 sessions (implementation in `zk-kit-tools`).
//!
//! Platform gating is honest: `open`/`send` spawn `pwsh`; when the binary is missing the shared
//! implementation returns `TOOL_UNSUPPORTED_PLATFORM` instead of pretending success.
#![forbid(unsafe_code)]
use zk_kit_tools::persistent::{SessionTool, ShellKind};

/// Construct the `pwsh_persistent` tool (PowerShell 7 sessions, multi-session per conversation).
#[must_use]
pub fn tool() -> SessionTool {
    SessionTool::new(ShellKind::Pwsh)
}
