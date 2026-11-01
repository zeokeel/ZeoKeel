//! Workspace + installed skill loader (single implementation).
//!
//! `list`/`read` back both the agent tool (default text output stays the legacy
//! newline-separated directory list) and the host `skill.list`/`skill.read` IPC
//! through the generated facade (`2026-11-01-client-skill-commands`):
//!
//! - workspace skills: `<workspace>/.agents/skills/<name>/SKILL.md` (personal);
//! - installed skills: `$ZEOKEEL_HOME/skills/<name>/<version>/SKILL.md`
//!   (enterprise store installs; the latest installed version wins when
//!   `skill.read` omits `version`);
//! - frontmatter parsing, `user-invocable`, the 8 KiB Instructions bound and the
//!   safe path/name/version checks live here only (no second scanner, C17).
#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::io::Read;
use std::path::{Path, PathBuf};

use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

/// Maximum `SKILL.md` bytes injected as composer Instructions (8 KiB).
pub const MAX_SKILL_BYTES: usize = 8 * 1024;

/// Frontmatter parse window (first 4 KiB of `SKILL.md`).
pub const FRONTMATTER_WINDOW_BYTES: usize = 4 * 1024;

/// Where one skill candidate comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillSource {
    /// Workspace `.agents/skills` (personal edition).
    Workspace,
    /// Installed store skill `$ZEOKEEL_HOME/skills/<name>/<version>` (enterprise).
    Installed,
}

impl SkillSource {
    /// Stable wire string (`workspace` | `installed`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::Installed => "installed",
        }
    }
}

/// One skill candidate (frontmatter metadata, sorted by name).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SkillEntry {
    /// Display name (frontmatter `name`, falling back to the directory name).
    pub name: String,
    /// Frontmatter `description` (empty when absent).
    pub description: String,
    /// Frontmatter `user-invocable` (true unless explicitly false).
    pub user_invocable: bool,
    /// Installed version (enterprise only).
    pub version: Option<String>,
    /// Candidate source.
    pub source: SkillSource,
}

/// One read skill body (≤ [`MAX_SKILL_BYTES`]) plus provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillContent {
    /// Display name (frontmatter `name`, falling back to the directory name).
    pub name: String,
    /// Installed version (enterprise only).
    pub version: Option<String>,
    /// `SKILL.md` content (UTF-8, ≤8 KiB).
    pub content: String,
    /// Content source.
    pub source: SkillSource,
}

/// Closed skill loading error set (mapped to `skill/*` codes by the host).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SkillError {
    /// Unknown skill (or workspace skill directory missing).
    #[error("skill/not-found: {0}")]
    NotFound(String),
    /// Installed skill/version missing (enterprise; never reads uninstalled content).
    #[error("skill/unavailable: {0}")]
    Unavailable(String),
    /// `SKILL.md` exceeds the 8 KiB Instructions bound.
    #[error("skill/too-large: {0}")]
    TooLarge(String),
    /// Unsafe/unsupported name or version.
    #[error("skill/invalid: {0}")]
    Invalid(String),
    /// Filesystem/IO failure.
    #[error("skill/failed: {0}")]
    Failed(String),
}

/// `skill` agent tool (list/read; read-only, defaults keep the legacy text list).
pub struct SkillTool;

/// Tool factory used by generated glue (role `tool.skill`).
#[must_use]
pub fn tool() -> SkillTool {
    SkillTool
}

impl Tool for SkillTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "skill".to_owned(),
            description:
                "List or read agent skills under .agents/skills (read-only; `list` defaults to the \
                 legacy newline list, `format:\"json\"` returns frontmatter metadata)."
                    .to_owned(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["list", "read"] },
                    "name": { "type": "string" },
                    "format": { "type": "string", "enum": ["text", "json"] }
                },
                "required": ["action"],
                "additionalProperties": false
            }),
            mutating: false,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "skill requires JSON arguments".to_owned(),
        ))
    }
    fn call_json(
        &self,
        context: &TurnContext,
        args: serde_json::Value,
    ) -> Result<String, ToolError> {
        let action = args
            .get("action")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("action".to_owned()))?;
        match action {
            "list" => {
                let format = args
                    .get("format")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("text");
                match format {
                    "text" => list_legacy_names(&context.workspace_root),
                    "json" => {
                        let root = workspace_skills_dir(&context.workspace_root);
                        let skills = scan_workspace(&root).map_err(tool_error)?;
                        serde_json::to_string(&serde_json::json!({ "skills": skills }))
                            .map_err(|error| ToolError::Io(error.to_string()))
                    }
                    other => Err(ToolError::InvalidArgs(format!(
                        "unknown format {other} (text|json)"
                    ))),
                }
            }
            "read" => {
                let name = args
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| ToolError::InvalidArgs("name".to_owned()))?;
                read_workspace(&context.workspace_root, name)
                    .map(|content| zk_kit_tools::truncate(content.content))
                    .map_err(tool_error)
            }
            other => Err(ToolError::InvalidArgs(format!("unknown action {other}"))),
        }
    }
}

fn tool_error(error: SkillError) -> ToolError {
    // Keep the stable `skill/*` code in the model-visible tool error text.
    let message = error.to_string();
    match error {
        SkillError::Invalid(_) => ToolError::InvalidArgs(message),
        SkillError::Failed(_) => ToolError::Io(message),
        SkillError::NotFound(_) | SkillError::Unavailable(_) | SkillError::TooLarge(_) => {
            ToolError::Denied(message)
        }
    }
}

/// `<workspace>/.agents/skills`.
#[must_use]
pub fn workspace_skills_dir(workspace_root: &str) -> PathBuf {
    Path::new(workspace_root).join(".agents").join("skills")
}

/// Installed-skill root: `$ZEOKEEL_HOME/skills` (or `~/.zeokeel/skills`).
///
/// # Errors
/// [`SkillError::Failed`] when neither `ZEOKEEL_HOME` nor a home directory resolves.
pub fn installed_root() -> Result<PathBuf, SkillError> {
    match std::env::var("ZEOKEEL_HOME") {
        Ok(value) if !value.trim().is_empty() => Ok(PathBuf::from(value).join("skills")),
        Ok(_) | Err(_) => dirs::home_dir()
            .map(|home| home.join(".zeokeel").join("skills"))
            .ok_or_else(|| SkillError::Failed("home directory unavailable".to_owned())),
    }
}

/// Whether a skill name/version is one safe path component (no separator/traversal).
#[must_use]
pub fn is_safe_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value != "."
        && value != ".."
        && !value.starts_with('.')
        && !value.contains(':')
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
}

/// Legacy text list: sorted directory names under `.agents/skills` (unchanged).
fn list_legacy_names(workspace_root: &str) -> Result<String, ToolError> {
    let root = workspace_skills_dir(workspace_root);
    if !root.is_dir() {
        return Ok("no skills".to_owned());
    }
    let mut names = Vec::new();
    for entry in std::fs::read_dir(&root).map_err(|error| ToolError::Io(error.to_string()))? {
        let entry = entry.map_err(|error| ToolError::Io(error.to_string()))?;
        if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    names.sort();
    Ok(zk_kit_tools::truncate(names.join("\n")))
}

/// List workspace skills (frontmatter parsed; sorted by display name).
///
/// # Errors
/// [`SkillError::Failed`] for filesystem read failures; a missing root is empty.
pub fn list_workspace(workspace_root: &str) -> Result<Vec<SkillEntry>, SkillError> {
    scan_workspace(&workspace_skills_dir(workspace_root))
}

/// List installed skills: one entry per name (latest installed version).
///
/// # Errors
/// [`SkillError::Failed`] for filesystem read failures; a missing root is empty.
pub fn list_installed(root: &Path) -> Result<Vec<SkillEntry>, SkillError> {
    let mut skills = Vec::new();
    for name in sorted_dirs(root)? {
        let dir = root.join(&name);
        let versions = versions_of(&dir)?;
        let Some(version) = versions.last() else {
            continue;
        };
        let frontmatter = frontmatter_of(&dir.join(version).join("SKILL.md"));
        skills.push(SkillEntry {
            name: frontmatter.name.clone().unwrap_or_else(|| name.clone()),
            description: frontmatter.description.unwrap_or_default(),
            user_invocable: frontmatter.user_invocable.unwrap_or(true),
            version: Some(version.clone()),
            source: SkillSource::Installed,
        });
    }
    skills.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(skills)
}

/// Read one workspace skill (`SKILL.md`, ≤8 KiB).
///
/// The name may be the directory name or the frontmatter display name; the
/// filesystem directory remains the identity for path resolution.
///
/// # Errors
/// [`SkillError::NotFound`] unknown skill, [`SkillError::TooLarge`] >8 KiB,
/// [`SkillError::Invalid`]/[`SkillError::Failed`] unsafe path or IO failure.
pub fn read_workspace(workspace_root: &str, name: &str) -> Result<SkillContent, SkillError> {
    let root = workspace_skills_dir(workspace_root);
    let (dir_name, display) = resolve_named_dir(&root, name)?;
    let context = TurnContext {
        conversation_id: String::new(),
        workspace_id: String::new(),
        workspace_root: workspace_root.to_owned(),
    };
    let relative = format!(".agents/skills/{dir_name}/SKILL.md");
    let path = zk_kit_tools::safe_path(&context, &relative).map_err(|error| match error {
        ToolError::NotFound(_) => SkillError::NotFound(name.to_owned()),
        ToolError::Denied(_) => SkillError::Invalid(format!("unsafe skill path: {name}")),
        other => SkillError::Failed(other.to_string()),
    })?;
    let content = read_bounded(&path, name)?;
    Ok(SkillContent {
        name: display,
        version: None,
        content,
        source: SkillSource::Workspace,
    })
}

/// Read one installed skill (`$ZEOKEEL_HOME/skills/<name>/<version>/SKILL.md`).
///
/// When `version` is absent the latest installed version (deterministic
/// [`compare_versions`] order) is read; an uninstalled version is
/// [`SkillError::Unavailable`] and its content is never read.
///
/// # Errors
/// [`SkillError::Unavailable`] not installed, [`SkillError::NotFound`] unknown
/// name, [`SkillError::TooLarge`] >8 KiB, [`SkillError::Invalid`]/[`SkillError::Failed`]
/// unsafe path or IO failure.
pub fn read_installed(
    root: &Path,
    name: &str,
    version: Option<&str>,
) -> Result<SkillContent, SkillError> {
    let (dir_name, display) = resolve_named_dir(root, name)?;
    let dir = root.join(&dir_name);
    let versions = versions_of(&dir)?;
    let Some(latest) = versions.last() else {
        return Err(SkillError::Unavailable(format!(
            "skill {name} has no installed version"
        )));
    };
    let selected = match version {
        Some(requested) => {
            if !is_safe_component(requested) || !versions.iter().any(|entry| entry == requested) {
                return Err(SkillError::Unavailable(format!(
                    "skill {name} version {requested} is not installed"
                )));
            }
            requested.to_owned()
        }
        None => latest.clone(),
    };
    let path = zk_kit_tools::safe_resolve_under(root, &[&dir_name, &selected, "SKILL.md"])
        .map_err(|error| match error {
            ToolError::NotFound(_) => {
                SkillError::NotFound(format!("installed skill {display} is missing SKILL.md"))
            }
            ToolError::Denied(_) => SkillError::Invalid(format!(
                "installed skill path is not allowed: {dir_name}/{selected}"
            )),
            other => SkillError::Failed(other.to_string()),
        })?;
    let content = read_bounded(&path, &display)?;
    Ok(SkillContent {
        name: display,
        version: Some(selected),
        content,
        source: SkillSource::Installed,
    })
}

/// Deterministic latest-last version order (`1.0.10` > `1.0.9` > `1.0.0` >
/// `1.0.0-beta`); non-numeric versions fall back to lexicographic comparison.
#[must_use]
pub fn compare_versions(left: &str, right: &str) -> Ordering {
    match (numeric_key(left), numeric_key(right)) {
        (Some((left_numbers, left_pre)), Some((right_numbers, right_pre))) => {
            compare_numbers(&left_numbers, &right_numbers)
                .then_with(|| compare_prerelease(left_pre.as_deref(), right_pre.as_deref()))
                .then_with(|| left.cmp(right))
        }
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => left.cmp(right),
    }
}

fn numeric_key(version: &str) -> Option<(Vec<u64>, Option<String>)> {
    let (core, prerelease) = match version.split_once('-') {
        Some((core, pre)) => (core, Some(pre.to_owned())),
        None => (version, None),
    };
    if core.is_empty() {
        return None;
    }
    let mut numbers = Vec::new();
    for part in core.split('.') {
        if part.is_empty() || !part.chars().all(|character| character.is_ascii_digit()) {
            return None;
        }
        numbers.push(part.parse::<u64>().unwrap_or(u64::MAX));
    }
    Some((numbers, prerelease))
}

fn compare_numbers(left: &[u64], right: &[u64]) -> Ordering {
    let length = left.len().max(right.len());
    for index in 0..length {
        let left = left.get(index).copied().unwrap_or(0);
        let right = right.get(index).copied().unwrap_or(0);
        match left.cmp(&right) {
            Ordering::Equal => {}
            other => return other,
        }
    }
    Ordering::Equal
}

fn compare_prerelease(left: Option<&str>, right: Option<&str>) -> Ordering {
    match (left, right) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(left), Some(right)) => left.cmp(right),
    }
}

fn scan_workspace(root: &Path) -> Result<Vec<SkillEntry>, SkillError> {
    let mut skills = Vec::new();
    for name in sorted_dirs(root)? {
        let frontmatter = frontmatter_of(&root.join(&name).join("SKILL.md"));
        skills.push(SkillEntry {
            name: frontmatter.name.unwrap_or_else(|| name.clone()),
            description: frontmatter.description.unwrap_or_default(),
            user_invocable: frontmatter.user_invocable.unwrap_or(true),
            version: None,
            source: SkillSource::Workspace,
        });
    }
    skills.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(skills)
}

/// Sorted safe directory names under `root` (missing root → empty; symlinks skipped).
fn sorted_dirs(root: &Path) -> Result<Vec<String>, SkillError> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in std::fs::read_dir(root).map_err(|error| SkillError::Failed(error.to_string()))? {
        let entry = entry.map_err(|error| SkillError::Failed(error.to_string()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        if is_dir && is_safe_component(&name) {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

fn versions_of(dir: &Path) -> Result<Vec<String>, SkillError> {
    let mut versions = sorted_dirs(dir)?;
    versions.sort_by(|left, right| compare_versions(left, right));
    Ok(versions)
}

/// Resolve a display/directory name to `(directory name, display name)`.
fn resolve_named_dir(root: &Path, name: &str) -> Result<(String, String), SkillError> {
    if is_safe_component(name) && root.join(name).is_dir() {
        let frontmatter = frontmatter_of(&root.join(name).join("SKILL.md"));
        let display = frontmatter.name.unwrap_or_else(|| name.to_owned());
        return Ok((name.to_owned(), display));
    }
    for candidate in sorted_dirs(root)? {
        let frontmatter = frontmatter_of(&root.join(&candidate).join("SKILL.md"));
        let display = frontmatter.name.unwrap_or_else(|| candidate.clone());
        if display == name {
            return Ok((candidate, display));
        }
    }
    Err(SkillError::NotFound(name.to_owned()))
}

fn read_bounded(path: &Path, name: &str) -> Result<String, SkillError> {
    let metadata = std::fs::metadata(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => SkillError::NotFound(name.to_owned()),
        _ => SkillError::Failed(error.to_string()),
    })?;
    if !metadata.is_file() {
        return Err(SkillError::NotFound(name.to_owned()));
    }
    if metadata.len() > u64::try_from(MAX_SKILL_BYTES).unwrap_or(u64::MAX) {
        return Err(SkillError::TooLarge(format!(
            "SKILL.md for {name} exceeds 8 KiB ({} bytes)",
            metadata.len()
        )));
    }
    std::fs::read_to_string(path).map_err(|error| SkillError::Failed(error.to_string()))
}

/// Parsed subset of `SKILL.md` YAML frontmatter (unknown keys ignored).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Frontmatter {
    name: Option<String>,
    description: Option<String>,
    user_invocable: Option<bool>,
}

fn frontmatter_of(skill_md: &Path) -> Frontmatter {
    let Ok(mut file) = std::fs::File::open(skill_md) else {
        return Frontmatter::default();
    };
    let limit = u64::try_from(FRONTMATTER_WINDOW_BYTES).unwrap_or(u64::MAX);
    let mut bytes = Vec::new();
    let mut reader = std::io::Read::take(&mut file, limit);
    if reader.read_to_end(&mut bytes).is_err() {
        return Frontmatter::default();
    }
    parse_frontmatter(&String::from_utf8_lossy(&bytes))
}

fn parse_frontmatter(content: &str) -> Frontmatter {
    let mut lines = content.lines();
    let Some(first) = lines.next() else {
        return Frontmatter::default();
    };
    if first.trim() != "---" {
        return Frontmatter::default();
    }
    let mut frontmatter = Frontmatter::default();
    let mut block_key: Option<String> = None;
    let mut block_lines: Vec<String> = Vec::new();
    let mut block_folded = false;
    let mut closed = false;
    for line in lines {
        if line.trim() == "---" {
            closed = true;
            break;
        }
        if block_key.is_some()
            && (line.starts_with(' ') || line.starts_with('\t') || line.trim().is_empty())
        {
            block_lines.push(line.to_owned());
            continue;
        }
        if let Some(key) = block_key.take() {
            assign_block(&mut frontmatter, &key, &block_lines, block_folded);
            block_lines.clear();
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if is_block_marker(value) {
            block_key = Some(key.to_owned());
            block_lines.clear();
            block_folded = value.starts_with('>');
            continue;
        }
        assign_scalar(&mut frontmatter, key, &scalar(value));
    }
    if !closed {
        // Unterminated frontmatter: degrade to defaults instead of guessing.
        return Frontmatter::default();
    }
    if let Some(key) = block_key {
        assign_block(&mut frontmatter, &key, &block_lines, block_folded);
    }
    frontmatter
}

fn is_block_marker(value: &str) -> bool {
    matches!(value, "|" | "|-" | "|+" | ">" | ">-" | ">+")
}

fn assign_scalar(frontmatter: &mut Frontmatter, key: &str, value: &str) {
    match key {
        "name" if is_valid_display_name(value) => {
            frontmatter.name = Some(value.to_owned());
        }
        "description" if !value.is_empty() => {
            frontmatter.description = Some(value.to_owned());
        }
        "user-invocable" | "user_invocable" => {
            frontmatter.user_invocable = parse_bool(value);
        }
        _ => {}
    }
}

fn assign_block(frontmatter: &mut Frontmatter, key: &str, lines: &[String], folded: bool) {
    let value = normalize_block(lines, folded);
    if !value.is_empty() {
        assign_scalar(frontmatter, key, &value);
    }
}

fn normalize_block(lines: &[String], folded: bool) -> String {
    let indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len().saturating_sub(line.trim_start().len()))
        .min()
        .unwrap_or(0);
    let stripped: Vec<&str> = lines
        .iter()
        .map(|line| line.get(indent..).unwrap_or(""))
        .collect();
    if folded {
        let mut out = String::new();
        for line in stripped {
            let line = line.trim();
            if line.is_empty() {
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push('\n');
                }
                continue;
            }
            if !out.is_empty() && !out.ends_with('\n') {
                out.push(' ');
            }
            out.push_str(line);
        }
        out.trim().to_owned()
    } else {
        stripped.join("\n").trim().to_owned()
    }
}

fn scalar(value: &str) -> String {
    let trimmed = strip_inline_comment(value).trim();
    if trimmed.len() >= 2 {
        let first = trimmed.chars().next();
        let last = trimmed.chars().last();
        if let (Some(first), Some(last)) = (first, last) {
            if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
                let inner = trimmed
                    .get(1..trimmed.len().saturating_sub(1))
                    .unwrap_or("");
                return if first == '\'' {
                    inner.replace("''", "'")
                } else {
                    inner.to_owned()
                };
            }
        }
    }
    trimmed.to_owned()
}

/// Strip a YAML inline comment (` #...`) outside quoted scalars.
fn strip_inline_comment(value: &str) -> &str {
    let mut quote: Option<char> = None;
    let mut previous_whitespace = true;
    for (index, character) in value.char_indices() {
        match quote {
            Some(active) => {
                if character == active {
                    quote = None;
                }
            }
            None => {
                if character == '"' || character == '\'' {
                    quote = Some(character);
                } else if character == '#' && previous_whitespace {
                    return value.get(..index).unwrap_or(value);
                }
            }
        }
        previous_whitespace = character.is_whitespace();
    }
    value
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

fn is_valid_display_name(value: &str) -> bool {
    !value.is_empty() && value.chars().count() <= 128 && !value.chars().any(char::is_control)
}
