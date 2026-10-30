//! Shared implementations for modular agent tools.
//!
//! @capability kit.tools
#![forbid(unsafe_code)]

use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use zk_contract_tool::{ToolError, TurnContext};

/// Maximum bytes returned by one tool call.
pub const MAX_OUTPUT_BYTES: usize = 64 * 1024;
/// Maximum bytes accepted by a write/edit.
pub const MAX_WRITE_BYTES: usize = 512 * 1024;
/// Default command timeout.
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// Resolve the workspace root from a turn context.
pub fn workspace_root(context: &TurnContext) -> Result<PathBuf, ToolError> {
    if context.workspace_root.trim().is_empty() {
        return Err(ToolError::Denied("workspace_root is empty".to_owned()));
    }
    let root = PathBuf::from(&context.workspace_root);
    root.canonicalize().map_err(|error| ToolError::Io(format!("workspace root: {error}")))
}

/// Resolve a relative path inside the workspace; rejects absolute/traversal/symlink escape.
pub fn safe_path(context: &TurnContext, relative: &str) -> Result<PathBuf, ToolError> {
    let relative = relative.trim();
    if relative.is_empty() {
        return Err(ToolError::InvalidArgs("path is empty".to_owned()));
    }
    let requested = Path::new(relative);
    if requested.is_absolute() {
        return Err(ToolError::Denied("absolute paths are not allowed".to_owned()));
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
        ancestor = ancestor.parent().ok_or_else(|| ToolError::Denied("path escapes workspace".to_owned()))?;
    }
    let canonical_ancestor = ancestor.canonicalize().map_err(|error| ToolError::Io(error.to_string()))?;
    if !canonical_ancestor.starts_with(&root) {
        return Err(ToolError::Denied("path escapes workspace".to_owned()));
    }
    Ok(joined)
}

/// Read a UTF-8 file (bounded).
pub fn read_text(path: &Path) -> Result<String, ToolError> {
    let metadata = std::fs::metadata(path).map_err(|error| ToolError::Io(error.to_string()))?;
    if metadata.len() > u64::try_from(MAX_WRITE_BYTES).unwrap_or(u64::MAX) {
        return Err(ToolError::Denied(format!("file too large: {} bytes", metadata.len())));
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
        if number < offset { continue; }
        if number >= offset.saturating_add(limit) { break; }
        out.push_str(&format!("{number}\t{line}\n"));
    }
    out
}

/// Run a shell command in the workspace with a timeout and bounded output.
pub fn run_bash(context: &TurnContext, command: &str, cwd: Option<&str>, timeout_ms: Option<u64>) -> Result<String, ToolError> {
    if command.trim().is_empty() { return Err(ToolError::InvalidArgs("command is empty".to_owned())); }
    let root = workspace_root(context)?;
    let workdir = match cwd {
        Some(cwd) if !cwd.trim().is_empty() => safe_path(context, cwd)?,
        _ => root,
    };
    if !workdir.is_dir() { return Err(ToolError::InvalidArgs("cwd is not a directory".to_owned())); }
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
        if child.try_wait().map_err(|error| ToolError::Io(error.to_string()))?.is_some() { break; }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ToolError::Io(format!("command timed out after {}ms", timeout.as_millis())));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().map_err(|error| ToolError::Io(error.to_string()))?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() { text.push_str("\n[stderr]\n"); text.push_str(&stderr); }
    if !output.status.success() { text.push_str(&format!("\n[exit code: {}]", output.status.code().unwrap_or(-1))); }
    Ok(truncate(text))
}

/// Search files by name/extension and line content (bounded).
pub fn search(context: &TurnContext, query: &str, path: Option<&str>) -> Result<String, ToolError> {
    let root = workspace_root(context)?;
    let base = match path { Some(path) if !path.trim().is_empty() => safe_path(context, path)?, _ => root.clone() };
    let mut results = Vec::new();
    let mut stack = vec![base];
    while let Some(dir) = stack.pop() {
        if results.len() >= 200 { break; }
        let entries = std::fs::read_dir(&dir).map_err(|error| ToolError::Io(error.to_string()))?;
        for entry in entries {
            let entry = entry.map_err(|error| ToolError::Io(error.to_string()))?;
            let file_type = entry.file_type().map_err(|error| ToolError::Io(error.to_string()))?;
            let path = entry.path();
            if file_type.is_dir() {
                if entry.file_name() == ".git" || entry.file_name() == "node_modules" { continue; }
                stack.push(path);
                continue;
            }
            if !file_type.is_file() { continue; }
            let relative = path.strip_prefix(&root).unwrap_or(&path).display().to_string();
            if relative.contains(query) { results.push(relative.clone()); continue; }
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
    if results.is_empty() { Ok("no matches".to_owned()) } else { Ok(truncate(results.join("\n"))) }
}

pub mod persistent;


fn glob_match(pattern: &str, text: &str) -> bool {
    fn inner(p: &[u8], t: &[u8]) -> bool {
        let Some(&first) = p.first() else { return t.is_empty(); };
        let rest = p.get(1..).unwrap_or_default();
        let text_rest = t.get(1..).unwrap_or_default();
        match first {
            b'*' => {
                if p.get(1) == Some(&b'*') {
                    let after = p.get(2..).unwrap_or_default();
                    if inner(after, t) { return true; }
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
pub fn glob_files(context: &TurnContext, pattern: &str, path: Option<&str>) -> Result<String, ToolError> {
    let root = workspace_root(context)?;
    let base = match path { Some(path) if !path.trim().is_empty() => safe_path(context, path)?, _ => root.clone() };
    let mut stack = vec![base];
    let mut out = Vec::new();
    while let Some(dir) = stack.pop() {
        if out.len() >= 100 { break; }
        for entry in std::fs::read_dir(&dir).map_err(|error| ToolError::Io(error.to_string()))? {
            let entry = entry.map_err(|error| ToolError::Io(error.to_string()))?;
            let file_type = entry.file_type().map_err(|error| ToolError::Io(error.to_string()))?;
            let path = entry.path();
            if file_type.is_dir() { if entry.file_name() != ".git" && entry.file_name() != "node_modules" { stack.push(path); } continue; }
            if !file_type.is_file() { continue; }
            let relative = path.strip_prefix(&root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
            if glob_match(pattern, &relative) || glob_match(pattern, entry.file_name().to_string_lossy().as_ref()) { out.push(relative); }
        }
    }
    out.sort();
    if out.is_empty() { Ok("no matches".to_owned()) } else { Ok(truncate(out.join("\n"))) }
}

/// Search content with a regex.
pub fn grep_files(context: &TurnContext, pattern: &str, path: Option<&str>, include: Option<&str>) -> Result<String, ToolError> {
    let regex = regex::Regex::new(pattern).map_err(|error| ToolError::InvalidArgs(error.to_string()))?;
    let root = workspace_root(context)?;
    let base = match path { Some(path) if !path.trim().is_empty() => safe_path(context, path)?, _ => root.clone() };
    let mut stack = vec![base];
    let mut out = Vec::new();
    while let Some(dir) = stack.pop() {
        if out.len() >= 250 { break; }
        for entry in std::fs::read_dir(&dir).map_err(|error| ToolError::Io(error.to_string()))? {
            let entry = entry.map_err(|error| ToolError::Io(error.to_string()))?;
            let file_type = entry.file_type().map_err(|error| ToolError::Io(error.to_string()))?;
            let path = entry.path();
            if file_type.is_dir() { if entry.file_name() != ".git" && entry.file_name() != "node_modules" { stack.push(path); } continue; }
            if !file_type.is_file() { continue; }
            let relative = path.strip_prefix(&root).unwrap_or(&path).to_string_lossy().to_string();
            if let Some(include) = include { if !glob_match(include, &relative) { continue; } }
            let Ok(text) = std::fs::read_to_string(&path) else { continue; };
            for (index, line) in text.lines().enumerate() {
                if regex.is_match(line) { out.push(format!("{relative}:{}\t{line}", index.saturating_add(1))); if out.len() >= 250 { break; } }
            }
        }
    }
    if out.is_empty() { Ok("no matches".to_owned()) } else { Ok(truncate(out.join("\n"))) }
}
