#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test-only
//! Transcription capability tests with the kit's deterministic in-process model
//! (no network, no provider): audio bytes reach the model and the transcript
//! comes back.

use std::io::Write;
use std::sync::{Mutex, MutexGuard};

use zk_cap_transcription::{Transcript, TranscriptionError};
use zk_kit_media::test_support::{fake_transcription_model, FAKE_PROVIDER};
use zk_kit_media::MediaConfig;

static LOCK: Mutex<()> = Mutex::new(());

fn guard() -> MutexGuard<'static, ()> {
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[test]
fn configured_capability_transcribes_bytes() {
    let _guard = guard();
    zk_cap_transcription::configure(fake_transcription_model()).unwrap();
    assert!(zk_cap_transcription::available());
    assert_eq!(zk_cap_transcription::unavailable_reason(), None);

    let transcript: Transcript =
        zk_cap_transcription::transcribe(b"hello audio", "clip.wav").unwrap();
    assert_eq!(transcript.text, "clip.wav:hello audio");
    assert_eq!(transcript.language.as_deref(), Some("en"));
    assert_eq!(transcript.duration, Some(1.5));
    assert_eq!(transcript.provider, FAKE_PROVIDER);
    assert_eq!(transcript.model.as_deref(), Some("fake-whisper"));

    zk_cap_transcription::unconfigure().unwrap();
    assert!(!zk_cap_transcription::available());
    assert!(zk_cap_transcription::unavailable_reason().is_some());
}

#[test]
fn default_filename_is_used_when_empty() {
    let _guard = guard();
    zk_cap_transcription::configure(fake_transcription_model()).unwrap();
    let transcript = zk_cap_transcription::transcribe(b"payload", "   ").unwrap();
    assert_eq!(transcript.text, "audio:payload");
    zk_cap_transcription::unconfigure().unwrap();
}

#[test]
fn empty_audio_is_rejected_before_the_model() {
    let _guard = guard();
    zk_cap_transcription::configure(fake_transcription_model()).unwrap();
    match zk_cap_transcription::transcribe(b"", "clip.wav").unwrap_err() {
        TranscriptionError::Config(message) => {
            assert!(message.contains("audio data is empty"), "{message}");
        }
        other => panic!("expected Config, got {other:?}"),
    }
    zk_cap_transcription::unconfigure().unwrap();
}

#[test]
fn transcribe_file_reads_the_file_and_names_it() {
    let _guard = guard();
    zk_cap_transcription::configure(fake_transcription_model()).unwrap();
    let path = std::env::temp_dir().join("zk-cap-transcription-test.wav");
    {
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(b"file audio").unwrap();
    }
    let transcript = zk_cap_transcription::transcribe_file(&path).unwrap();
    assert_eq!(transcript.text, "zk-cap-transcription-test.wav:file audio");
    std::fs::remove_file(&path).unwrap();
    zk_cap_transcription::unconfigure().unwrap();
}

#[test]
fn transcribe_file_reports_unreadable_paths() {
    let _guard = guard();
    zk_cap_transcription::configure(fake_transcription_model()).unwrap();
    let path = std::env::temp_dir().join("zk-cap-transcription-does-not-exist.wav");
    match zk_cap_transcription::transcribe_file(&path).unwrap_err() {
        TranscriptionError::Config(message) => {
            assert!(message.contains("read `"), "{message}");
        }
        other => panic!("expected Config, got {other:?}"),
    }
    zk_cap_transcription::unconfigure().unwrap();
}

#[test]
fn configure_from_env_is_optional() {
    let _guard = guard();
    if std::env::var("ZEOKEEL_TRANSCRIPTION_API_KEY").is_ok() {
        return;
    }
    zk_cap_transcription::unconfigure().unwrap();
    assert!(!zk_cap_transcription::configure_from_env().unwrap());
    assert!(!zk_cap_transcription::available());
}

#[test]
fn configure_from_config_maps_model_errors() {
    let _guard = guard();
    let error = zk_cap_transcription::configure_from_config(&MediaConfig::default())
        .expect_err("unconfigured media config must fail");
    match error {
        TranscriptionError::Unavailable(message) => {
            assert!(
                message.contains("ZEOKEEL_TRANSCRIPTION_API_KEY"),
                "{message}"
            );
        }
        other => panic!("expected Unavailable, got {other:?}"),
    }
    assert!(!zk_cap_transcription::available());
}
