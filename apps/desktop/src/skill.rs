//! `/<skill>` host: user-invocable candidates + composer Instructions content.
//!
//! Skill discovery/reading is the single implementation in `zk-tool-skill`; the
//! host only picks the edition-specific root, verifies workspace/conversation
//! ownership (C18) and maps the closed `skill/*` errors to IPC results. The
//! generated `zk-composed` facade re-exports the package (C4: no direct import).
//!
//! - personal: `<workspace>/.agents/skills` (frontmatter `user-invocable != false`);
//! - enterprise: `$ZEOKEEL_HOME/skills/<name>/<version>` installed skills only
//!   (uninstalled names/versions are `skill/unavailable`; content is never read).
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use zk_composed::zk_tool_skill::{self, SkillEntry};
use zk_contract_ipc::results::{SkillEntryView, SkillListResult, SkillReadResult};

use crate::ipc::AppState;

/// List user-invocable skill candidates for one edition.
///
/// `installed_root` is only consulted for `enterprise`; `None` means the home
/// directory is unavailable (honest `skill/unavailable`, never a fake list).
///
/// # Errors
/// `skill/*` code strings and `skill/invalid` for an unknown edition.
pub fn list_for(
    edition: &str,
    workspace_root: &Path,
    installed_root: Option<&Path>,
) -> Result<SkillListResult, String> {
    let entries = match edition {
        "personal" => zk_tool_skill::list_workspace(&workspace_root.to_string_lossy())
            .map_err(|error| error.to_string())?,
        "enterprise" => {
            let root = installed_root
                .ok_or_else(|| "skill/unavailable: home directory unavailable".to_owned())?;
            zk_tool_skill::list_installed(root).map_err(|error| error.to_string())?
        }
        other => return Err(format!("skill/invalid: unknown edition {other}")),
    };
    Ok(SkillListResult {
        skills: entries
            .into_iter()
            .filter(|entry| entry.user_invocable)
            .map(view_of)
            .collect(),
    })
}

/// Read one skill's Instructions body (≤8 KiB) for the active edition.
///
/// # Errors
/// `skill/not-found`, `skill/unavailable`, `skill/too-large`, `skill/invalid`,
/// `skill/failed` (all via the single `zk-tool-skill` implementation).
pub fn read_for(
    edition: &str,
    workspace_root: &Path,
    installed_root: Option<&Path>,
    name: &str,
    version: Option<&str>,
) -> Result<SkillReadResult, String> {
    let content = match edition {
        "personal" => zk_tool_skill::read_workspace(&workspace_root.to_string_lossy(), name),
        "enterprise" => {
            let root = installed_root
                .ok_or_else(|| "skill/unavailable: home directory unavailable".to_owned())?;
            zk_tool_skill::read_installed(root, name, version)
        }
        other => return Err(format!("skill/invalid: unknown edition {other}")),
    }
    .map_err(|error| error.to_string())?;
    Ok(SkillReadResult {
        name: content.name,
        version: content.version,
        content: content.content,
        source: content.source.as_str().to_owned(),
    })
}

fn view_of(entry: SkillEntry) -> SkillEntryView {
    SkillEntryView {
        name: entry.name,
        description: entry.description,
        user_invocable: entry.user_invocable,
        version: entry.version,
        source: entry.source.as_str().to_owned(),
    }
}

fn installed_root() -> Option<PathBuf> {
    zk_tool_skill::installed_root().ok()
}

/// `skill.list` host entry (`/<skill>` candidates; current workspace only).
///
/// # Errors
/// `workspace/not-found`, `workspace/path-unavailable`,
/// `session/workspace-mismatch` (optional conversation) and `skill/*`.
pub fn skill_list(
    state: &Arc<AppState>,
    workspace_id: &str,
    conversation_id: Option<&str>,
) -> Result<SkillListResult, String> {
    if let Some(conversation_id) = conversation_id {
        crate::ipc::verify_owner(&*state.runtime().sessions(), workspace_id, conversation_id)?;
    }
    let root = crate::ipc::workspace_root_of(state, workspace_id)?;
    list_for(zk_composed::EDITION, &root, installed_root().as_deref())
}

/// `skill.read` host entry (Instructions content for the next message).
///
/// # Errors
/// `workspace/not-found`, `session/workspace-mismatch` and `skill/*`.
pub fn skill_read(
    state: &Arc<AppState>,
    workspace_id: &str,
    conversation_id: &str,
    name: &str,
    version: Option<&str>,
) -> Result<SkillReadResult, String> {
    crate::ipc::verify_owner(&*state.runtime().sessions(), workspace_id, conversation_id)?;
    let root = crate::ipc::workspace_root_of(state, workspace_id)?;
    read_for(
        zk_composed::EDITION,
        &root,
        installed_root().as_deref(),
        name,
        version,
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)] // test-only
mod tests {
    use super::{list_for, read_for};
    use std::path::{Path, PathBuf};

    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("zk-skill-host-{tag}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_workspace_skill(workspace: &Path, name: &str, body: &str) {
        let dir = workspace.join(".agents/skills").join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), body).unwrap();
    }

    fn write_installed(root: &Path, name: &str, version: &str, body: &str) {
        let dir = root.join(name).join(version);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), body).unwrap();
    }

    #[test]
    fn personal_lists_user_invocable_workspace_skills_only() {
        let workspace = temp_dir("personal");
        write_workspace_skill(
            &workspace,
            "demo",
            "---\nname: demo\ndescription: demo skill\n---\nbody\n",
        );
        write_workspace_skill(
            &workspace,
            "hidden",
            "---\nuser-invocable: false\n---\nbody\n",
        );
        // An installed skill is not visible in personal mode (no cross-source leak).
        let installed = temp_dir("personal-installed");
        write_installed(&installed, "store-only", "1.0.0", "body\n");
        let result = list_for("personal", &workspace, Some(installed.as_path())).unwrap();
        assert_eq!(result.skills.len(), 1);
        let demo = result.skills.first().unwrap();
        assert_eq!(demo.name, "demo");
        assert_eq!(demo.description, "demo skill");
        assert_eq!(demo.source, "workspace");
        assert_eq!(demo.version, None);
        let read = read_for("personal", &workspace, None, "demo", None).unwrap();
        assert_eq!(
            read.content,
            "---\nname: demo\ndescription: demo skill\n---\nbody\n"
        );
        assert_eq!(read.source, "workspace");
        // Unknown/personal-installed names are not found (no uninstalled read).
        let missing = read_for("personal", &workspace, None, "store-only", None).unwrap_err();
        assert!(missing.contains("skill/not-found"), "{missing}");
        std::fs::remove_dir_all(&workspace).unwrap();
        std::fs::remove_dir_all(&installed).unwrap();
    }

    #[test]
    fn enterprise_lists_only_installed_and_selects_latest_version() {
        let workspace = temp_dir("enterprise-ws");
        // Workspace skills never leak into the enterprise source.
        write_workspace_skill(&workspace, "workspace-only", "---\n---\nbody\n");
        let installed = temp_dir("enterprise-installed");
        write_installed(
            &installed,
            "demo",
            "1.0.0",
            "---\ndescription: old\n---\nold\n",
        );
        write_installed(
            &installed,
            "demo",
            "1.0.10",
            "---\ndescription: newest\n---\nnewest\n",
        );
        write_installed(
            &installed,
            "hidden",
            "1.0.0",
            "---\nuser-invocable: false\n---\n",
        );
        let result = list_for("enterprise", &workspace, Some(installed.as_path())).unwrap();
        assert_eq!(result.skills.len(), 1);
        let listed = result.skills.first().unwrap();
        assert_eq!(listed.name, "demo");
        assert_eq!(listed.description, "newest");
        assert_eq!(listed.version.as_deref(), Some("1.0.10"));
        assert_eq!(listed.source, "installed");
        let latest = read_for(
            "enterprise",
            &workspace,
            Some(installed.as_path()),
            "demo",
            None,
        )
        .unwrap();
        assert_eq!(latest.version.as_deref(), Some("1.0.10"));
        let pinned = read_for(
            "enterprise",
            &workspace,
            Some(installed.as_path()),
            "demo",
            Some("1.0.0"),
        )
        .unwrap();
        assert_eq!(pinned.content, "---\ndescription: old\n---\nold\n");
        let uninstalled = read_for(
            "enterprise",
            &workspace,
            Some(installed.as_path()),
            "demo",
            Some("9.9.9"),
        )
        .unwrap_err();
        assert!(uninstalled.contains("skill/unavailable"), "{uninstalled}");
        let workspace_only = read_for(
            "enterprise",
            &workspace,
            Some(installed.as_path()),
            "workspace-only",
            None,
        )
        .unwrap_err();
        assert!(
            workspace_only.contains("skill/not-found"),
            "{workspace_only}"
        );
        // Home directory unavailable → honest unavailable, never a fake list.
        let no_home = list_for("enterprise", &workspace, None).unwrap_err();
        assert!(no_home.contains("skill/unavailable"), "{no_home}");
        std::fs::remove_dir_all(&workspace).unwrap();
        std::fs::remove_dir_all(&installed).unwrap();
    }

    #[test]
    fn unknown_edition_is_rejected() {
        let workspace = temp_dir("edition");
        let error = list_for("oem-x", &workspace, None).unwrap_err();
        assert!(error.contains("skill/invalid"), "{error}");
        std::fs::remove_dir_all(&workspace).unwrap();
    }
}
