#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
//! `tool-skill` conformance: legacy text compatibility, frontmatter JSON,
//! the 8 KiB bound, installed version selection and path safety.

use std::path::{Path, PathBuf};

use zk_contract_tool::{Tool, TurnContext};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "zk-skill-{tag}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn turn(workspace: &Path) -> TurnContext {
    TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: workspace.display().to_string(),
    }
}

fn write_skill(workspace: &Path, name: &str, body: &str) {
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
fn legacy_text_list_stays_newline_directory_names() {
    let workspace = temp_dir("legacy");
    let tool = zk_tool_skill::tool();
    let context = turn(&workspace);
    // Missing `.agents/skills`: unchanged legacy marker.
    assert_eq!(
        tool.call_json(&context, serde_json::json!({"action":"list"}))
            .unwrap(),
        "no skills"
    );
    write_skill(&workspace, "zeta", "# zeta\n");
    write_skill(&workspace, "alpha", "# alpha\n");
    std::fs::create_dir_all(workspace.join(".agents/skills/not-a-skill-file")).unwrap();
    let text = tool
        .call_json(&context, serde_json::json!({"action":"list"}))
        .unwrap();
    assert_eq!(text, "alpha\nnot-a-skill-file\nzeta");
    std::fs::remove_dir_all(&workspace).unwrap();
}

#[test]
fn json_list_parses_frontmatter_with_graceful_defaults() {
    let workspace = temp_dir("json");
    let tool = zk_tool_skill::tool();
    let context = turn(&workspace);
    write_skill(
        &workspace,
        "demo",
        "---\nname: demo-skill\ndescription: \"Demo: quoted # hash\"\nuser-invocable: false\n---\n# body\n",
    );
    write_skill(&workspace, "no-frontmatter", "just a body\n");
    write_skill(
        &workspace,
        "broken",
        "---\nname: [unterminated\nuser-invocable: false\n",
    );
    write_skill(
        &workspace,
        "folded",
        "---\ndescription: >-\n  folded line one\n  folded line two\n---\n",
    );
    let raw = tool
        .call_json(
            &context,
            serde_json::json!({"action":"list","format":"json"}),
        )
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let skills = parsed
        .get("skills")
        .and_then(serde_json::Value::as_array)
        .unwrap();
    assert_eq!(skills.len(), 4);
    // Sorted by display name: broken, demo-skill, folded, no-frontmatter.
    let first = skills.first().unwrap();
    assert_eq!(first["name"], "broken");
    // Unterminated frontmatter degrades to the directory name (no panic/fake data).
    assert_eq!(first["description"], "");
    assert_eq!(first["user_invocable"], true);
    let second = skills.get(1).unwrap();
    assert_eq!(second["name"], "demo-skill");
    assert_eq!(second["description"], "Demo: quoted # hash");
    assert_eq!(second["user_invocable"], false);
    assert_eq!(
        skills.get(2).unwrap()["description"],
        "folded line one folded line two"
    );
    let fourth = skills.get(3).unwrap();
    assert_eq!(fourth["name"], "no-frontmatter");
    assert_eq!(fourth["description"], "");
    // Invalid format is rejected (no silent fallback).
    assert!(tool
        .call_json(
            &context,
            serde_json::json!({"action":"list","format":"yaml"})
        )
        .is_err());
    std::fs::remove_dir_all(&workspace).unwrap();
}

#[test]
fn read_returns_content_and_rejects_missing_unsafe_and_oversized() {
    let workspace = temp_dir("read");
    let tool = zk_tool_skill::tool();
    let context = turn(&workspace);
    write_skill(&workspace, "demo", "# Demo instructions\n");
    assert_eq!(
        tool.call_json(&context, serde_json::json!({"action":"read","name":"demo"}))
            .unwrap(),
        "# Demo instructions\n"
    );
    // Frontmatter display names resolve back to the directory (single scanner).
    write_skill(&workspace, "dir-name", "---\nname: display-name\n---\nx\n");
    assert_eq!(
        tool.call_json(
            &context,
            serde_json::json!({"action":"read","name":"display-name"})
        )
        .unwrap(),
        "---\nname: display-name\n---\nx\n"
    );
    assert!(tool
        .call_json(
            &context,
            serde_json::json!({"action":"read","name":"missing"})
        )
        .is_err());
    assert!(tool
        .call_json(
            &context,
            serde_json::json!({"action":"read","name":"../escape"})
        )
        .is_err());
    // 8 KiB + 1 byte is rejected with the stable `skill/too-large` code.
    write_skill(
        &workspace,
        "big",
        &"x".repeat(zk_tool_skill::MAX_SKILL_BYTES + 1),
    );
    let error = tool
        .call_json(&context, serde_json::json!({"action":"read","name":"big"}))
        .unwrap_err()
        .to_string();
    assert!(error.contains("skill/too-large"), "{error}");
    // Exactly 8 KiB is allowed (bound is inclusive).
    write_skill(
        &workspace,
        "limit",
        &"y".repeat(zk_tool_skill::MAX_SKILL_BYTES),
    );
    assert_eq!(
        tool.call_json(
            &context,
            serde_json::json!({"action":"read","name":"limit"})
        )
        .unwrap()
        .len(),
        zk_tool_skill::MAX_SKILL_BYTES
    );
    std::fs::remove_dir_all(&workspace).unwrap();
}

#[test]
fn installed_list_uses_latest_version_and_user_invocable() {
    let root = temp_dir("installed-list");
    write_installed(
        &root,
        "demo",
        "1.0.0",
        "---\ndescription: old\n---\nold body\n",
    );
    write_installed(
        &root,
        "demo",
        "1.0.10",
        "---\ndescription: newest\nuser-invocable: true\n---\nnew body\n",
    );
    write_installed(
        &root,
        "demo",
        "1.0.9",
        "---\ndescription: middle\n---\nmiddle body\n",
    );
    write_installed(
        &root,
        "hidden",
        "2.0.0",
        "---\nuser-invocable: false\n---\n",
    );
    let skills = zk_tool_skill::list_installed(&root).unwrap();
    assert_eq!(skills.len(), 2);
    let first = skills.first().unwrap();
    assert_eq!(first.name, "demo");
    assert_eq!(first.description, "newest");
    assert_eq!(first.version.as_deref(), Some("1.0.10"));
    assert!(first.user_invocable);
    assert!(!skills.get(1).unwrap().user_invocable);
    // Missing root is an honest empty list (enterprise not installed).
    assert!(zk_tool_skill::list_installed(&root.join("absent"))
        .unwrap()
        .is_empty());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn installed_read_selects_latest_or_explicit_version_and_never_reads_uninstalled() {
    let root = temp_dir("installed-read");
    write_installed(&root, "demo", "1.0.0", "v1\n");
    write_installed(&root, "demo", "1.0.10", "v2\n");
    let latest = zk_tool_skill::read_installed(&root, "demo", None).unwrap();
    assert_eq!(latest.content, "v2\n");
    assert_eq!(latest.version.as_deref(), Some("1.0.10"));
    assert_eq!(latest.source, zk_tool_skill::SkillSource::Installed);
    let pinned = zk_tool_skill::read_installed(&root, "demo", Some("1.0.0")).unwrap();
    assert_eq!(pinned.content, "v1\n");
    // Uninstalled version / unknown name / unsafe components are unavailable/not-found.
    let error = zk_tool_skill::read_installed(&root, "demo", Some("9.9.9")).unwrap_err();
    assert!(error.to_string().contains("skill/unavailable"), "{error}");
    let unknown = zk_tool_skill::read_installed(&root, "missing", None).unwrap_err();
    assert!(unknown.to_string().contains("skill/not-found"), "{unknown}");
    let unsafe_name = zk_tool_skill::read_installed(&root, "../demo", None).unwrap_err();
    assert!(
        unsafe_name.to_string().contains("skill/not-found"),
        "{unsafe_name}"
    );
    let unsafe_version =
        zk_tool_skill::read_installed(&root, "demo", Some("../1.0.0")).unwrap_err();
    assert!(
        unsafe_version.to_string().contains("skill/unavailable"),
        "{unsafe_version}"
    );
    // Oversized installed skill content is rejected.
    write_installed(
        &root,
        "big",
        "1.0.0",
        &"z".repeat(zk_tool_skill::MAX_SKILL_BYTES + 1),
    );
    let too_large = zk_tool_skill::read_installed(&root, "big", None).unwrap_err();
    assert!(
        too_large.to_string().contains("skill/too-large"),
        "{too_large}"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[cfg(unix)]
#[test]
fn installed_read_rejects_symlink_escape() {
    let outside = temp_dir("outside");
    std::fs::write(outside.join("SKILL.md"), "secret\n").unwrap();
    let root = temp_dir("symlink");
    let version_dir = root.join("demo").join("1.0.0");
    std::fs::create_dir_all(&version_dir).unwrap();
    // A real installed directory whose SKILL.md escapes through a symlink is rejected.
    std::os::unix::fs::symlink(outside.join("SKILL.md"), version_dir.join("SKILL.md")).unwrap();
    let error = zk_tool_skill::read_installed(&root, "demo", None).unwrap_err();
    assert!(error.to_string().contains("skill/invalid"), "{error}");
    // A symlinked version directory is never treated as installed (no outside read).
    let symlinked = temp_dir("symlink-version");
    std::fs::create_dir_all(symlinked.join("pkg")).unwrap();
    std::os::unix::fs::symlink(&outside, symlinked.join("pkg").join("1.0.0")).unwrap();
    let error = zk_tool_skill::read_installed(&symlinked, "pkg", None).unwrap_err();
    assert!(error.to_string().contains("skill/unavailable"), "{error}");
    std::fs::remove_dir_all(&root).unwrap();
    std::fs::remove_dir_all(&symlinked).unwrap();
    std::fs::remove_dir_all(&outside).unwrap();
}

#[test]
fn package_component_validation_matches_store_installs() {
    for safe in ["demo", "demo-skill_2.0", "1.0.0"] {
        assert!(zk_tool_skill::is_safe_component(safe), "{safe}");
    }
    for unsafe_value in ["", ".", "..", ".hidden", "C:evil", "a/b", "a\\b", "a b"] {
        assert!(
            !zk_tool_skill::is_safe_component(unsafe_value),
            "{unsafe_value}"
        );
    }
}

#[test]
fn version_order_is_numeric_then_prerelease() {
    use std::cmp::Ordering;
    let ordered = [
        "1.0.0-beta",
        "1.0.0",
        "1.0.1",
        "1.0.9",
        "1.0.10",
        "1.1.0",
        "2.0.0",
    ];
    for pair in ordered.windows(2) {
        let (Some(left), Some(right)) = (pair.first(), pair.get(1)) else {
            continue;
        };
        assert_eq!(
            zk_tool_skill::compare_versions(left, right),
            Ordering::Less,
            "{left} < {right}"
        );
        assert_eq!(
            zk_tool_skill::compare_versions(right, left),
            Ordering::Greater,
            "{right} > {left}"
        );
    }
    assert_eq!(
        zk_tool_skill::compare_versions("1.0", "1.0.0"),
        Ordering::Less,
        "raw tie-break keeps a total order"
    );
    // Non-numeric versions still sort deterministically.
    assert_eq!(zk_tool_skill::compare_versions("v1", "v2"), Ordering::Less);
    let mut unordered = vec!["v2".to_owned(), "1.0.0".to_owned(), "v1".to_owned()];
    unordered.sort_by(|left, right| zk_tool_skill::compare_versions(left, right));
    assert_eq!(unordered, vec!["v1", "v2", "1.0.0"]);
}
