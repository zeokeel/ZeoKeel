//! Audio-transcription capability (`media.transcription`).
//!
//! @capability media.transcription
//!
//! Backed by the shared [`zk_kit_media`] adapter (rig-core `transcription`,
//! OpenAI-compatible). The capability is **dynamically available**:
//! [`available`] is true only after a transcription model is configured
//! ([`configure`], [`configure_from_config`], or [`configure_from_env`] with
//! `ZEOKEEL_TRANSCRIPTION_*`). [`unavailable_reason`] gives the honest reason
//! otherwise, and [`UNAVAILABLE`] is the closed code hosts surface instead of
//! silently dropping the feature.
//!
//! Host wiring: the personal-edition boot calls the generated
//! `zk_composed::configure_capabilities_from_env()` (which calls
//! [`configure_from_env`]); the enterprise client never configures media
//! providers (AI only through the gateway, C7/ADR-031). The package stores no
//! audio: callers pass bytes ([`transcribe`]) or a path ([`transcribe_file`])
//! and get the transcript back.
#![forbid(unsafe_code)]

use std::path::Path;

use zk_kit_media::{
    transcribe_audio, transcription_model, transcription_model_from_config, MediaConfig,
    MediaError, MediaKind, ModelSlot, TranscriptionModel,
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

static MODEL: ModelSlot<TranscriptionModel> = ModelSlot::new();

fn map_media_error(error: MediaError) -> TranscriptionError {
    match error {
        MediaError::Unconfigured(message) => TranscriptionError::Unavailable(message),
        MediaError::Config(message) => TranscriptionError::Config(message),
        MediaError::Provider(message) => TranscriptionError::Provider(message),
        MediaError::Runtime(message) => TranscriptionError::Runtime(message),
    }
}

/// Whether a transcription model is configured.
#[must_use]
pub fn available() -> bool {
    MODEL.is_configured()
}

/// Why transcription is unavailable, or `None` when it is available.
#[must_use]
pub fn unavailable_reason() -> Option<String> {
    if available() {
        return None;
    }
    Some(MediaKind::Transcription.unavailable_reason())
}

/// Install the transcription model used by [`transcribe`].
///
/// # Errors
///
/// [`TranscriptionError::Config`] when the model slot lock is poisoned.
pub fn configure(model: TranscriptionModel) -> Result<(), TranscriptionError> {
    MODEL.set(model).map_err(map_media_error)
}

/// Build and install a transcription model from explicit [`MediaConfig`].
///
/// # Errors
///
/// As [`zk_kit_media::transcription_model_from_config`], plus
/// [`TranscriptionError::Config`] for a poisoned model slot.
pub fn configure_from_config(config: &MediaConfig) -> Result<(), TranscriptionError> {
    configure(transcription_model_from_config(config).map_err(map_media_error)?)
}

/// Configure from `ZEOKEEL_TRANSCRIPTION_*`; `Ok(false)` when no API key is set.
///
/// This is the entry point the personal-edition boot uses through
/// `zk_composed::configure_capabilities_from_env()`.
///
/// # Errors
///
/// [`TranscriptionError::Config`] for an invalid base URL/provider, and as
/// [`configure`] for a poisoned model slot.
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
/// [`TranscriptionError::Config`] when the model slot lock is poisoned.
pub fn unconfigure() -> Result<(), TranscriptionError> {
    MODEL.clear().map_err(map_media_error)
}

/// Transcribe encoded audio bytes.
///
/// `filename` names the audio for the provider (e.g. `clip.wav`); an empty
/// name falls back to `"audio"`.
///
/// # Errors
///
/// [`TranscriptionError::Unavailable`] when no model is configured,
/// [`TranscriptionError::Config`] for empty audio, [`TranscriptionError::Provider`]
/// when the provider call fails, and [`TranscriptionError::Runtime`] when the
/// shared runtime cannot start.
pub fn transcribe(data: &[u8], filename: &str) -> Result<Transcript, TranscriptionError> {
    if data.is_empty() {
        return Err(TranscriptionError::Config("audio data is empty".to_owned()));
    }
    let Some(model) = MODEL.get().map_err(map_media_error)? else {
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
    let output = transcribe_audio(&model, data, &filename).map_err(map_media_error)?;
    Ok(Transcript {
        text: output.text,
        language: output.language,
        duration: output.duration,
        provider: output.provider,
        model: output.model,
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
