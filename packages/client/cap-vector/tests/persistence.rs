#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)] // test-only

//! Persistence tests for the optional on-disk vector index: cross-instance
//! recovery, corrupt/oversized/overfull degradation, clear persistence and
//! Unix permissions. No network; the deterministic kit fake model is used.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

use zk_cap_vector::{INDEX_FILE, MAX_DOCUMENTS, MAX_INDEX_BYTES};
use zk_kit_media::test_support::fake_embedding_model;

static LOCK: Mutex<()> = Mutex::new(());
static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

fn guard() -> MutexGuard<'static, ()> {
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "zk-cap-vector-persist-{tag}-{}-{}",
        std::process::id(),
        NEXT_DIR.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn index_path(dir: &Path) -> PathBuf {
    dir.join(INDEX_FILE)
}

fn cleanup(dir: &Path) {
    let _ = fs::remove_dir_all(dir);
}

fn write_index(dir: &Path, value: &serde_json::Value) {
    fs::create_dir_all(dir).unwrap();
    fs::write(index_path(dir), serde_json::to_vec(value).unwrap()).unwrap();
}

fn read_index(dir: &Path) -> serde_json::Value {
    let bytes = fs::read(index_path(dir)).unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[test]
fn persisted_index_survives_reconfigure() {
    let _guard = guard();
    let dir = temp_dir("roundtrip");
    zk_cap_vector::configure_with_dir(fake_embedding_model(), &dir).unwrap();
    assert!(zk_cap_vector::available());
    assert_eq!(zk_cap_vector::len(), 0);
    assert!(zk_cap_vector::persistence_reason().is_none());

    zk_cap_vector::insert("doc-alpha", "alpha document").unwrap();
    zk_cap_vector::insert("doc-beta", "beta document").unwrap();
    assert_eq!(zk_cap_vector::len(), 2);
    assert!(index_path(&dir).is_file());
    assert_eq!(
        zk_cap_vector::persistence_dir().as_deref(),
        Some(dir.as_path())
    );
    // The file records the embedding-model fingerprint and vector width.
    let persisted = read_index(&dir);
    assert_eq!(persisted["version"], 1);
    assert_eq!(persisted["model"], "fake-embed");
    assert_eq!(persisted["width"], 3);

    // New store instance: drop the process-global state, then reload from disk.
    zk_cap_vector::unconfigure().unwrap();
    assert!(!zk_cap_vector::available());
    assert_eq!(zk_cap_vector::len(), 0);
    zk_cap_vector::configure_with_dir(fake_embedding_model(), &dir).unwrap();
    assert!(zk_cap_vector::available());
    assert_eq!(zk_cap_vector::len(), 2);
    let hits = zk_cap_vector::search("alpha", 1).unwrap();
    assert_eq!(hits.first().map(|hit| hit.id.as_str()), Some("doc-alpha"));

    zk_cap_vector::unconfigure().unwrap();
    cleanup(&dir);
}

#[test]
fn missing_index_starts_empty() {
    let _guard = guard();
    let dir = temp_dir("missing");
    zk_cap_vector::configure_with_dir(fake_embedding_model(), &dir).unwrap();
    assert!(zk_cap_vector::available());
    assert_eq!(zk_cap_vector::len(), 0);
    assert!(zk_cap_vector::persistence_reason().is_none());
    // Nothing is written until the first mutation.
    assert!(!index_path(&dir).exists());
    zk_cap_vector::unconfigure().unwrap();
    cleanup(&dir);
}

#[test]
fn corrupt_index_degrades_with_reason_and_is_repaired() {
    let _guard = guard();
    let dir = temp_dir("corrupt");
    fs::create_dir_all(&dir).unwrap();
    fs::write(index_path(&dir), b"{ this is not json").unwrap();

    zk_cap_vector::configure_with_dir(fake_embedding_model(), &dir).unwrap();
    assert!(zk_cap_vector::available());
    assert_eq!(zk_cap_vector::len(), 0);
    let reason = zk_cap_vector::persistence_reason().expect("reason");
    assert!(reason.contains("corrupt"), "{reason}");

    // The capability keeps working and the next successful save repairs the file.
    zk_cap_vector::insert("doc", "alpha document").unwrap();
    assert!(zk_cap_vector::persistence_reason().is_none());
    zk_cap_vector::unconfigure().unwrap();
    zk_cap_vector::configure_with_dir(fake_embedding_model(), &dir).unwrap();
    assert_eq!(zk_cap_vector::len(), 1);
    assert!(zk_cap_vector::persistence_reason().is_none());

    zk_cap_vector::unconfigure().unwrap();
    cleanup(&dir);
}

#[test]
fn clear_persists_empty_index() {
    let _guard = guard();
    let dir = temp_dir("clear");
    zk_cap_vector::configure_with_dir(fake_embedding_model(), &dir).unwrap();
    zk_cap_vector::insert("doc-alpha", "alpha document").unwrap();
    zk_cap_vector::insert("doc-beta", "beta document").unwrap();
    assert_eq!(zk_cap_vector::len(), 2);

    zk_cap_vector::clear().unwrap();
    assert_eq!(zk_cap_vector::len(), 0);
    zk_cap_vector::unconfigure().unwrap();
    zk_cap_vector::configure_with_dir(fake_embedding_model(), &dir).unwrap();
    assert_eq!(zk_cap_vector::len(), 0);
    assert!(zk_cap_vector::persistence_reason().is_none());

    zk_cap_vector::unconfigure().unwrap();
    cleanup(&dir);
}

#[test]
fn oversized_index_file_degrades() {
    let _guard = guard();
    let dir = temp_dir("oversized");
    fs::create_dir_all(&dir).unwrap();
    {
        let file = fs::File::create(index_path(&dir)).unwrap();
        // Sparse file: no 16 MB write, but a metadata size over the cap.
        file.set_len(MAX_INDEX_BYTES + 1).unwrap();
    }
    zk_cap_vector::configure_with_dir(fake_embedding_model(), &dir).unwrap();
    assert!(zk_cap_vector::available());
    assert_eq!(zk_cap_vector::len(), 0);
    let reason = zk_cap_vector::persistence_reason().expect("reason");
    assert!(reason.contains("exceeds"), "{reason}");
    zk_cap_vector::unconfigure().unwrap();
    cleanup(&dir);
}

#[test]
fn overfull_index_file_degrades() {
    let _guard = guard();
    let dir = temp_dir("overfull");
    fs::create_dir_all(&dir).unwrap();
    let documents: Vec<serde_json::Value> = (0..=MAX_DOCUMENTS)
        .map(|index| {
            serde_json::json!({
                "id": format!("doc-{index}"),
                "text": "text",
                "embedding": [1.0]
            })
        })
        .collect();
    let file = serde_json::json!({"version": 1, "documents": documents});
    fs::write(index_path(&dir), serde_json::to_vec(&file).unwrap()).unwrap();

    zk_cap_vector::configure_with_dir(fake_embedding_model(), &dir).unwrap();
    assert!(zk_cap_vector::available());
    assert_eq!(zk_cap_vector::len(), 0);
    let reason = zk_cap_vector::persistence_reason().expect("reason");
    assert!(reason.contains("more than"), "{reason}");
    zk_cap_vector::unconfigure().unwrap();
    cleanup(&dir);
}

#[cfg(unix)]
#[test]
fn unix_permissions_are_private() {
    use std::os::unix::fs::PermissionsExt;

    let _guard = guard();
    let dir = temp_dir("perm");
    zk_cap_vector::configure_with_dir(fake_embedding_model(), &dir).unwrap();
    zk_cap_vector::insert("doc", "alpha document").unwrap();

    let dir_mode = fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(dir_mode, 0o700);
    let file_mode = fs::metadata(index_path(&dir)).unwrap().permissions().mode() & 0o777;
    assert_eq!(file_mode, 0o600);

    // Atomic write leaves no temp files behind.
    let entries: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(entries, vec![INDEX_FILE.to_owned()]);

    zk_cap_vector::unconfigure().unwrap();
    cleanup(&dir);
}

#[test]
fn configure_from_env_loads_zeokeel_vector_dir() {
    let _guard = guard();
    let dir = temp_dir("env");
    // The persisted index must match the model configured from env; write it
    // directly (the fake model has a different id and would be rejected).
    write_index(
        &dir,
        &serde_json::json!({
            "version": 1,
            "model": "text-embedding-3-small",
            "width": 3,
            "documents": [{
                "id": "doc-env",
                "text": "alpha document",
                "embedding": [1.0, 0.0, 1.0]
            }]
        }),
    );
    zk_cap_vector::unconfigure().unwrap();

    std::env::set_var("ZEOKEEL_EMBEDDING_BASE_URL", "http://127.0.0.1:1/v1");
    std::env::set_var("ZEOKEEL_EMBEDDING_API_KEY", "test-key");
    std::env::set_var("ZEOKEEL_EMBEDDING_MODEL", "text-embedding-3-small");
    std::env::set_var("ZEOKEEL_VECTOR_DIR", &dir);
    let configured = zk_cap_vector::configure_from_env().unwrap();
    assert!(configured);
    assert!(zk_cap_vector::available());
    assert_eq!(zk_cap_vector::len(), 1);
    assert!(zk_cap_vector::persistence_reason().is_none());
    assert_eq!(
        zk_cap_vector::persistence_dir().as_deref(),
        Some(dir.as_path())
    );

    for key in [
        "ZEOKEEL_EMBEDDING_BASE_URL",
        "ZEOKEEL_EMBEDDING_API_KEY",
        "ZEOKEEL_EMBEDDING_MODEL",
        "ZEOKEEL_VECTOR_DIR",
    ] {
        std::env::remove_var(key);
    }
    zk_cap_vector::unconfigure().unwrap();
    cleanup(&dir);
}

#[test]
fn insert_many_is_an_atomic_batch() {
    let _guard = guard();
    let dir = temp_dir("batch");
    zk_cap_vector::configure_with_dir(fake_embedding_model(), &dir).unwrap();
    zk_cap_vector::insert("existing", "alpha document").unwrap();

    // Validation fails for the whole batch: nothing is inserted or persisted.
    let error = zk_cap_vector::insert_many([
        ("ok".to_owned(), "alpha".to_owned()),
        ("   ".to_owned(), "bad".to_owned()),
    ])
    .unwrap_err();
    assert!(matches!(error, zk_cap_vector::VectorError::Config(_)));
    assert_eq!(zk_cap_vector::len(), 1);
    let persisted = read_index(&dir);
    assert_eq!(persisted["documents"].as_array().unwrap().len(), 1);

    // A valid batch is applied and saved in one go.
    zk_cap_vector::insert_many([
        ("a".to_owned(), "alpha one".to_owned()),
        ("b".to_owned(), "beta two".to_owned()),
        ("c".to_owned(), "gamma three".to_owned()),
    ])
    .unwrap();
    assert_eq!(zk_cap_vector::len(), 4);
    zk_cap_vector::unconfigure().unwrap();
    zk_cap_vector::configure_with_dir(fake_embedding_model(), &dir).unwrap();
    assert_eq!(zk_cap_vector::len(), 4);

    zk_cap_vector::unconfigure().unwrap();
    cleanup(&dir);
}

#[test]
fn incompatible_embedding_model_degrades() {
    let _guard = guard();
    let dir = temp_dir("model-mismatch");
    write_index(
        &dir,
        &serde_json::json!({
            "version": 1,
            "model": "old-embedding-model",
            "width": 3,
            "documents": [{
                "id": "doc",
                "text": "alpha",
                "embedding": [1.0, 0.0, 1.0]
            }]
        }),
    );
    zk_cap_vector::configure_with_dir(fake_embedding_model(), &dir).unwrap();
    assert!(zk_cap_vector::available());
    assert_eq!(zk_cap_vector::len(), 0);
    let reason = zk_cap_vector::persistence_reason().expect("reason");
    assert!(reason.contains("embedding model"), "{reason}");
    zk_cap_vector::unconfigure().unwrap();
    cleanup(&dir);
}

#[test]
fn incompatible_dimensions_degrade() {
    let _guard = guard();
    let dir = temp_dir("dims-mismatch");
    let current = zk_kit_media::embedding_model_from_config(&zk_kit_media::EmbeddingConfig::new(
        "http://127.0.0.1:1/v1",
        "test-key",
        "text-embedding-3-small",
        Some(4),
    ))
    .unwrap();
    write_index(
        &dir,
        &serde_json::json!({
            "version": 1,
            "model": "text-embedding-3-small",
            "dims": 8,
            "width": 8,
            "documents": [{
                "id": "doc",
                "text": "alpha",
                "embedding": [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0]
            }]
        }),
    );
    zk_cap_vector::configure_with_dir(current, &dir).unwrap();
    assert!(zk_cap_vector::available());
    assert_eq!(zk_cap_vector::len(), 0);
    let reason = zk_cap_vector::persistence_reason().expect("reason");
    assert!(reason.contains("incompatible"), "{reason}");
    zk_cap_vector::unconfigure().unwrap();
    cleanup(&dir);
}

#[test]
fn legacy_index_without_fingerprint_loads() {
    let _guard = guard();
    let dir = temp_dir("legacy");
    write_index(
        &dir,
        &serde_json::json!({
            "version": 1,
            "documents": [{
                "id": "doc",
                "text": "alpha document",
                "embedding": [1.0, 0.0]
            }]
        }),
    );
    zk_cap_vector::configure_with_dir(fake_embedding_model(), &dir).unwrap();
    assert_eq!(zk_cap_vector::len(), 1);
    assert!(zk_cap_vector::persistence_reason().is_none());
    zk_cap_vector::unconfigure().unwrap();
    cleanup(&dir);
}

#[test]
fn inconsistent_vector_widths_degrade() {
    let _guard = guard();
    let dir = temp_dir("widths");
    write_index(
        &dir,
        &serde_json::json!({
            "version": 1,
            "documents": [
                {"id": "a", "text": "a", "embedding": [1.0, 0.0]},
                {"id": "b", "text": "b", "embedding": [1.0, 0.0, 1.0]}
            ]
        }),
    );
    zk_cap_vector::configure_with_dir(fake_embedding_model(), &dir).unwrap();
    assert_eq!(zk_cap_vector::len(), 0);
    let reason = zk_cap_vector::persistence_reason().expect("reason");
    assert!(reason.contains("inconsistent"), "{reason}");
    zk_cap_vector::unconfigure().unwrap();
    cleanup(&dir);
}

#[cfg(unix)]
#[test]
fn pre_existing_directory_is_tightened_to_0700() {
    use std::os::unix::fs::PermissionsExt;

    let _guard = guard();
    let dir = temp_dir("tighten");
    fs::create_dir_all(&dir).unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    zk_cap_vector::configure_with_dir(fake_embedding_model(), &dir).unwrap();
    let mode = fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o700);
    zk_cap_vector::unconfigure().unwrap();
    cleanup(&dir);
}
