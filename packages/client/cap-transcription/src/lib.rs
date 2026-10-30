//! Audio-transcription capability (`media.transcription`).
//!
//! Backed by rig-core's OpenAI-compatible `transcription` model. The
//! capability is **dynamically available**: [`available`] is true only after a
//! transcription model is configured ([`configure`], [`configure_from_config`],
//! or [`configure_from_env`] with `ZEOKEEL_TRANSCRIPTION_*`).
//! [`unavailable_reason`] gives the honest reason otherwise, and
//! [`UNAVAILABLE`] is the closed code hosts surface instead of silently
//! dropping the feature.
//!
//! The package stores no audio: callers pass bytes ([`transcribe`]) or a path
//! ([`transcribe_file`]) and get the transcript back.
#![forbid(unsafe_code)]

use std::path::Path;
use std::sync::{Mutex, OnceLock};

use rig_core::operation::Transcription;
use rig_core::transcription::TranscriptionRequestBuilder;
use rig_core::DynModel;
use zk_llm_rig::{
    block_on, transcription_model, transcription_model_from_config, MediaConfig, MediaError,
};

/// Closed unavailability code hosts surface instead of silently dropping the feature.
pub const UNAVAILABLE: &str = "CAPABILITY_UNAVAILABLE";

/// Contract spelling of [`UNAVAILABLE`] (the `CAPABILITY_UNAVAILABLE` code).
pub const CAPABILITY_UNAVAILABLE: &str = "CAPABILITY_UNAVAILABLE";

/// File name a request carries when the caller does not supply one.
const DEFAULT_FILENAME: &str = "audio";

/// Errors from the transcription capability.
#[derive(Debug, thiserror::Error)]
pub enum TranscriptionError {
    /// No transcription model is configured.
    #[error("{UNAVAILABLE}: {0}")]
    Unavailable(String),
    /// Configuration/input is invalid (empty audio, unreadable file, bad env).
    #[error("transcription config: {0}")]
    Config(String),
    /// The transcription provider call failed.
    #[error("transcription provider: {0}")]
    Provider(String),
    /// The shared blocking runtime failed.
    #[error("transcription runtime: {0}")]
    Runtime(String),
}

/// One transcript, returned to the caller without storing the audio asset.
#[derive(Debug, Clone, PartialEq)]
pub struct Transcript {
    /// The transcribed text.
    pub text: String,
    /// Language the provider reported, when it reported one.
    pub language: Option<String>,
    /// Audio duration in seconds, when the provider reported it.
    pub duration: Option<f64>,
    /// Provider descriptor name rig reports (`"openai"`).
    pub provider: String,
    /// Provider-reported model id, when the reply named one.
    pub model: Option<String>,
}

static MODEL: OnceLock<Mutex<Option<DynModel<Transcription>>>> = OnceLock::new();

fn model_slot() -> &'static Mutex<Option<DynModel<Transcription>>> {
    MODEL.get_or_init(|| Mutex::new(None))
}

fn map_media_error(error: MediaError) -> TranscriptionError {
    match error {
        MediaError::Unconfigured(message) => TranscriptionError::Unavailable(message),
        MediaError::Config(message) => TranscriptionError::Config(message),
        MediaError::Runtime(message) => TranscriptionError::Runtime(message),
    }
}

/// Whether a transcription model is configured.
#[must_use]
pub fn available() -> bool {
    model_slot()
        .lock()
        .map(|guard| guard.is_some())
        .unwrap_or(false)
}

/// Why transcription is unavailable, or `None` when it is available.
#[must_use]
pub fn unavailable_reason() -> Option<String> {
    if available() {
        return None;
    }
    Some(
        "no transcription model configured: call zk_cap_transcription::configure(model), \
         configure_from_config(&zk_llm_rig::MediaConfig), or set \
         ZEOKEEL_TRANSCRIPTION_API_KEY/ZEOKEEL_TRANSCRIPTION_BASE_URL/ZEOKEEL_TRANSCRIPTION_MODEL \
         and call configure_from_env()"
            .to_owned(),
    )
}

/// Install the transcription model used by [`transcribe`].
///
/// # Errors
///
/// [`TranscriptionError::Config`] when the model lock is poisoned.
pub fn configure(model: DynModel<Transcription>) -> Result<(), TranscriptionError> {
    let mut guard = model_slot()
        .lock()
        .map_err(|_| TranscriptionError::Config("transcription model lock poisoned".to_owned()))?;
    *guard = Some(model);
    Ok(())
}

/// Build and install a transcription model from explicit [`MediaConfig`].
///
/// # Errors
///
/// As [`zk_llm_rig::transcription_model_from_config`], plus
/// [`TranscriptionError::Config`] for a poisoned model lock.
pub fn configure_from_config(config: &MediaConfig) -> Result<(), TranscriptionError> {
    configure(transcription_model_from_config(config).map_err(map_media_error)?)
}

/// Configure from `ZEOKEEL_TRANSCRIPTION_*`; `Ok(false)` when no API key is set.
///
/// # Errors
///
/// [`TranscriptionError::Config`] for an invalid base URL/provider, and as
/// [`configure`] for a poisoned model lock.
pub fn configure_from_env() -> Result<bool, TranscriptionError> {
    match transcription_model() {
        Ok(model) => {
            configure(model)?;
            Ok(true)
        }
        Err(MediaError::Unconfigured(_)) => Ok(false),
        Err(error) => Err(map_media_error(error)),
    }
}

/// Remove the transcription model, so [`available`] honestly returns false.
///
/// # Errors
///
/// [`TranscriptionError::Config`] when the model lock is poisoned.
pub fn unconfigure() -> Result<(), TranscriptionError> {
    let mut guard = model_slot()
        .lock()
        .map_err(|_| TranscriptionError::Config("transcription model lock poisoned".to_owned()))?;
    *guard = None;
    Ok(())
}

/// Transcribe encoded audio bytes.
///
/// `filename` names the audio for the provider (e.g. `clip.wav`); an empty
/// name falls back to `"audio"`.
///
/// # Errors
///
/// [`TranscriptionError::Unavailable`] when no model is configured,
/// [`TranscriptionError::Config`] for empty audio or a poisoned model lock,
/// [`TranscriptionError::Provider`] when the provider call fails, and
/// [`TranscriptionError::Runtime`] when the shared runtime cannot start.
pub fn transcribe(data: &[u8], filename: &str) -> Result<Transcript, TranscriptionError> {
    if data.is_empty() {
        return Err(TranscriptionError::Config("audio data is empty".to_owned()));
    }
    let model = {
        let guard = model_slot().lock().map_err(|_| {
            TranscriptionError::Config("transcription model lock poisoned".to_owned())
        })?;
        guard.clone()
    };
    let Some(model) = model else {
        return Err(TranscriptionError::Unavailable(
            unavailable_reason().unwrap_or_default(),
        ));
    };
    let filename = {
        let name = filename.trim();
        if name.is_empty() {
            DEFAULT_FILENAME.to_owned()
        } else {
            name.to_owned()
        }
    };
    let request = TranscriptionRequestBuilder::new(data.to_vec())
        .filename(filename)
        .build();
    let response = block_on(model.call(request))
        .map_err(map_media_error)?
        .map_err(|error| TranscriptionError::Provider(error.to_string()))?;
    Ok(Transcript {
        text: response.text,
        language: response.language,
        duration: response.duration,
        provider: response.provider,
        model: response.model,
    })
}

/// Transcribe the encoded audio file at `path`.
///
/// # Errors
///
/// [`TranscriptionError::Config`] when the file cannot be read, plus all
/// [`transcribe`] errors.
pub fn transcribe_file(path: impl AsRef<Path>) -> Result<Transcript, TranscriptionError> {
    let path = path.as_ref();
    let data = std::fs::read(path).map_err(|error| {
        TranscriptionError::Config(format!("read `{}`: {error}", path.display()))
    })?;
    let filename = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    transcribe(&data, &filename)
}
