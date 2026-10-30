#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_llm_rig::{model_from, RigConfig};

#[test]
fn empty_config_is_invalid() {
    assert!(model_from(&RigConfig::default()).is_err());
}

#[test]
fn configured_model_builds() {
    let model = model_from(&RigConfig { provider: "openai".to_owned(), base_url: "http://127.0.0.1:1/v1".to_owned(), api_key: "test".to_owned(), model: "test".to_owned() }).unwrap();
    let _ = model;
}
