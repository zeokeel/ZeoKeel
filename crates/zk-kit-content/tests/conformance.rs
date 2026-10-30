#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_kit_content::{is_safe_archive_path, tree_sha256};

#[test]
fn rejects_unsafe_paths() {
    for path in [
        "../a",
        "/abs",
        "a\\b",
        "a//b",
        "",
        "a/../b",
        "a\0b",
        "C:/evil",
        "C:evil",
        "//server/share",
    ] {
        assert!(!is_safe_archive_path(path), "{path:?}");
    }
    assert!(is_safe_archive_path("SKILL.md"));
    assert!(is_safe_archive_path("nested/file.txt"));
}

#[test]
fn tree_hash_is_order_independent_and_matches_golden() {
    let a = vec![
        ("b.txt".to_owned(), b"B".to_vec()),
        ("a.txt".to_owned(), b"A".to_vec()),
    ];
    let b = vec![
        ("a.txt".to_owned(), b"A".to_vec()),
        ("b.txt".to_owned(), b"B".to_vec()),
    ];
    assert_eq!(tree_sha256(a), tree_sha256(b));
    // Golden is stable; computed once and pinned to catch algorithm drift.
    let golden = tree_sha256(vec![("a.txt".to_owned(), b"A".to_vec())]);
    assert_eq!(
        golden,
        "e38ea95356341478c9fe9c2ef54674e4f75d321a5302532709405004f88e98fb"
    );
}
