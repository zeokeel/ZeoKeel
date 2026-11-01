//! Workspace-scoped plan store (`.zeokeel/plans.json`, one entry per conversation).
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Plan mode of one conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanMode {
    /// Plan mode active: mutating/delegation tools are blocked.
    Active,
    /// Plan mode off (no tool filtering).
    Off,
}

/// Review status of the submitted plan.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    /// No submitted plan yet.
    #[default]
    Draft,
    /// `exit_plan_mode` submitted; waiting for local review.
    PendingReview,
    /// Locally approved (never auto-executed).
    Approved,
    /// Locally rejected with optional feedback; still in plan mode.
    Rejected,
}

/// One conversation plan entry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PlanEntry {
    /// Plan mode (`active`/`off`).
    pub mode: PlanMode,
    /// Review status.
    #[serde(default)]
    pub status: PlanStatus,
    /// Plan Markdown (empty before submission).
    #[serde(default)]
    pub plan: String,
    /// Optional local review feedback.
    #[serde(default)]
    pub feedback: Option<String>,
    /// Turn id that submitted the plan.
    #[serde(default)]
    pub through_turn: Option<String>,
    /// Last update time (ms since epoch).
    #[serde(default)]
    pub updated_at_ms: u64,
}

impl Default for PlanEntry {
    fn default() -> Self {
        Self {
            mode: PlanMode::Off,
            status: PlanStatus::Draft,
            plan: String::new(),
            feedback: None,
            through_turn: None,
            updated_at_ms: 0,
        }
    }
}

/// Whole plan store file.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PlanFile {
    /// Conversation id → entry.
    #[serde(default)]
    pub conversations: BTreeMap<String, PlanEntry>,
}

/// Plan store errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlanError {
    /// Filesystem failure.
    #[error("io: {0}")]
    Io(String),
    /// Invalid/corrupt JSON.
    #[error("corrupt plans.json: {0}")]
    Corrupt(String),
    /// File exceeds the 64 KiB contract bound.
    #[error("plan/too-large: plans.json exceeds 64 KiB")]
    TooLarge,
}

/// Current time in ms since the Unix epoch.
#[must_use]
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// `<workspace>/.zeokeel/plans.json`.
#[must_use]
pub fn plan_path(workspace_root: &str) -> PathBuf {
    Path::new(workspace_root)
        .join(".zeokeel")
        .join("plans.json")
}

/// Path shown to the model/UI (workspace-relative).
#[must_use]
pub fn relative_display(path: &Path) -> String {
    path.components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<&str>>()
        .join("/")
}

/// Load the plan store (missing file → empty store).
///
/// # Errors
/// [`PlanError`] for IO/corruption/oversize.
pub fn load(workspace_root: &str) -> Result<PlanFile, PlanError> {
    let path = plan_path(workspace_root);
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(PlanFile::default())
        }
        Err(error) => return Err(PlanError::Io(error.to_string())),
    };
    if raw.len() > 64 * 1024 {
        return Err(PlanError::TooLarge);
    }
    serde_json::from_str(&raw).map_err(|error| PlanError::Corrupt(error.to_string()))
}

/// Atomically save the plan store (`*.tmp` + rename), enforcing the 64 KiB bound.
///
/// # Errors
/// [`PlanError`] for validation/IO/serialization failures.
pub fn save(workspace_root: &str, file: &PlanFile) -> Result<(), PlanError> {
    let raw = serde_json::to_string_pretty(file)
        .map_err(|error| PlanError::Corrupt(error.to_string()))?;
    if raw.len() > 64 * 1024 {
        return Err(PlanError::TooLarge);
    }
    let path = plan_path(workspace_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| PlanError::Io(error.to_string()))?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, raw).map_err(|error| PlanError::Io(error.to_string()))?;
    std::fs::rename(&tmp, &path).map_err(|error| PlanError::Io(error.to_string()))?;
    Ok(())
}

impl PlanFile {
    /// Entry for one conversation (inserted as off/draft when missing).
    pub fn entry_mut(&mut self, conversation_id: &str) -> &mut PlanEntry {
        self.conversations
            .entry(conversation_id.to_owned())
            .or_default()
    }

    /// Enter plan mode (keeps an already submitted plan; new submissions overwrite).
    pub fn enter(&mut self, conversation_id: &str, now: u64) -> &PlanEntry {
        let entry = self.entry_mut(conversation_id);
        entry.mode = PlanMode::Active;
        entry.updated_at_ms = now;
        entry
    }

    /// Leave plan mode (cancels unsubmitted draft; approved/rejected history kept).
    pub fn leave(&mut self, conversation_id: &str, now: u64) {
        let entry = self.entry_mut(conversation_id);
        entry.mode = PlanMode::Off;
        if entry.status == PlanStatus::Draft {
            entry.plan.clear();
        }
        entry.updated_at_ms = now;
    }

    /// Record `exit_plan_mode` submission → pending_review.
    ///
    /// # Errors
    /// [`PlanError::TooLarge`] when the plan exceeds 64 KiB.
    pub fn submit(
        &mut self,
        conversation_id: &str,
        plan: String,
        now: u64,
    ) -> Result<&PlanEntry, PlanError> {
        if plan.len() > 64 * 1024 {
            return Err(PlanError::TooLarge);
        }
        let entry = self.entry_mut(conversation_id);
        entry.status = PlanStatus::PendingReview;
        entry.plan = plan;
        entry.feedback = None;
        entry.updated_at_ms = now;
        Ok(entry)
    }

    /// Local approve (does not execute the plan; leaves plan mode).
    pub fn approve(&mut self, conversation_id: &str, now: u64) {
        let entry = self.entry_mut(conversation_id);
        if entry.status == PlanStatus::PendingReview {
            entry.status = PlanStatus::Approved;
        }
        entry.mode = PlanMode::Off;
        entry.updated_at_ms = now;
    }

    /// Local reject with optional feedback; stays in plan mode for editing.
    pub fn reject(&mut self, conversation_id: &str, feedback: Option<String>, now: u64) {
        let entry = self.entry_mut(conversation_id);
        if entry.status == PlanStatus::PendingReview {
            entry.status = PlanStatus::Rejected;
            entry.feedback = feedback;
        }
        entry.mode = PlanMode::Active;
        entry.updated_at_ms = now;
    }

    /// Continue editing (back to draft, stays in plan mode).
    pub fn continue_editing(&mut self, conversation_id: &str, now: u64) {
        let entry = self.entry_mut(conversation_id);
        entry.status = PlanStatus::Draft;
        entry.feedback = None;
        entry.mode = PlanMode::Active;
        entry.updated_at_ms = now;
    }
}
