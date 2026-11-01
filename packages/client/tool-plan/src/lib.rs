//! Plan mode (`/plan`) submission tool + workspace plan store.
//!
//! `exit_plan_mode` is the single submission entry into `pending_review`; it never
//! executes the plan and never turns plan mode off. Policy (which tools may run in
//! plan mode) is enforced by agent-rig, not by prompts.
//!
//! @capability tool.plan
#![forbid(unsafe_code)]

pub mod plans;

use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

/// Maximum plan Markdown bytes (also bounds `.zeokeel/plans.json` size).
pub const MAX_PLAN_BYTES: usize = 64 * 1024;

/// `exit_plan_mode` tool.
pub struct ExitPlanModeTool;

/// Tool factory used by generated glue (role `tool.plan`).
#[must_use]
pub fn tool() -> ExitPlanModeTool {
    ExitPlanModeTool
}

impl Tool for ExitPlanModeTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "exit_plan_mode".to_owned(),
            description: "Submit the current plan for local review (status pending_review). \
                Use only in plan mode; this does not execute the plan and does not leave plan \
                mode. The user then approves, rejects or continues editing."
                .to_owned(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "plan": { "type": "string", "description": "Plan Markdown (≤64 KiB)." }
                },
                "required": ["plan"],
                "additionalProperties": false
            }),
            mutating: false,
            requires_approval: false,
        }
    }

    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "exit_plan_mode requires JSON".to_owned(),
        ))
    }

    fn call_json(
        &self,
        context: &TurnContext,
        args: serde_json::Value,
    ) -> Result<String, ToolError> {
        let plan = args
            .get("plan")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("plan is required".to_owned()))?;
        if plan.trim().is_empty() {
            return Err(ToolError::InvalidArgs("plan is empty".to_owned()));
        }
        if plan.len() > MAX_PLAN_BYTES {
            return Err(ToolError::InvalidArgs(
                "plan/too-large: plan exceeds 64 KiB".to_owned(),
            ));
        }
        let mut file = plans::load(&context.workspace_root)
            .map_err(|error| ToolError::Io(format!("plan/failed: {error}")))?;
        let Some(entry) = file.conversations.get(&context.conversation_id) else {
            return Err(ToolError::Denied(
                "plan/not-active: plan mode is not active".to_owned(),
            ));
        };
        if entry.mode != plans::PlanMode::Active {
            return Err(ToolError::Denied(
                "plan/not-active: plan mode is not active".to_owned(),
            ));
        }
        file.submit(&context.conversation_id, plan.to_owned(), plans::now_ms())
            .map_err(|error| ToolError::Io(format!("plan/failed: {error}")))?;
        plans::save(&context.workspace_root, &file)
            .map_err(|error| ToolError::Io(format!("plan/failed: {error}")))?;
        let path = plans::plan_path(&context.workspace_root);
        Ok(serde_json::json!({
            "status": "pending_review",
            "plan_path": plans::relative_display(&path),
        })
        .to_string())
    }
}
