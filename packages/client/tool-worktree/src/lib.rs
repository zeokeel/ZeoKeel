//! Git worktree helper (argv-only, workspace-confined).
#![forbid(unsafe_code)]
use std::process::Command;
use zk_contract_tool::{ApprovalMetadata, ApprovalRisk, Tool, ToolError, ToolSpec, TurnContext};

pub struct WorktreeTool;
#[must_use]
pub fn tool() -> WorktreeTool {
    WorktreeTool
}

fn branch_allowed(branch: &str) -> bool {
    !branch.is_empty()
        && branch.len() <= 100
        && branch
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '/' | '-'))
        && !branch.contains("..")
}

fn run(root: &std::path::Path, args: &[&str]) -> Result<String, ToolError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| ToolError::Io(error.to_string()))?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        text.push_str("\n[stderr]\n");
        text.push_str(&stderr);
    }
    if !output.status.success() {
        text.push_str(&format!(
            "\n[exit code: {}]",
            output.status.code().unwrap_or(-1)
        ));
    }
    Ok(zk_kit_tools::truncate(text))
}

impl Tool for WorktreeTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "create_worktree".to_owned(),
            description:
                "Create/list/remove a git worktree inside the workspace (action + path/branch)."
                    .to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{"action":{"type":"string","enum":["create","list","remove"]},"path":{"type":"string"},"branch":{"type":"string"}},"required":["action"],"additionalProperties":false}),
            mutating: true,
            requires_approval: true,
        }
    }
    fn approval_metadata(
        &self,
        args: &serde_json::Value,
        context: &TurnContext,
    ) -> Option<ApprovalMetadata> {
        let action = args.get("action")?.as_str()?;
        let path = args.get("path").and_then(serde_json::Value::as_str);
        let target = path.map_or_else(
            || context.workspace_root.clone(),
            |relative| {
                format!(
                    "{}/{}",
                    context.workspace_root.trim_end_matches('/'),
                    relative
                )
            },
        );
        let (read_paths, write_paths) = match action {
            "list" => (vec![target.clone()], Vec::new()),
            _ => (Vec::new(), vec![target.clone()]),
        };
        Some(ApprovalMetadata {
            summary: format!("worktree {action} {target}"),
            cwd: context.workspace_root.clone(),
            read_paths,
            write_paths,
            risk: ApprovalRisk::High,
            ..ApprovalMetadata::default()
        })
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "create_worktree requires JSON".to_owned(),
        ))
    }
    fn call_json(
        &self,
        context: &TurnContext,
        args: serde_json::Value,
    ) -> Result<String, ToolError> {
        let root = zk_kit_tools::workspace_root(context)?;
        let action = args
            .get("action")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("action".to_owned()))?;
        match action {
            "list" => run(&root, &["worktree", "list"]),
            "create" => {
                let path_arg = args
                    .get("path")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| ToolError::InvalidArgs("path".to_owned()))?;
                let branch = args
                    .get("branch")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| ToolError::InvalidArgs("branch".to_owned()))?;
                if !branch_allowed(branch) {
                    return Err(ToolError::InvalidArgs("invalid branch name".to_owned()));
                }
                let path = zk_kit_tools::safe_path(context, path_arg)?;
                if path.exists() {
                    return Err(ToolError::InvalidArgs(
                        "worktree path already exists".to_owned(),
                    ));
                }
                let path_text = path.display().to_string();
                run(&root, &["worktree", "add", &path_text, branch])
            }
            "remove" => {
                let path_arg = args
                    .get("path")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| ToolError::InvalidArgs("path".to_owned()))?;
                let path = zk_kit_tools::safe_path(context, path_arg)?;
                let path_text = path.display().to_string();
                run(&root, &["worktree", "remove", &path_text])
            }
            other => Err(ToolError::InvalidArgs(format!("unknown action {other}"))),
        }
    }
}
