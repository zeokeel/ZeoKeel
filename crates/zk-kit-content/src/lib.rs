//! Shared content/archive hash helpers (single implementation).
//!
//! @capability kit.content
#![forbid(unsafe_code)]
use sha2::{Digest, Sha256};

/// Reject absolute paths, `..`, NUL, backslashes and empty segments.
#[must_use]
pub fn is_safe_archive_path(path: &str) -> bool {
    if path.is_empty()
        || path.starts_with('/')
        || path.starts_with('\\')
        || path.contains('\\')
        || path.contains(':')
        || path.contains('\0')
    {
        return false;
    }
    path.split('/')
        .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

/// Canonical archive tree hash: sorted UTF-8 paths, `path NUL size NUL content`.
///
/// Directory entries are omitted by callers; only files participate.
#[must_use]
pub fn tree_sha256(mut entries: Vec<(String, Vec<u8>)>) -> String {
    entries.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    let mut hasher = Sha256::new();
    for (path, content) in entries {
        hasher.update(path.as_bytes());
        hasher.update([0_u8]);
        hasher.update(content.len().to_le_bytes());
        hasher.update([0_u8]);
        hasher.update(content);
    }
    format!("{:x}", hasher.finalize())
}
