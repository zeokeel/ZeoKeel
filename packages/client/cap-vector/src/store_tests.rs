#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test-only
//! Unit tests for persistence caps that are impractical to trigger through the
//! public API (they would require ~10k inserted documents / a ~16 MB index).

use zk_kit_media::test_support::fake_embedding_model;

use super::{
    configure_with_dir, insert, len, save_documents, store, unconfigure, Fingerprint,
    StoredDocument, VectorError, INDEX_FILE, MAX_DOCUMENTS,
};

fn document(index: usize) -> StoredDocument {
    StoredDocument {
        id: format!("doc-{index}"),
        text: format!("document {index}"),
        vector: vec![1.0],
    }
}

#[test]
fn save_rejects_more_than_max_documents() {
    let documents: Vec<StoredDocument> = (0..=MAX_DOCUMENTS).map(document).collect();
    match save_documents(None, &documents, &Fingerprint::default()).unwrap_err() {
        VectorError::Capacity(message) => {
            assert!(message.contains(&MAX_DOCUMENTS.to_string()), "{message}");
        }
        other => panic!("expected Capacity, got {other:?}"),
    }
}

#[test]
fn save_rejects_index_over_byte_cap() {
    let dir = std::env::temp_dir().join(format!(
        "zk-cap-vector-cap-{}-{}",
        std::process::id(),
        INDEX_FILE
    ));
    let documents = vec![StoredDocument {
        id: "big".to_owned(),
        // Comfortably over MAX_INDEX_BYTES once serialized.
        text: "x".repeat(17_000_000),
        vector: vec![1.0],
    }];
    match save_documents(Some(&dir), &documents, &Fingerprint::default()).unwrap_err() {
        VectorError::Capacity(message) => assert!(message.contains("exceed"), "{message}"),
        other => panic!("expected Capacity, got {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn full_index_is_a_typed_capacity_error_not_a_panic() {
    configure_with_dir(fake_embedding_model(), "").unwrap();
    {
        let mut state = store().write().unwrap();
        state.documents = (0..MAX_DOCUMENTS).map(document).collect();
    }
    match insert("extra", "text").unwrap_err() {
        VectorError::Capacity(message) => {
            assert!(message.contains(&MAX_DOCUMENTS.to_string()), "{message}");
        }
        other => panic!("expected Capacity, got {other:?}"),
    }
    assert_eq!(len(), MAX_DOCUMENTS);
    unconfigure().unwrap();
}
