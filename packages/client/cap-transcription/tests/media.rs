#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test-only
//! Transcription capability tests with a deterministic in-process model (rig
//! `Local<Transcription>` wire + fake transport): audio bytes reach the model
//! and the transcript comes back without any network or provider.

use std::io::Write;
use std::sync::{Mutex, MutexGuard};

use futures::stream;
use rig_core::driver::{Exchange, Local, Model, Opened, Opening, Step, Transport};
use rig_core::operation::Transcription;
use rig_core::transcription::{TranscriptionRequest, TranscriptionResponse};
use rig_core::DynModel;
use zk_cap_transcription::{Transcript, TranscriptionError};

static LOCK: Mutex<()> = Mutex::new(());

fn guard() -> MutexGuard<'static, ()> {
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[derive(Clone)]
struct FakeTranscriptionTransport;

impl Transport<Local<Transcription>> for FakeTranscriptionTransport {
    fn send(
        &self,
        payload: TranscriptionRequest,
        _exchange: Exchange,
    ) -> Opening<Step<Transcription>> {
        // Echo the bytes and file name so the test proves the request reached
        // the model.
        let text = format!(
            "{}:{}",
            payload.filename,
            String::from_utf8_lossy(&payload.data)
        );
        let mut response = TranscriptionResponse::new(text);
        response.language = Some("en".to_owned());
        response.duration = Some(1.5);
        response.model = Some("fake-whisper".to_owned());
        Opening::ready(Opened::new(stream::once(
            async move { Ok(Step::End(response)) },
        )))
    }
}

fn fake_model() -> DynModel<Transcription> {
    Model::new(
        Local::<Transcription>::new("fake-provider"),
        FakeTranscriptionTransport,
    )
    .into()
}

#[test]
fn configured_capability_transcribes_bytes() {
    let _guard = guard();
    zk_cap_transcription::configure(fake_model()).unwrap();
    assert!(zk_cap_transcription::available());
    assert_eq!(zk_cap_transcription::unavailable_reason(), None);

    let transcript: Transcript =
        zk_cap_transcription::transcribe(b"hello audio", "clip.wav").unwrap();
    assert_eq!(transcript.text, "clip.wav:hello audio");
    assert_eq!(transcript.language.as_deref(), Some("en"));
    assert_eq!(transcript.duration, Some(1.5));
    assert_eq!(transcript.provider, "fake-provider");
    assert_eq!(transcript.model.as_deref(), Some("fake-whisper"));

    zk_cap_transcription::unconfigure().unwrap();
    assert!(!zk_cap_transcription::available());
    assert!(zk_cap_transcription::unavailable_reason().is_some());
}

#[test]
fn default_filename_is_used_when_empty() {
    let _guard = guard();
    zk_cap_transcription::configure(fake_model()).unwrap();
    let transcript = zk_cap_transcription::transcribe(b"payload", "   ").unwrap();
    assert_eq!(transcript.text, "audio:payload");
    zk_cap_transcription::unconfigure().unwrap();
}

#[test]
fn empty_audio_is_rejected_before_the_model() {
    let _guard = guard();
    zk_cap_transcription::configure(fake_model()).unwrap();
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
    zk_cap_transcription::configure(fake_model()).unwrap();
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
    zk_cap_transcription::configure(fake_model()).unwrap();
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
