#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test-only
//! Audio capability tests with the kit's deterministic in-process model (no
//! network, no provider): text/voice reach the model and audio bytes/provider
//! come back.

use std::sync::{Mutex, MutexGuard};

use zk_cap_audio::{AudioError, GeneratedAudio};
use zk_kit_media::test_support::{fake_audio_model, FAKE_PROVIDER};
use zk_kit_media::MediaConfig;

static LOCK: Mutex<()> = Mutex::new(());

fn guard() -> MutexGuard<'static, ()> {
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[test]
fn configured_capability_synthesizes_audio() {
    let _guard = guard();
    zk_cap_audio::configure(fake_audio_model()).unwrap();
    assert!(zk_cap_audio::available());
    assert_eq!(zk_cap_audio::unavailable_reason(), None);

    let generated: GeneratedAudio = zk_cap_audio::synthesize("hello world", "alloy").unwrap();
    assert_eq!(generated.provider, FAKE_PROVIDER);
    assert_eq!(generated.model.as_deref(), Some("fake-tts-model"));
    assert_eq!(generated.bytes, b"hello world|alloy\0\x01".to_vec());

    zk_cap_audio::unconfigure().unwrap();
    assert!(!zk_cap_audio::available());
    assert!(zk_cap_audio::unavailable_reason().is_some());
}

#[test]
fn empty_text_or_voice_is_rejected_before_the_model() {
    let _guard = guard();
    zk_cap_audio::configure(fake_audio_model()).unwrap();
    match zk_cap_audio::synthesize("   ", "alloy").unwrap_err() {
        AudioError::Config(message) => assert!(message.contains("text is empty"), "{message}"),
        other => panic!("expected Config, got {other:?}"),
    }
    match zk_cap_audio::synthesize("hello", "  ").unwrap_err() {
        AudioError::Config(message) => assert!(message.contains("voice is empty"), "{message}"),
        other => panic!("expected Config, got {other:?}"),
    }
    zk_cap_audio::unconfigure().unwrap();
}

#[test]
fn configure_from_env_is_optional() {
    let _guard = guard();
    if std::env::var("ZEOKEEL_AUDIO_API_KEY").is_ok() {
        return;
    }
    zk_cap_audio::unconfigure().unwrap();
    assert!(!zk_cap_audio::configure_from_env().unwrap());
    assert!(!zk_cap_audio::available());
}

#[test]
fn configure_from_config_maps_model_errors() {
    let _guard = guard();
    let error = zk_cap_audio::configure_from_config(&MediaConfig::default())
        .expect_err("unconfigured media config must fail");
    match error {
        AudioError::Unavailable(message) => {
            assert!(message.contains("ZEOKEEL_AUDIO_API_KEY"), "{message}");
        }
        other => panic!("expected Unavailable, got {other:?}"),
    }
    assert!(!zk_cap_audio::available());
}
