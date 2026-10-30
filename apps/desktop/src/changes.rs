//! Read-only `workspace.changes` support.
//!
//! Runs `git -C <root> status --porcelain -z` with a fixed argv (never a shell)
//! and coarse file-level add/delete markers. The scan is bounded: a 5 s
//! wall-clock budget, 2 MiB of captured output and at most 500 entries. A
//! missing git binary, a non-repository workspace, a timeout or truncation
//! yields an empty/partial list plus a human-readable warning instead of an
//! error so the inspector can stay usable.
#![forbid(unsafe_code)]

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use zk_contract_ipc::results::{WorkspaceChange, WorkspaceChangesResult};

/// Wall-clock budget for one `git status` invocation.
const SCAN_TIMEOUT: Duration = Duration::from_secs(5);
/// Maximum bytes captured from git stdout (beyond this the output is truncated).
const MAX_OUTPUT_BYTES: u64 = 2 * 1024 * 1024;
/// Maximum number of changed paths returned.
const MAX_CHANGES: usize = 500;

/// Resolve one workspace path and return its bounded, read-only git status.
///
/// Coarse semantics: a non-deletion status (`A`, `??`, `AM`, `MM`, `M`, `R`,
/// `C`, …) contributes `add: 1`; any status containing `D` contributes
/// `del: 1`. No line-level diff is computed; rename/copy entries expose the
/// new path. Items are sorted by path.
#[must_use]
pub fn workspace_changes(root: &str) -> WorkspaceChangesResult {
    let root = match std::fs::canonicalize(root) {
        Ok(path) => path,
        Err(error) => {
            return warning_result(format!("workspace path unavailable: {error}"));
        }
    };
    let (bytes, output_truncated) = match run_git_status(&root) {
        Ok(result) => result,
        Err(warning) => return warning_result(warning),
    };
    let (mut items, parse_truncated) = parse_porcelain(&bytes, MAX_CHANGES);
    items.sort_unstable_by_key(|item| item.path.clone());
    let mut warnings: Vec<&str> = Vec::new();
    if output_truncated {
        warnings.push("git status output truncated at 2 MiB");
    }
    if parse_truncated {
        warnings.push("changes truncated at 500 entries");
    }
    let warning = if warnings.is_empty() {
        None
    } else {
        Some(warnings.join("; "))
    };
    WorkspaceChangesResult { items, warning }
}

fn warning_result(warning: String) -> WorkspaceChangesResult {
    WorkspaceChangesResult {
        items: Vec::new(),
        warning: Some(warning),
    }
}

/// Run `git -C <root> status --porcelain -z`; returns `(stdout, truncated)`.
/// Build the hardened `git status` command.
///
/// Repository-local config is untrusted: `core.fsmonitor` can point at an
/// executable that git would run on `status`. Disable it explicitly, disable
/// optional index locks (read-only intent), and give git a minimal environment
/// that ignores system/global config. Only repo-local config remains, and the
/// known command-execution vector is overridden with `-c`.
fn git_status_command(root: &Path) -> Command {
    let mut command = Command::new("git");
    command.env_clear();
    if let Some(path) = std::env::var_os("PATH") {
        command.env("PATH", path);
    }
    if let Some(home) = std::env::var_os("HOME") {
        command.env("HOME", home);
    }
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", null_device())
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(["-c", "core.fsmonitor=false", "--no-optional-locks", "-C"])
        .arg(root)
        .args(["status", "--porcelain", "-z"]);
    command
}

const fn null_device() -> &'static str {
    if cfg!(windows) {
        "NUL"
    } else {
        "/dev/null"
    }
}

fn run_git_status(root: &Path) -> Result<(Vec<u8>, bool), String> {
    let mut child = git_status_command(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("git unavailable: {error}"))?;

    // Drain stdout on a helper thread so a large/blocked write cannot deadlock
    // the timeout loop; capture at most MAX_OUTPUT_BYTES.
    let reader = child.stdout.take().map(|stdout| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stdout.take(MAX_OUTPUT_BYTES).read_to_end(&mut bytes);
            bytes
        })
    });

    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if started.elapsed() >= SCAN_TIMEOUT {
                    timed_out = true;
                    let _ = child.kill();
                    break child
                        .wait()
                        .map_err(|error| format!("git status failed: {error}"))?;
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => {
                let _ = child.kill();
                return Err(format!("git status failed: {error}"));
            }
        }
    };

    let bytes = reader
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default();
    if timed_out {
        return Err("git status timed out after 5s".to_owned());
    }
    let output_truncated = u64::try_from(bytes.len()).unwrap_or(u64::MAX) >= MAX_OUTPUT_BYTES;
    if !status.success() && !output_truncated {
        return Err(format!(
            "not a git repository or git status failed: {status}"
        ));
    }
    Ok((bytes, output_truncated))
}

/// Parse `git status --porcelain -z` output into coarse change entries.
///
/// Porcelain v1 `-z` records are `XY <path>\0`; rename/copy records additionally
/// contain the original path as the next NUL-terminated field, which is
/// consumed (only the new path is returned).
fn parse_porcelain(bytes: &[u8], limit: usize) -> (Vec<WorkspaceChange>, bool) {
    let mut items = Vec::new();
    let mut truncated = false;
    let mut fields = bytes
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty());
    while let Some(field) = fields.next() {
        if items.len() >= limit {
            truncated = true;
            break;
        }
        let (Some(x), Some(y), Some(separator)) = (field.first(), field.get(1), field.get(2))
        else {
            continue;
        };
        if *separator != b' ' {
            continue;
        }
        let Some(path) = field.get(3..) else {
            continue;
        };
        let status_x = char::from(*x);
        let status_y = char::from(*y);
        if matches!(status_x, 'R' | 'C') || matches!(status_y, 'R' | 'C') {
            // Consume the original path of a rename/copy record.
            let _ = fields.next();
        }
        let path = String::from_utf8_lossy(path).into_owned();
        if path.is_empty() {
            continue;
        }
        let deleted = matches!(status_x, 'D') || matches!(status_y, 'D');
        items.push(WorkspaceChange {
            path,
            add: if deleted { 0 } else { 1 },
            del: if deleted { 1 } else { 0 },
        });
    }
    (items, truncated)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
    use super::{parse_porcelain, workspace_changes, MAX_CHANGES};
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "zk-desktop-changes-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn git(root: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .status()
            .expect("git available");
        assert!(status.success(), "git {args:?} failed");
    }

    fn git_available() -> bool {
        Command::new("git").arg("--version").output().is_ok()
    }

    #[test]
    fn parses_statuses_renames_and_caps_entries() {
        let raw = b"?? new file.txt\0A  added.txt\0AM added-then-modified.txt\0MM modified.txt\0D  deleted.txt\0RM renamed.txt\0old.txt\0";
        let (items, truncated) = parse_porcelain(raw, MAX_CHANGES);
        assert!(!truncated);
        assert_eq!(items.len(), 6, "rename consumes its original-path field");
        let by_path = |path: &str| {
            items
                .iter()
                .find(|item| item.path == path)
                .cloned()
                .expect("entry")
        };
        assert_eq!(
            (by_path("new file.txt").add, by_path("new file.txt").del),
            (1, 0)
        );
        assert_eq!((by_path("added.txt").add, by_path("added.txt").del), (1, 0));
        assert_eq!(
            (
                by_path("added-then-modified.txt").add,
                by_path("added-then-modified.txt").del
            ),
            (1, 0)
        );
        assert_eq!(
            (by_path("modified.txt").add, by_path("modified.txt").del),
            (1, 0)
        );
        assert_eq!(
            (by_path("deleted.txt").add, by_path("deleted.txt").del),
            (0, 1)
        );
        assert_eq!(
            (by_path("renamed.txt").add, by_path("renamed.txt").del),
            (1, 0)
        );
        assert!(!items.iter().any(|item| item.path == "old.txt"));

        let mut many = Vec::new();
        for index in 0..=MAX_CHANGES {
            many.extend_from_slice(format!("?? file-{index}\0").as_bytes());
        }
        let (capped, truncated) = parse_porcelain(&many, MAX_CHANGES);
        assert_eq!(capped.len(), MAX_CHANGES);
        assert!(truncated);
    }

    #[test]
    fn temp_repository_reports_added_and_deleted_paths() {
        if !git_available() {
            return;
        }
        let dir = temp_dir("repo");
        git(&dir, &["init", "-q"]);
        git(&dir, &["config", "user.email", "review@example.invalid"]);
        git(&dir, &["config", "user.name", "Reviewer"]);
        std::fs::write(dir.join("keep.txt"), "one").expect("write keep");
        std::fs::write(dir.join("deleted.txt"), "gone soon").expect("write deleted");
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "-qm", "init"]);

        std::fs::write(dir.join("keep.txt"), "changed").expect("modify keep");
        std::fs::write(dir.join("post-commit.txt"), "new").expect("write new");
        std::fs::remove_file(dir.join("deleted.txt")).expect("delete");

        let result = workspace_changes(dir.to_str().expect("utf8 temp path"));
        assert_eq!(result.warning, None, "{:?}", result.warning);
        let entry = |path: &str| {
            result
                .items
                .iter()
                .find(|item| item.path == path)
                .cloned()
                .expect("entry")
        };
        assert_eq!((entry("keep.txt").add, entry("keep.txt").del), (1, 0));
        assert_eq!(
            (entry("post-commit.txt").add, entry("post-commit.txt").del),
            (1, 0)
        );
        assert_eq!((entry("deleted.txt").add, entry("deleted.txt").del), (0, 1));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn repo_local_fsmonitor_is_not_executed() {
        use std::os::unix::fs::PermissionsExt;

        if !git_available() {
            return;
        }
        let dir = temp_dir("fsmonitor");
        let marker = std::env::temp_dir().join(format!(
            "zk-desktop-fsmonitor-marker-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let script = dir.join("fsmonitor.sh");
        let script_body = format!("#!/bin/sh\nprintf executed > '{}'\n", marker.display());
        std::fs::write(&script, script_body).expect("write fsmonitor script");
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
            .expect("chmod fsmonitor");
        git(&dir, &["init", "-q"]);
        git(&dir, &["config", "user.email", "review@example.invalid"]);
        git(&dir, &["config", "user.name", "Reviewer"]);
        git(
            &dir,
            &[
                "config",
                "core.fsmonitor",
                script.to_str().expect("utf8 script"),
            ],
        );
        // Prove the fixture is sensitive: a plain git status executes the hook.
        let probe = Command::new("git")
            .arg("-C")
            .arg(&dir)
            .arg("status")
            .arg("--porcelain")
            .output()
            .expect("git status probe");
        assert!(probe.status.success(), "probe status failed");
        assert!(
            marker.exists(),
            "fixture invalid: plain git status did not execute core.fsmonitor"
        );
        std::fs::remove_file(&marker).expect("clear marker");

        let result = workspace_changes(dir.to_str().expect("utf8 temp path"));
        assert_eq!(result.warning, None, "{:?}", result.warning);
        assert!(
            !marker.exists(),
            "workspace_changes executed repo-local core.fsmonitor"
        );
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&marker);
    }

    #[test]
    fn non_repository_returns_empty_with_warning() {
        if !git_available() {
            return;
        }
        let dir = temp_dir("not-repo");
        let result = workspace_changes(dir.to_str().expect("utf8 temp path"));
        assert!(result.items.is_empty(), "{:?}", result.items);
        let warning = result.warning.expect("warning");
        assert!(warning.contains("not a git repository"), "{warning}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
