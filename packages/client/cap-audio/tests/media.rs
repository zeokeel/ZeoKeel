#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test-only
//! Audio capability tests with a deterministic in-process model (rig
//! `Local<AudioGeneration>` wire + fake transport): text/voice reach the model
//! and audio bytes/provider come back without any network or provider.

use std::sync::{Mutex, MutexGuard};

use futures::stream;
use rig_core::audio_generation::{AudioGenerationRequest, AudioGenerationResponse};
use rig_core::driver::{Exchange, Local, Model, Opened, Opening, Step, Transport};
use rig_core::operation::AudioGeneration;
use rig_core::DynModel;
use zk_cap_audio::{AudioError, GeneratedAudio};

static LOCK: Mutex<()> = Mutex::new(());

fn guard() -> MutexGuard<'static, ()> {
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[derive(Clone)]
struct FakeAudioTransport;

impl Transport<Local<AudioGeneration>> for FakeAudioTransport {
    fn send(
        &self,
        payload: AudioGenerationRequest,
        _exchange: Exchange,
    ) -> Opening<Step<AudioGeneration>> {
        // Echo text+voice so the test proves the request reached the model.
        let mut audio = format!("{}|{}", payload.text, payload.voice).into_bytes();
        audio.extend_from_slice(&[0x00, 0x01]);
        let mut response = AudioGenerationResponse::new(audio);
        response.model = Some("fake-tts-model".to_owned());
        Opening::ready(Opened::new(stream::once(
            async move { Ok(Step::End(response)) },
        )))
    }
}

fn fake_model() -> DynModel<AudioGeneration> {
    Model::new(
        Local::<AudioGeneration>::new("fake-provider"),
        FakeAudioTransport,
    )
    .into()
}

#[test]
fn configured_capability_synthesizes_audio() {
    let _guard = guard();
    zk_cap_audio::configure(fake_model()).unwrap();
    assert!(zk_cap_audio::available());
    assert_eq!(zk_cap_audio::unavailable_reason(), None);

    let generated: GeneratedAudio = zk_cap_audio::synthesize("hello world", "alloy").unwrap();
    assert_eq!(generated.provider, "fake-provider");
    assert_eq!(generated.model.as_deref(), Some("fake-tts-model"));
    assert_eq!(generated.bytes, b"hello world|alloy\0\x01".to_vec());

    zk_cap_audio::unconfigure().unwrap();
    assert!(!zk_cap_audio::available());
    assert!(zk_cap_audio::unavailable_reason().is_some());
}

#[test]
fn empty_text_or_voice_is_rejected_before_the_model() {
    let _guard = guard();
    zk_cap_audio::configure(fake_model()).unwrap();
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
    let error = zk_cap_audio::configure_from_config(&zk_llm_rig::MediaConfig::default())
        .expect_err("unconfigured media config must fail");
    match error {
        AudioError::Unavailable(message) => {
            assert!(message.contains("ZEOKEEL_AUDIO_API_KEY"), "{message}");
        }
        other => panic!("expected Unavailable, got {other:?}"),
    }
    assert!(!zk_cap_audio::available());
}
