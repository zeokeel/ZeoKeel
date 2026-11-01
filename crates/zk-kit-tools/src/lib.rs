//! Shared implementations for modular agent tools.
//!
//! @capability kit.tools
#![forbid(unsafe_code)]

use std::path::{Component, Path, PathBuf};

use regex::Regex;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use zk_contract_tool::{ToolError, TurnContext};

/// Maximum bytes returned by one tool call.
pub const MAX_OUTPUT_BYTES: usize = 64 * 1024;
/// Maximum bytes accepted by a write/edit.
pub const MAX_WRITE_BYTES: usize = 512 * 1024;
/// Default command timeout.
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;
/// Bounded approval diff size (bytes).
pub const MAX_APPROVAL_DIFF_BYTES: u64 = 256 * 1024;

/// Argument keys always masked in approval cards and persisted tool rows, in
/// addition to the keys a tool declares via `Tool::sensitive_arg_keys`.
const BUILTIN_SENSITIVE_KEYS: &[&str] = &[
    "api_key",
    "apikey",
    "api-key",
    "access_token",
    "token",
    "secret",
    "client_secret",
    "password",
    "authorization",
    "credential",
];

/// Resolve the workspace root from a turn context.
pub fn workspace_root(context: &TurnContext) -> Result<PathBuf, ToolError> {
    if context.workspace_root.trim().is_empty() {
        return Err(ToolError::Denied("workspace_root is empty".to_owned()));
    }
    let root = PathBuf::from(&context.workspace_root);
    root.canonicalize()
        .map_err(|error| ToolError::Io(format!("workspace root: {error}")))
}

/// Resolve a relative path inside the workspace; rejects absolute/traversal/symlink escape.
pub fn safe_path(context: &TurnContext, relative: &str) -> Result<PathBuf, ToolError> {
    let relative = relative.trim();
    if relative.is_empty() {
        return Err(ToolError::InvalidArgs("path is empty".to_owned()));
    }
    let requested = Path::new(relative);
    if requested.is_absolute() {
        return Err(ToolError::Denied(
            "absolute paths are not allowed".to_owned(),
        ));
    }
    for component in requested.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(ToolError::Denied(format!("unsafe path: {relative}")));
            }
        }
    }
    let root = workspace_root(context)?;
    let joined = root.join(requested);
    // For new files, canonicalize the nearest existing parent.
    let mut ancestor = joined.as_path();
    while !ancestor.exists() {
        ancestor = ancestor
            .parent()
            .ok_or_else(|| ToolError::Denied("path escapes workspace".to_owned()))?;
    }
    let canonical_ancestor = ancestor
        .canonicalize()
        .map_err(|error| ToolError::Io(error.to_string()))?;
    if !canonical_ancestor.starts_with(&root) {
        return Err(ToolError::Denied("path escapes workspace".to_owned()));
    }
    Ok(joined)
}

/// Join `parts` under an existing workspace `root` for a **host-side write**.
///
/// This is the single path-safety boundary for host writes that do not run inside
/// a tool turn (composer attachments, `2026-11-01-client-composer-surface`):
///
/// - the root must exist and canonicalize; the returned path is anchored there;
/// - every part must be one normal component: non-empty, not `.`/`..`, no path
///   separator, no control character, not absolute;
/// - intermediate directories are created step by step, each one checked with
///   `symlink_metadata` (no-follow) **before** use and canonicalized afterwards
///   (`starts_with(canonical root)` must hold at every level);
/// - an existing symlink at any component (parent directory **or** the final
///   file) is rejected, so a symlinked parent cannot redirect the write outside
///   the workspace;
/// - the final component may be a new file name (parent chain already validated);
///   the caller owns the atomic write itself.
///
/// # Errors
///
/// [`ToolError::Denied`] for unsafe components/symlinks/escapes, [`ToolError::Io`]
/// for filesystem failures. Nothing is written when a check fails.
pub fn safe_join_under(root: &Path, parts: &[&str]) -> Result<PathBuf, ToolError> {
    if parts.is_empty() {
        return Err(ToolError::InvalidArgs("path parts are empty".to_owned()));
    }
    // Validate every component before touching the filesystem: a rejected path
    // must not leave partially-created directories behind.
    for part in parts {
        validate_path_part(part)?;
    }
    let canonical_root = root
        .canonicalize()
        .map_err(|error| ToolError::Io(format!("workspace root: {error}")))?;
    if !canonical_root.is_dir() {
        return Err(ToolError::Denied(
            "workspace root is not a directory".to_owned(),
        ));
    }
    let last = parts.len().saturating_sub(1);
    let mut current = canonical_root.clone();
    for (index, part) in parts.iter().enumerate() {
        let next = current.join(part);
        match std::fs::symlink_metadata(&next) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(ToolError::Denied(format!(
                        "symlink is not allowed in a host write path: {part}"
                    )));
                }
                if index < last && !metadata.is_dir() {
                    return Err(ToolError::Denied(format!(
                        "path component is not a directory: {part}"
                    )));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if index < last {
                    std::fs::create_dir(&next)
                        .map_err(|error| ToolError::Io(format!("create dir {part}: {error}")))?;
                }
            }
            Err(error) => {
                return Err(ToolError::Io(format!("inspect path part {part}: {error}")));
            }
        }
        if index < last {
            let canonical = next
                .canonicalize()
                .map_err(|error| ToolError::Io(format!("canonicalize {part}: {error}")))?;
            if !canonical.starts_with(&canonical_root) {
                return Err(ToolError::Denied("path escapes the workspace".to_owned()));
            }
            current = canonical;
        } else {
            current = next;
        }
    }
    Ok(current)
}

/// Resolve an existing path under `root` without creating or modifying anything.
///
/// Same validation as [`safe_join_under`] (no `..`/absolute/separators/control
/// characters; every component must exist and must not be a symlink; the
/// canonical parent must stay under the canonical root), but read-only: the
/// final target must already exist and no directory is created. Used by file
/// preview (`workspace.file_read`) so an invalid path cannot create dirs.
///
/// # Errors
/// [`ToolError::Denied`] for unsafe paths/symlinks, [`ToolError::NotFound`] when a
/// component is missing, [`ToolError::Io`] for other filesystem failures.
pub fn safe_resolve_under(root: &Path, parts: &[&str]) -> Result<PathBuf, ToolError> {
    if parts.is_empty() {
        return Err(ToolError::InvalidArgs("path parts are empty".to_owned()));
    }
    for part in parts {
        validate_path_part(part)?;
    }
    let canonical_root = root
        .canonicalize()
        .map_err(|error| ToolError::Io(format!("workspace root: {error}")))?;
    if !canonical_root.is_dir() {
        return Err(ToolError::Denied(
            "workspace root is not a directory".to_owned(),
        ));
    }
    let last = parts.len().saturating_sub(1);
    let mut current = canonical_root.clone();
    for (index, part) in parts.iter().enumerate() {
        let next = current.join(part);
        let metadata = match std::fs::symlink_metadata(&next) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(ToolError::NotFound(format!("path not found: {part}")));
            }
            Err(error) => {
                return Err(ToolError::Io(format!("inspect path part {part}: {error}")));
            }
        };
        if metadata.file_type().is_symlink() {
            return Err(ToolError::Denied(format!(
                "symlink is not allowed in a host read path: {part}"
            )));
        }
        if index < last {
            if !metadata.is_dir() {
                return Err(ToolError::Denied(format!(
                    "path component is not a directory: {part}"
                )));
            }
            let canonical = next
                .canonicalize()
                .map_err(|error| ToolError::Io(format!("canonicalize {part}: {error}")))?;
            if !canonical.starts_with(&canonical_root) {
                return Err(ToolError::Denied("path escapes the workspace".to_owned()));
            }
            current = canonical;
        } else {
            current = next;
        }
    }
    Ok(current)
}

fn validate_path_part(part: &str) -> Result<(), ToolError> {
    if part.is_empty() || part == "." || part == ".." {
        return Err(ToolError::Denied(format!("unsafe path part: {part:?}")));
    }
    if part.contains('/') || part.contains('\\') {
        return Err(ToolError::Denied(format!(
            "path separator in one path part: {part:?}"
        )));
    }
    if part.chars().any(char::is_control) {
        return Err(ToolError::Denied(format!(
            "control character in path part: {part:?}"
        )));
    }
    let path = Path::new(part);
    if path.is_absolute() || path.components().count() != 1 {
        return Err(ToolError::Denied(format!("unsafe path part: {part:?}")));
    }
    Ok(())
}

/// Read a UTF-8 file (bounded).
pub fn read_text(path: &Path) -> Result<String, ToolError> {
    let metadata = std::fs::metadata(path).map_err(|error| ToolError::Io(error.to_string()))?;
    if metadata.len() > u64::try_from(MAX_WRITE_BYTES).unwrap_or(u64::MAX) {
        return Err(ToolError::Denied(format!(
            "file too large: {} bytes",
            metadata.len()
        )));
    }
    std::fs::read_to_string(path).map_err(|error| ToolError::Io(error.to_string()))
}

/// Write a UTF-8 file, creating parents.
pub fn write_text(path: &Path, content: &str) -> Result<(), ToolError> {
    if content.len() > MAX_WRITE_BYTES {
        return Err(ToolError::Denied("content too large".to_owned()));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| ToolError::Io(error.to_string()))?;
    }
    std::fs::write(path, content).map_err(|error| ToolError::Io(error.to_string()))
}

/// Truncate output to [`MAX_OUTPUT_BYTES`] with a visible marker.
#[must_use]
pub fn truncate(mut text: String) -> String {
    if text.len() <= MAX_OUTPUT_BYTES {
        return text;
    }
    let mut boundary = MAX_OUTPUT_BYTES;
    while boundary > 0 && !text.is_char_boundary(boundary) {
        boundary = boundary.saturating_sub(1);
    }
    text.truncate(boundary);
    text.push_str("\n... [truncated]");
    text
}

/// Numbered read window.
pub fn read_window(text: &str, offset: usize, limit: usize) -> String {
    let offset = offset.max(1);
    let limit = limit.clamp(1, 2000);
    let mut out = String::new();
    for (index, line) in text.lines().enumerate() {
        let number = index.saturating_add(1);
        if number < offset {
            continue;
        }
        if number >= offset.saturating_add(limit) {
            break;
        }
        out.push_str(&format!("{number}\t{line}\n"));
    }
    out
}

/// Run a shell command in the workspace with a timeout and bounded output.
pub fn run_bash(
    context: &TurnContext,
    command: &str,
    cwd: Option<&str>,
    timeout_ms: Option<u64>,
) -> Result<String, ToolError> {
    if command.trim().is_empty() {
        return Err(ToolError::InvalidArgs("command is empty".to_owned()));
    }
    let root = workspace_root(context)?;
    let workdir = match cwd {
        Some(cwd) if !cwd.trim().is_empty() => safe_path(context, cwd)?,
        _ => root,
    };
    if !workdir.is_dir() {
        return Err(ToolError::InvalidArgs("cwd is not a directory".to_owned()));
    }
    let mut child = Command::new("bash")
        .arg("-lc")
        .arg(command)
        .current_dir(&workdir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| ToolError::Io(error.to_string()))?;
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS).clamp(1, 120_000));
    let start = Instant::now();
    loop {
        if child
            .try_wait()
            .map_err(|error| ToolError::Io(error.to_string()))?
            .is_some()
        {
            break;
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ToolError::Io(format!(
                "command timed out after {}ms",
                timeout.as_millis()
            )));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child
        .wait_with_output()
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
    Ok(truncate(text))
}

/// Search files by name/extension and line content (bounded).
pub fn search(context: &TurnContext, query: &str, path: Option<&str>) -> Result<String, ToolError> {
    let root = workspace_root(context)?;
    let base = match path {
        Some(path) if !path.trim().is_empty() => safe_path(context, path)?,
        _ => root.clone(),
    };
    let mut results = Vec::new();
    let mut stack = vec![base];
    while let Some(dir) = stack.pop() {
        if results.len() >= 200 {
            break;
        }
        let entries = std::fs::read_dir(&dir).map_err(|error| ToolError::Io(error.to_string()))?;
        for entry in entries {
            let entry = entry.map_err(|error| ToolError::Io(error.to_string()))?;
            let file_type = entry
                .file_type()
                .map_err(|error| ToolError::Io(error.to_string()))?;
            let path = entry.path();
            if file_type.is_dir() {
                if entry.file_name() == ".git" || entry.file_name() == "node_modules" {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            let relative = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .display()
                .to_string();
            if relative.contains(query) {
                results.push(relative.clone());
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&path) {
                for (index, line) in text.lines().enumerate() {
                    if line.contains(query) {
                        results.push(format!("{relative}:{}\t{line}", index.saturating_add(1)));
                        break;
                    }
                }
            }
        }
    }
    if results.is_empty() {
        Ok("no matches".to_owned())
    } else {
        Ok(truncate(results.join("\n")))
    }
}

pub mod persistent;

fn glob_match(pattern: &str, text: &str) -> bool {
    fn inner(p: &[u8], t: &[u8]) -> bool {
        let Some(&first) = p.first() else {
            return t.is_empty();
        };
        let rest = p.get(1..).unwrap_or_default();
        let text_rest = t.get(1..).unwrap_or_default();
        match first {
            b'*' => {
                if p.get(1) == Some(&b'*') {
                    let after = p.get(2..).unwrap_or_default();
                    if inner(after, t) {
                        return true;
                    }
                    return !t.is_empty() && inner(p, text_rest);
                }
                inner(rest, t) || (!t.is_empty() && inner(p, text_rest))
            }
            b'?' => !t.is_empty() && inner(rest, text_rest),
            byte => t.first() == Some(&byte) && inner(rest, text_rest),
        }
    }
    inner(pattern.as_bytes(), text.as_bytes())
}

/// Find files whose relative path matches a glob pattern.
pub fn glob_files(
    context: &TurnContext,
    pattern: &str,
    path: Option<&str>,
) -> Result<String, ToolError> {
    let root = workspace_root(context)?;
    let base = match path {
        Some(path) if !path.trim().is_empty() => safe_path(context, path)?,
        _ => root.clone(),
    };
    let mut stack = vec![base];
    let mut out = Vec::new();
    while let Some(dir) = stack.pop() {
        if out.len() >= 100 {
            break;
        }
        for entry in std::fs::read_dir(&dir).map_err(|error| ToolError::Io(error.to_string()))? {
            let entry = entry.map_err(|error| ToolError::Io(error.to_string()))?;
            let file_type = entry
                .file_type()
                .map_err(|error| ToolError::Io(error.to_string()))?;
            let path = entry.path();
            if file_type.is_dir() {
                if entry.file_name() != ".git" && entry.file_name() != "node_modules" {
                    stack.push(path);
                }
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            let relative = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if glob_match(pattern, &relative)
                || glob_match(pattern, entry.file_name().to_string_lossy().as_ref())
            {
                out.push(relative);
            }
        }
    }
    out.sort();
    if out.is_empty() {
        Ok("no matches".to_owned())
    } else {
        Ok(truncate(out.join("\n")))
    }
}

/// Search content with a regex.
pub fn grep_files(
    context: &TurnContext,
    pattern: &str,
    path: Option<&str>,
    include: Option<&str>,
) -> Result<String, ToolError> {
    let regex =
        regex::Regex::new(pattern).map_err(|error| ToolError::InvalidArgs(error.to_string()))?;
    let root = workspace_root(context)?;
    let base = match path {
        Some(path) if !path.trim().is_empty() => safe_path(context, path)?,
        _ => root.clone(),
    };
    let mut stack = vec![base];
    let mut out = Vec::new();
    while let Some(dir) = stack.pop() {
        if out.len() >= 250 {
            break;
        }
        for entry in std::fs::read_dir(&dir).map_err(|error| ToolError::Io(error.to_string()))? {
            let entry = entry.map_err(|error| ToolError::Io(error.to_string()))?;
            let file_type = entry
                .file_type()
                .map_err(|error| ToolError::Io(error.to_string()))?;
            let path = entry.path();
            if file_type.is_dir() {
                if entry.file_name() != ".git" && entry.file_name() != "node_modules" {
                    stack.push(path);
                }
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            let relative = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            if let Some(include) = include {
                if !glob_match(include, &relative) {
                    continue;
                }
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            for (index, line) in text.lines().enumerate() {
                if regex.is_match(line) {
                    out.push(format!("{relative}:{}\t{line}", index.saturating_add(1)));
                    if out.len() >= 250 {
                        break;
                    }
                }
            }
        }
    }
    if out.is_empty() {
        Ok("no matches".to_owned())
    } else {
        Ok(truncate(out.join("\n")))
    }
}

/// Mask sensitive argument values for display and persistence.
///
/// `sensitive_keys` comes from [`zk_contract_tool::Tool::sensitive_arg_keys`];
/// the built-in key list above is always applied. Object keys are matched
/// case-insensitively and recursively, then common token shapes are masked
/// inside every remaining string (the rest of the text stays readable).
#[must_use]
pub fn redact_tool_args(sensitive_keys: &[&str], args: &serde_json::Value) -> serde_json::Value {
    static KEY_VALUE: std::sync::OnceLock<Option<Regex>> = std::sync::OnceLock::new();
    static BEARER: std::sync::OnceLock<Option<Regex>> = std::sync::OnceLock::new();
    static SK: std::sync::OnceLock<Option<Regex>> = std::sync::OnceLock::new();
    let kv = KEY_VALUE.get_or_init(|| {
        Regex::new(r"(?i)(api[_-]?key|access[_-]?token|token|secret|password)\s*[=:]\s*\S+").ok()
    });
    let bearer = BEARER.get_or_init(|| Regex::new(r"(?i)bearer\s+[A-Za-z0-9._~+/=-]{8,}").ok());
    let sk = SK.get_or_init(|| Regex::new(r"sk-[A-Za-z0-9_-]{8,}").ok());
    fn mask_string(
        value: &str,
        kv: Option<&Regex>,
        bearer: Option<&Regex>,
        sk: Option<&Regex>,
    ) -> String {
        let mut masked = value.to_owned();
        if let Some(regex) = kv {
            masked = regex.replace_all(&masked, "$1=***").into_owned();
        }
        if let Some(regex) = bearer {
            masked = regex.replace_all(&masked, "Bearer ***").into_owned();
        }
        if let Some(regex) = sk {
            masked = regex.replace_all(&masked, "sk-***").into_owned();
        }
        masked
    }
    fn walk(
        value: &serde_json::Value,
        keys: &[&str],
        kv: Option<&Regex>,
        bearer: Option<&Regex>,
        sk: Option<&Regex>,
    ) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let mut out = serde_json::Map::with_capacity(map.len());
                for (key, item) in map {
                    let lower = key.to_ascii_lowercase();
                    let sensitive = keys.iter().any(|k| k.eq_ignore_ascii_case(&lower));
                    let next = if sensitive {
                        match item {
                            serde_json::Value::String(_) => {
                                serde_json::Value::String("***".to_owned())
                            }
                            serde_json::Value::Null => serde_json::Value::Null,
                            other => serde_json::Value::String(mask_string(
                                &other.to_string(),
                                kv,
                                bearer,
                                sk,
                            )),
                        }
                    } else {
                        walk(item, keys, kv, bearer, sk)
                    };
                    out.insert(key.clone(), next);
                }
                serde_json::Value::Object(out)
            }
            serde_json::Value::Array(items) => serde_json::Value::Array(
                items
                    .iter()
                    .map(|item| walk(item, keys, kv, bearer, sk))
                    .collect(),
            ),
            serde_json::Value::String(text) => {
                serde_json::Value::String(mask_string(text, kv, bearer, sk))
            }
            other => other.clone(),
        }
    }
    let mut keys: Vec<&str> = BUILTIN_SENSITIVE_KEYS.to_vec();
    for key in sensitive_keys {
        if !keys
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(key))
        {
            keys.push(key);
        }
    }
    walk(args, &keys, kv.as_ref(), bearer.as_ref(), sk.as_ref())
}

/// Render `path` relative to the workspace root when it is inside it.
#[must_use]
pub fn workspace_relative(context: &TurnContext, path: &Path) -> String {
    let root = Path::new(&context.workspace_root);
    if let Ok(relative) = path.strip_prefix(root) {
        return relative
            .components()
            .filter_map(|component| match component {
                Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("/");
    }
    path.display().to_string()
}

/// Build a bounded, human-readable diff for an approval card.
///
/// Returns `None` when the previous content is too large or unreadable; the
/// tool then omits the preview instead of blocking the approval.
#[must_use]
pub fn approval_diff(previous: Option<&str>, next: &str) -> Option<String> {
    let previous = previous?;
    if u64::try_from(previous.len()).unwrap_or(u64::MAX) > MAX_APPROVAL_DIFF_BYTES
        || u64::try_from(next.len()).unwrap_or(u64::MAX) > MAX_APPROVAL_DIFF_BYTES
    {
        return None;
    }
    let old_lines: Vec<&str> = previous.lines().collect();
    let new_lines: Vec<&str> = next.lines().collect();
    let mut diff = String::new();
    let mut index = 0_usize;
    while index < old_lines.len() || index < new_lines.len() {
        let old_line = old_lines.get(index).copied().unwrap_or("");
        let new_line = new_lines.get(index).copied().unwrap_or("");
        if old_line != new_line {
            if !old_line.is_empty() || index < old_lines.len() {
                diff.push('-');
                diff.push_str(old_line);
                diff.push('\n');
            }
            if !new_line.is_empty() || index < new_lines.len() {
                diff.push('+');
                diff.push_str(new_line);
                diff.push('\n');
            }
        }
        index = index.saturating_add(1);
    }
    if diff.is_empty() {
        Some("(no changes)".to_owned())
    } else {
        Some(truncate(diff))
    }
}

/// Read an existing UTF-8 file for approval-diff previews (bounded).
#[must_use]
pub fn read_for_diff(path: &Path) -> Option<String> {
    let metadata = std::fs::metadata(path).ok()?;
    if metadata.len() > MAX_APPROVAL_DIFF_BYTES {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)] // test-only
mod safe_join_tests {
    use super::{safe_join_under, ToolError};
    use std::path::{Path, PathBuf};

    fn temp_root(name: &str) -> PathBuf {
        let unique = format!(
            "{}-{}-{}",
            std::process::id(),
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let root = std::env::temp_dir().join(format!("zk-safe-join-{unique}"));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[cfg(unix)]
    fn symlink(target: &Path, link: &Path) {
        std::os::unix::fs::symlink(target, link).unwrap();
    }

    #[test]
    fn creates_nested_directories_and_writes_inside_the_root() {
        let root = temp_root("ok");
        let path = safe_join_under(
            &root,
            &[".zeokeel", "attachments", "conv-1", "abc-shot.png"],
        )
        .unwrap();
        assert!(path.starts_with(root.canonicalize().unwrap()));
        assert!(path.parent().unwrap().is_dir());
        std::fs::write(&path, b"data").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"data");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn rejects_traversal_absolute_and_separator_parts() {
        let root = temp_root("parts");
        for parts in [
            vec!["..", "escape.txt"],
            vec!["a", "..", "escape.txt"],
            vec!["/tmp/escape.txt"],
            vec!["a/b.txt"],
            vec!["..\\escape.txt"],
            vec![""],
            vec!["."],
            vec!["a", "bad\u{0007}name"],
        ] {
            let error = safe_join_under(&root, &parts).expect_err("unsafe part must be denied");
            assert!(
                matches!(error, ToolError::Denied(_) | ToolError::InvalidArgs(_)),
                "{parts:?} -> {error:?}"
            );
        }
        assert_eq!(
            std::fs::read_dir(&root).unwrap().count(),
            0,
            "rejected paths must not create anything"
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn rejects_a_missing_root() {
        let root = temp_root("missing").join("does-not-exist");
        assert!(safe_join_under(&root, &["x.txt"]).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_parent_directory_symlink_escape() {
        let root = temp_root("parent-symlink");
        let outside = temp_root("outside-parent");
        symlink(&outside, &root.join("link"));
        let error = safe_join_under(&root, &["link", "escape.txt"])
            .expect_err("parent symlink must be denied");
        assert!(matches!(error, ToolError::Denied(_)), "{error:?}");
        assert!(!outside.join("escape.txt").exists());
        std::fs::remove_dir_all(&root).unwrap();
        std::fs::remove_dir_all(&outside).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn rejects_nested_attachments_symlink_escape() {
        let root = temp_root("nested-symlink");
        let outside = temp_root("outside-nested");
        // `.zeokeel` itself is the symlinked parent the host must never follow.
        symlink(&outside, &root.join(".zeokeel"));
        let error = safe_join_under(&root, &[".zeokeel", "attachments", "conv", "x.png"])
            .expect_err("nested symlink must be denied");
        assert!(matches!(error, ToolError::Denied(_)), "{error:?}");
        assert!(!outside.join("attachments").exists());
        std::fs::remove_dir_all(&root).unwrap();
        std::fs::remove_dir_all(&outside).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn rejects_existing_symlink_at_the_final_component() {
        let root = temp_root("file-symlink");
        let outside = temp_root("outside-file");
        std::fs::create_dir_all(root.join(".zeokeel/attachments/conv")).unwrap();
        let target = outside.join("target.txt");
        std::fs::write(&target, b"outside").unwrap();
        symlink(&target, &root.join(".zeokeel/attachments/conv/leak.txt"));
        let error = safe_join_under(&root, &[".zeokeel", "attachments", "conv", "leak.txt"])
            .expect_err("final symlink must be denied");
        assert!(matches!(error, ToolError::Denied(_)), "{error:?}");
        assert_eq!(std::fs::read(&target).unwrap(), b"outside");
        std::fs::remove_dir_all(&root).unwrap();
        std::fs::remove_dir_all(&outside).unwrap();
    }

    #[test]
    fn allows_existing_regular_directory_components() {
        let root = temp_root("existing");
        std::fs::create_dir_all(root.join(".zeokeel/attachments/conv")).unwrap();
        let path = safe_join_under(&root, &[".zeokeel", "attachments", "conv", "new.txt"]).unwrap();
        assert!(path.starts_with(root.canonicalize().unwrap()));
        std::fs::remove_dir_all(&root).unwrap();
    }
}
