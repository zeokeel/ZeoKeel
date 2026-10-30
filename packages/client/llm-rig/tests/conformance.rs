#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test-only
use zk_llm_rig::{model_from, unconfigured_model, MediaConfig, MediaError, MediaKind, RigConfig};

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

fn configured() -> MediaConfig {
    MediaConfig::new("", "http://127.0.0.1:1/v1", "test-key", "")
}

#[test]
fn media_model_builders_build_openai_models() {
    let image = zk_llm_rig::image_model_from_config(&configured()).unwrap();
    assert_eq!(image.name(), "openai");
    assert_eq!(image.id(), Some("gpt-image-1"));

    let audio = zk_llm_rig::audio_model_from_config(&configured()).unwrap();
    assert_eq!(audio.name(), "openai");
    assert_eq!(audio.id(), Some("tts-1"));

    let transcription = zk_llm_rig::transcription_model_from_config(&configured()).unwrap();
    assert_eq!(transcription.name(), "openai");
    assert_eq!(transcription.id(), Some("whisper-1"));
}

#[test]
fn rerank_model_builds_from_voyageai_config() {
    let rerank = zk_llm_rig::rerank_model_from_config(&configured()).unwrap();
    assert_eq!(rerank.name(), "voyageai");
    assert_eq!(rerank.id(), Some("rerank-2.5"));
}

#[test]
fn media_models_require_api_key_and_name_the_prefix() {
    let cases: [(MediaKind, &str); 4] = [
        (MediaKind::Image, "ZEOKEEL_IMAGE_API_KEY"),
        (MediaKind::Audio, "ZEOKEEL_AUDIO_API_KEY"),
        (MediaKind::Transcription, "ZEOKEEL_TRANSCRIPTION_API_KEY"),
        (MediaKind::Rerank, "ZEOKEEL_RERANK_API_KEY"),
    ];
    for (kind, expected) in cases {
        let config = MediaConfig::default();
        let error = match kind {
            MediaKind::Image => zk_llm_rig::image_model_from_config(&config).err(),
            MediaKind::Audio => zk_llm_rig::audio_model_from_config(&config).err(),
            MediaKind::Transcription => zk_llm_rig::transcription_model_from_config(&config).err(),
            MediaKind::Rerank => zk_llm_rig::rerank_model_from_config(&config).err(),
        }
        .expect("unconfigured media model must be a typed error");
        match error {
            MediaError::Unconfigured(message) => {
                assert!(message.contains(expected), "{message} must name {expected}");
            }
            other => panic!("expected Unconfigured, got {other:?}"),
        }
    }
}

#[test]
fn media_models_reject_unknown_providers() {
    let image = zk_llm_rig::image_model_from_config(&MediaConfig::new("anthropic", "", "key", ""))
        .expect_err("unknown image provider must be rejected");
    assert!(matches!(image, MediaError::Config(_)), "{image:?}");

    // rig-core 0.45 ships no OpenAI rerank endpoint; the error must say so.
    let rerank = zk_llm_rig::rerank_model_from_config(&MediaConfig::new("openai", "", "key", ""))
        .expect_err("openai rerank must be rejected");
    match rerank {
        MediaError::Config(message) => assert!(message.contains("no rerank endpoint"), "{message}"),
        other => panic!("expected Config, got {other:?}"),
    }
}

#[test]
fn media_models_reject_non_http_base_url() {
    let error =
        zk_llm_rig::image_model_from_config(&MediaConfig::new("openai", "not-a-url", "key", ""))
            .expect_err("non-http base URL must be rejected");
    match error {
        MediaError::Config(message) => {
            assert!(message.contains("ZEOKEEL_IMAGE_BASE_URL"), "{message}")
        }
        other => panic!("expected Config, got {other:?}"),
    }
}

#[test]
fn media_env_defaults_apply_when_unset() {
    if std::env::var("ZEOKEEL_IMAGE_API_KEY").is_ok()
        || std::env::var("ZEOKEEL_RERANK_API_KEY").is_ok()
    {
        return;
    }
    let image = MediaConfig::from_env(MediaKind::Image);
    assert_eq!(image.provider, "openai");
    assert_eq!(image.base_url, "");
    assert_eq!(image.api_key, "");
    assert_eq!(image.model, "gpt-image-1");

    let rerank = MediaConfig::from_env(MediaKind::Rerank);
    assert_eq!(rerank.provider, "voyageai");
    assert_eq!(rerank.model, "rerank-2.5");

    assert!(zk_llm_rig::image_model().is_err());
    assert!(zk_llm_rig::rerank_model().is_err());
}
