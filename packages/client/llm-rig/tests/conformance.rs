#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_llm_rig::{model_from, unconfigured_model, RigConfig};

#[test]
fn empty_config_is_invalid() {
    assert!(model_from(&RigConfig::default()).is_err());
}

#[test]
fn unconfigured_fallback_model_builds() {
    // ADR-064: composed desktop must boot before provider settings are applied.
    let model = unconfigured_model().unwrap();
    let _ = model;
}

#[test]
fn configured_model_builds() {
    let model = model_from(&RigConfig {
        provider: "openai".to_owned(),
        base_url: "http://127.0.0.1:1/v1".to_owned(),
        api_key: "test".to_owned(),
        model: "test".to_owned(),
    })
    .unwrap();
    let _ = model;
}

#[test]
fn embedding_model_builds_from_explicit_config() {
    let model = zk_llm_rig::embedding_model_from_config(
        "http://127.0.0.1:1/v1",
        "test-key",
        "text-embedding-3-small",
        Some(256),
    )
    .unwrap();
    assert_eq!(model.id(), Some("text-embedding-3-small"));
}

#[test]
fn embedding_model_requires_provider_config() {
    assert!(zk_llm_rig::embedding_model_from_config("", "key", "model", None).is_err());
    assert!(
        zk_llm_rig::embedding_model_from_config("http://127.0.0.1:1/v1", "", "model", None)
            .is_err()
    );
    assert!(
        zk_llm_rig::embedding_model_from_config("http://127.0.0.1:1/v1", "key", "", None).is_err()
    );
}
