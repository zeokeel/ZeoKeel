#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test-only
//! Unit tests for the media kit: config redaction, builder errors, model-slot
//! lifecycle and the deterministic in-process models.

use crate::test_support::{
    fake_audio_model, fake_embedding_model, fake_image_model, fake_rerank_model,
    fake_transcription_model, FAKE_PROVIDER,
};
use crate::{
    audio_model_from_config, block_on, embed_text, embedding_model_from_config, generate_image,
    image_model, image_model_from_config, rerank_documents, rerank_model_from_config,
    synthesize_audio, transcribe_audio, transcription_model_from_config, EmbeddingConfig,
    EmbeddingError, MediaConfig, MediaError, MediaKind, ModelSlot,
};

fn configured() -> MediaConfig {
    MediaConfig::new("", "http://127.0.0.1:1/v1", "test-key", "")
}

#[test]
fn debug_redacts_api_key() {
    let config = MediaConfig::new("openai", "https://api.openai.com/v1", "sk-super-secret", "");
    let rendered = format!("{config:?}");
    assert!(!rendered.contains("sk-super-secret"), "{rendered}");
    assert!(rendered.contains("<redacted>"), "{rendered}");
    assert!(rendered.contains("https://api.openai.com/v1"), "{rendered}");

    let unset = format!("{:?}", MediaConfig::default());
    assert!(unset.contains("<unset>"), "{unset}");
}

#[test]
fn media_config_defaults_apply_when_env_unset() {
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
    assert!(image_model().is_err());
}

#[test]
fn media_kinds_name_their_env_prefix() {
    assert_eq!(MediaKind::Image.env_prefix(), "ZEOKEEL_IMAGE");
    assert_eq!(MediaKind::Audio.env_prefix(), "ZEOKEEL_AUDIO");
    assert_eq!(
        MediaKind::Transcription.env_prefix(),
        "ZEOKEEL_TRANSCRIPTION"
    );
    assert_eq!(MediaKind::Rerank.env_prefix(), "ZEOKEEL_RERANK");
    assert!(MediaKind::Image
        .unavailable_reason()
        .contains("ZEOKEEL_IMAGE_API_KEY"));
}

#[test]
fn builders_require_api_key_and_name_the_prefix() {
    let cases: [(MediaKind, &str); 4] = [
        (MediaKind::Image, "ZEOKEEL_IMAGE_API_KEY"),
        (MediaKind::Audio, "ZEOKEEL_AUDIO_API_KEY"),
        (MediaKind::Transcription, "ZEOKEEL_TRANSCRIPTION_API_KEY"),
        (MediaKind::Rerank, "ZEOKEEL_RERANK_API_KEY"),
    ];
    for (kind, expected) in cases {
        let config = MediaConfig::default();
        let error = match kind {
            MediaKind::Image => image_model_from_config(&config).err(),
            MediaKind::Audio => audio_model_from_config(&config).err(),
            MediaKind::Transcription => transcription_model_from_config(&config).err(),
            MediaKind::Rerank => rerank_model_from_config(&config).err(),
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
fn builders_reject_unknown_providers() {
    let image = image_model_from_config(&MediaConfig::new("anthropic", "", "key", ""))
        .expect_err("unknown image provider must be rejected");
    assert!(matches!(image, MediaError::Config(_)), "{image:?}");

    let rerank = rerank_model_from_config(&MediaConfig::new("openai", "", "key", ""))
        .expect_err("openai rerank must be rejected");
    match rerank {
        MediaError::Config(message) => assert!(message.contains("no rerank endpoint"), "{message}"),
        other => panic!("expected Config, got {other:?}"),
    }
}

#[test]
fn builders_reject_non_http_base_url() {
    let error = image_model_from_config(&MediaConfig::new("openai", "not-a-url", "key", ""))
        .expect_err("non-http base URL must be rejected");
    match error {
        MediaError::Config(message) => {
            assert!(message.contains("ZEOKEEL_IMAGE_BASE_URL"), "{message}");
        }
        other => panic!("expected Config, got {other:?}"),
    }
}

#[test]
fn fake_models_round_trip_requests_and_replies() {
    let image = generate_image(&fake_image_model(), "a mountain lake", 64, 32).unwrap();
    assert_eq!(image.provider, FAKE_PROVIDER);
    assert_eq!(image.model.as_deref(), Some("fake-image-model"));
    assert!(image.bytes.starts_with(&[0x89, b'P', b'N', b'G']));
    assert!(
        image
            .bytes
            .windows(b"a mountain lake:64x32".len())
            .any(|window| window == b"a mountain lake:64x32"),
        "request facts missing from {:?}",
        String::from_utf8_lossy(&image.bytes)
    );

    let audio = synthesize_audio(&fake_audio_model(), "hello world", "alloy").unwrap();
    assert_eq!(audio.provider, FAKE_PROVIDER);
    assert_eq!(audio.bytes, b"hello world|alloy\0\x01".to_vec());

    let transcript = transcribe_audio(&fake_transcription_model(), b"hello audio", "clip.wav");
    let transcript = transcript.unwrap();
    assert_eq!(transcript.text, "clip.wav:hello audio");
    assert_eq!(transcript.language.as_deref(), Some("en"));
    assert_eq!(transcript.duration, Some(1.5));

    let documents = vec!["python is slower".to_owned(), "rust is fast".to_owned()];
    let hits = rerank_documents(&fake_rerank_model(), "rust", &documents).unwrap();
    assert_eq!(hits.first().map(|hit| hit.index), Some(1));
    assert_eq!(hits.first().map(|hit| hit.score), Some(1.0));
}

#[test]
fn model_slot_lifecycle_is_dynamic() {
    static SLOT: ModelSlot<crate::ImageModel> = ModelSlot::new();
    assert!(!SLOT.is_configured());
    assert!(SLOT.get().unwrap().is_none());
    SLOT.set(fake_image_model()).unwrap();
    assert!(SLOT.is_configured());
    assert!(SLOT.get().unwrap().is_some());
    SLOT.clear().unwrap();
    assert!(!SLOT.is_configured());
    assert!(SLOT.get().unwrap().is_none());
}

#[test]
fn block_on_runs_a_future() {
    assert_eq!(block_on(async { 42 }).unwrap(), 42);
}

#[test]
fn configured_builders_build_handles() {
    // Configuring is enough to build the handle; no request is sent.
    let _ = image_model_from_config(&configured()).unwrap();
    let _ = audio_model_from_config(&configured()).unwrap();
    let _ = transcription_model_from_config(&configured()).unwrap();
    let _ = rerank_model_from_config(&configured()).unwrap();
}

#[test]
fn embedding_config_debug_redacts_api_key() {
    let config = EmbeddingConfig::new(
        "https://api.openai.com/v1",
        "sk-embedding-secret",
        "text-embedding-3-small",
        Some(256),
    );
    let rendered = format!("{config:?}");
    assert!(!rendered.contains("sk-embedding-secret"), "{rendered}");
    assert!(rendered.contains("<redacted>"), "{rendered}");
    assert!(rendered.contains("Some(256)"), "{rendered}");
}

#[test]
fn embedding_config_requires_endpoint_key_and_model() {
    for config in [
        EmbeddingConfig::default(),
        EmbeddingConfig::new("", "key", "model", None),
        EmbeddingConfig::new("http://127.0.0.1:1/v1", "", "model", None),
        EmbeddingConfig::new("http://127.0.0.1:1/v1", "key", "", None),
    ] {
        let error = embedding_model_from_config(&config)
            .expect_err("incomplete embedding config must be a typed error");
        match error {
            EmbeddingError::Unconfigured(message) => {
                assert!(message.contains("ZEOKEEL_EMBEDDING_API_KEY"), "{message}");
            }
            other => panic!("expected Unconfigured, got {other:?}"),
        }
    }
    assert!(
        embedding_model_from_config(&EmbeddingConfig::new("not-a-url", "key", "model", None,))
            .is_err()
    );
}

#[test]
fn embedding_model_exposes_id_and_dims_for_fingerprints() {
    let config = EmbeddingConfig::new(
        "http://127.0.0.1:1/v1",
        "test-key",
        "text-embedding-3-small",
        Some(256),
    );
    let model = embedding_model_from_config(&config).unwrap();
    assert_eq!(model.id(), Some("text-embedding-3-small"));
    assert_eq!(model.dims(), Some(256));

    let fake = fake_embedding_model();
    assert_eq!(fake.id(), Some("fake-embed"));
    assert_eq!(fake.dims(), None);
}

#[test]
fn fake_embedding_model_round_trips_text() {
    let alpha = embed_text(&fake_embedding_model(), "alpha document").unwrap();
    assert_eq!(alpha, vec![1.0, 0.0, 1.0]);
    let beta = embed_text(&fake_embedding_model(), "beta document").unwrap();
    assert_eq!(beta, vec![0.0, 1.0, 1.0]);
    let gamma = embed_text(&fake_embedding_model(), "gamma document").unwrap();
    assert_eq!(gamma, vec![0.0, 0.0, 1.0]);
}

#[test]
fn embedding_config_from_env_is_optional() {
    if std::env::var("ZEOKEEL_EMBEDDING_BASE_URL").is_ok()
        || std::env::var("ZEOKEEL_EMBEDDING_API_KEY").is_ok()
        || std::env::var("ZEOKEEL_EMBEDDING_MODEL").is_ok()
    {
        return;
    }
    let config = EmbeddingConfig::from_env().unwrap();
    assert!(!config.is_configured());
    assert_eq!(config.base_url, "");
    assert_eq!(config.dims, None);
}
