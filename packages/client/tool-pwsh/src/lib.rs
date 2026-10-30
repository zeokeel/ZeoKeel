//! PowerShell (platform gated).
#![forbid(unsafe_code)]
use std::process::Command;
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};
pub struct PwshTool;
#[must_use] pub fn tool() -> PwshTool { PwshTool }
impl Tool for PwshTool {
    fn spec(&self) -> ToolSpec { ToolSpec { name: "pwsh".to_owned(), description: "Execute a PowerShell command (Windows; returns TOOL_UNSUPPORTED_PLATFORM without pwsh).".to_owned(), parameters: serde_json::json!({"type":"object","properties":{"command":{"type":"string"},"cwd":{"type":"string"},"timeout_ms":{"type":"integer"}},"required":["command"],"additionalProperties":false}), mutating: true, requires_approval: true } }
    fn call(&self, _args: &str) -> Result<String, ToolError> { Err(ToolError::InvalidArgs("pwsh requires JSON".to_owned())) }
    fn call_json(&self, context: &TurnContext, args: serde_json::Value) -> Result<String, ToolError> {
        let command = args.get("command").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("command".to_owned()))?;
        if Command::new("pwsh").arg("-NoProfile").arg("-Command").arg("$PSVersionTable.PSVersion.Major").output().is_err() {
            return Err(ToolError::Denied("TOOL_UNSUPPORTED_PLATFORM: pwsh not found".to_owned()));
        }
        let root = zk_kit_tools::workspace_root(context)?;
        let cwd = match args.get("cwd").and_then(serde_json::Value::as_str) { Some(cwd) if !cwd.trim().is_empty() => zk_kit_tools::safe_path(context, cwd)?, _ => root };
        let output = Command::new("pwsh").arg("-NoProfile").arg("-Command").arg(command).current_dir(cwd).output().map_err(|error| ToolError::Io(error.to_string()))?;
        let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() { text.push_str("\n[stderr]\n"); text.push_str(&stderr); }
        if !output.status.success() { text.push_str(&format!("\n[exit code: {}]", output.status.code().unwrap_or(-1))); }
        Ok(zk_kit_tools::truncate(text))
    }
}
