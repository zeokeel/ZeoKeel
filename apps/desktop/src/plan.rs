//! Plan mode (`/plan`) host state: `.zeokeel/plans.json` read/write + review IPC.
//!
//! The plan file itself is owned by `zk-tool-plan::plans` (one implementation);
//! plan-mode tool filtering happens in agent-rig for every turn.
#![forbid(unsafe_code)]

use std::sync::Arc;

use zk_composed::zk_tool_plan::plans::{self, PlanEntry, PlanError, PlanMode, PlanStatus};
use zk_contract_ipc::results::{PlanGetResult, PlanUpdateResult, PlanView};

use crate::ipc::AppState;

/// Maximum review feedback bytes (kept small; plan file bound is 64 KiB).
pub const MAX_FEEDBACK_BYTES: usize = 4 * 1024;

fn workspace_root(state: &Arc<AppState>, workspace_id: &str) -> Result<String, String> {
    let workspaces = state
        .runtime()
        .sessions()
        .list_workspaces()
        .map_err(|error| error.to_string())?;
    let Some(workspace) = workspaces
        .into_iter()
        .find(|workspace| workspace.id == workspace_id)
    else {
        return Err("workspace/not-found".to_owned());
    };
    std::fs::canonicalize(&workspace.path)
        .map(|root| root.to_string_lossy().into_owned())
        .map_err(|error| format!("workspace/path-unavailable: {error}"))
}

fn verify_owner(
    state: &Arc<AppState>,
    workspace_id: &str,
    conversation_id: &str,
) -> Result<(), String> {
    let conversations = state
        .runtime()
        .sessions()
        .list_conversations_in(workspace_id)
        .map_err(|error| error.to_string())?;
    if conversations
        .iter()
        .any(|conversation| conversation.id == conversation_id)
    {
        Ok(())
    } else {
        Err("session/workspace-mismatch".to_owned())
    }
}

fn plan_error(error: PlanError) -> String {
    match error {
        PlanError::TooLarge => "plan/too-large: plans.json exceeds 64 KiB".to_owned(),
        other => format!("plan/failed: {other}"),
    }
}

fn view_of(entry: &PlanEntry) -> PlanView {
    PlanView {
        mode: match entry.mode {
            PlanMode::Active => "active".to_owned(),
            PlanMode::Off => "off".to_owned(),
        },
        status: match entry.status {
            PlanStatus::Draft => "draft",
            PlanStatus::PendingReview => "pending_review",
            PlanStatus::Approved => "approved",
            PlanStatus::Rejected => "rejected",
        }
        .to_owned(),
        plan: entry.plan.clone(),
        feedback: entry.feedback.clone(),
        through_turn: entry.through_turn.clone(),
    }
}

fn off_view() -> PlanView {
    PlanView {
        mode: "off".to_owned(),
        status: "draft".to_owned(),
        plan: String::new(),
        feedback: None,
        through_turn: None,
    }
}

/// Read plan state (`plan.get`).
pub fn plan_get(
    state: &Arc<AppState>,
    workspace_id: &str,
    conversation_id: &str,
) -> Result<PlanGetResult, String> {
    verify_owner(state, workspace_id, conversation_id)?;
    let root = workspace_root(state, workspace_id)?;
    let file = plans::load(&root).map_err(plan_error)?;
    Ok(PlanGetResult {
        plan: file
            .conversations
            .get(conversation_id)
            .map_or_else(off_view, view_of),
    })
}

/// Apply one local review/toggle action (`plan.update`; never executes the plan).
pub fn plan_update(
    state: &Arc<AppState>,
    workspace_id: &str,
    conversation_id: &str,
    action: &str,
    feedback: Option<String>,
) -> Result<PlanUpdateResult, String> {
    verify_owner(state, workspace_id, conversation_id)?;
    let root = workspace_root(state, workspace_id)?;
    let mut file = plans::load(&root).map_err(plan_error)?;
    let now = plans::now_ms();
    match action {
        "enter" => {
            file.enter(conversation_id, now);
        }
        "off" => {
            file.leave(conversation_id, now);
        }
        "approve" => {
            let pending = file
                .conversations
                .get(conversation_id)
                .is_some_and(|entry| entry.status == PlanStatus::PendingReview);
            if !pending {
                return Err("plan/not-active: no pending plan to approve".to_owned());
            }
            file.approve(conversation_id, now);
        }
        "reject" => {
            let pending = file
                .conversations
                .get(conversation_id)
                .is_some_and(|entry| entry.status == PlanStatus::PendingReview);
            if !pending {
                return Err("plan/not-active: no pending plan to reject".to_owned());
            }
            if let Some(text) = feedback.as_ref() {
                if text.len() > MAX_FEEDBACK_BYTES {
                    return Err("plan/too-large: feedback exceeds 4 KiB".to_owned());
                }
            }
            file.reject(conversation_id, feedback, now);
        }
        "continue" => {
            file.continue_editing(conversation_id, now);
        }
        other => return Err(format!("plan/invalid: unknown action {other}")),
    }
    plans::save(&root, &file).map_err(plan_error)?;
    let plan = file
        .conversations
        .get(conversation_id)
        .map_or_else(off_view, view_of);
    Ok(PlanUpdateResult { plan })
}

/// Whether plan mode is active for one conversation (turn-start tool filtering).
#[must_use]
pub fn plan_mode_active(
    state: &Arc<AppState>,
    workspace_root: &str,
    conversation_id: &str,
) -> bool {
    let _ = state;
    plans::load(workspace_root)
        .ok()
        .and_then(|file| {
            file.conversations
                .get(conversation_id)
                .map(|entry| entry.mode)
        })
        .is_some_and(|mode| mode == PlanMode::Active)
}
