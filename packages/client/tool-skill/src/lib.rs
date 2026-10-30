//! Workspace skill loader.
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

pub struct SkillTool;
#[must_use] pub fn tool() -> SkillTool { SkillTool }
impl Tool for SkillTool {
    fn spec(&self) -> ToolSpec { ToolSpec { name: "skill".to_owned(), description: "List or read agent skills under .agents/skills (read-only).".to_owned(), parameters: serde_json::json!({"type":"object","properties":{"action":{"type":"string","enum":["list","read"]},"name":{"type":"string"}},"required":["action"],"additionalProperties":false}), mutating: false, requires_approval: false } }
    fn call(&self, _args: &str) -> Result<String, ToolError> { Err(ToolError::InvalidArgs("skill requires JSON arguments".to_owned())) }
    fn call_json(&self, context: &TurnContext, args: serde_json::Value) -> Result<String, ToolError> {
        let action = args.get("action").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("action".to_owned()))?;
        let root = zk_kit_tools::workspace_root(context)?.join(".agents/skills");
        match action {
            "list" => {
                if !root.is_dir() { return Ok("no skills".to_owned()); }
                let mut names = Vec::new();
                for entry in std::fs::read_dir(&root).map_err(|error| ToolError::Io(error.to_string()))? {
                    let entry = entry.map_err(|error| ToolError::Io(error.to_string()))?;
                    if entry.path().is_dir() { names.push(entry.file_name().to_string_lossy().into_owned()); }
                }
                names.sort();
                Ok(zk_kit_tools::truncate(names.join("\n")))
            }
            "read" => {
                let name = args.get("name").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("name".to_owned()))?;
                let path = zk_kit_tools::safe_path(context, &format!(".agents/skills/{name}/SKILL.md"))?;
                Ok(zk_kit_tools::truncate(zk_kit_tools::read_text(&path)?))
            }
            other => Err(ToolError::InvalidArgs(format!("unknown action {other}"))),
        }
    }
}
