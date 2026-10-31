//! Generated-glue embedding configuration (personal baseline): settings override
//! installs the cap-vector model/dir without any network call; empty/masked API
//! keys are a no-op so the env fallback remains in charge.
#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use zk_contract_config::EmbeddingSettings;

#[test]
fn embedding_settings_configure_the_vector_capability() {
    zk_cap_vector::unconfigure().unwrap();
    assert!(!zk_cap_vector::available());

    // Empty/masked key: no-op (env fallback stays authoritative).
    let empty = EmbeddingSettings {
        base_url: "https://api.example.invalid/v1".to_owned(),
        model: "text-embedding-3-small".to_owned(),
        ..EmbeddingSettings::default()
    };
    zk_composed::configure_embedding_from_settings(&empty);
    assert!(!zk_cap_vector::available());

    let dir = std::env::temp_dir().join(format!("zk-embedding-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let settings = EmbeddingSettings {
        base_url: "https://api.example.invalid/v1".to_owned(),
        api_key: "test-key".to_owned(),
        model: "text-embedding-3-small".to_owned(),
        dims: Some(8),
        dir: dir.to_string_lossy().to_string(),
    };
    zk_composed::configure_embedding_from_settings(&settings);
    assert!(
        zk_cap_vector::available(),
        "configured settings must publish an embedding model: {:?}",
        zk_cap_vector::unavailable_reason()
    );
    assert_eq!(zk_cap_vector::persistence_dir(), Some(dir.clone()));

    zk_cap_vector::unconfigure().unwrap();
    assert!(!zk_cap_vector::available());
    let _ = std::fs::remove_dir_all(&dir);
}
