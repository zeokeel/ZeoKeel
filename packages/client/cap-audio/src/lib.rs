//! Speech-synthesis capability (`media.audio`).
//!
//! Backed by rig-core's OpenAI-compatible `audio_generation` (TTS) model; the
//! workspace enables rig's `audio` feature. The capability is **dynamically
//! available**: [`available`] is true only after an audio model is configured
//! ([`configure`], [`configure_from_config`], or [`configure_from_env`] with
//! `ZEOKEEL_AUDIO_*`). [`unavailable_reason`] gives the honest reason
//! otherwise, and [`UNAVAILABLE`] is the closed code hosts surface instead of
//! silently dropping the feature.
//!
//! The package stores no assets: [`synthesize`] returns the audio bytes to the
//! caller.
#![forbid(unsafe_code)]

use std::sync::{Mutex, OnceLock};

use rig_core::audio_generation::AudioGenerationRequestBuilder;
use rig_core::operation::AudioGeneration;
use rig_core::DynModel;
use zk_llm_rig::{audio_model, audio_model_from_config, block_on, MediaConfig, MediaError};

/// Closed unavailability code hosts surface instead of silently dropping the feature.
pub const UNAVAILABLE: &str = "CAPABILITY_UNAVAILABLE";

/// Contract spelling of [`UNAVAILABLE`] (the `CAPABILITY_UNAVAILABLE` code).
pub const CAPABILITY_UNAVAILABLE: &str = "CAPABILITY_UNAVAILABLE";

/// Errors from the audio-generation capability.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    /// No audio model is configured.
    #[error("{UNAVAILABLE}: {0}")]
    Unavailable(String),
    /// Configuration is invalid (empty text/voice, bad provider/env).
    #[error("audio config: {0}")]
    Config(String),
    /// The audio provider call failed.
    #[error("audio provider: {0}")]
    Provider(String),
    /// The shared blocking runtime failed.
    #[error("audio runtime: {0}")]
    Runtime(String),
}

/// One synthesized audio clip, returned to the caller without storing an asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedAudio {
    /// Encoded audio bytes (provider container, e.g. MP3).
    pub bytes: Vec<u8>,
    /// Provider descriptor name rig reports (`"openai"`).
    pub provider: String,
    /// Provider-reported model id, when the reply named one.
    pub model: Option<String>,
}

static MODEL: OnceLock<Mutex<Option<DynModel<AudioGeneration>>>> = OnceLock::new();

fn model_slot() -> &'static Mutex<Option<DynModel<AudioGeneration>>> {
    MODEL.get_or_init(|| Mutex::new(None))
}

fn map_media_error(error: MediaError) -> AudioError {
    match error {
        MediaError::Unconfigured(message) => AudioError::Unavailable(message),
        MediaError::Config(message) => AudioError::Config(message),
        MediaError::Runtime(message) => AudioError::Runtime(message),
    }
}

/// Whether an audio model is configured.
#[must_use]
pub fn available() -> bool {
    model_slot()
        .lock()
        .map(|guard| guard.is_some())
        .unwrap_or(false)
}

/// Why speech synthesis is unavailable, or `None` when it is available.
#[must_use]
pub fn unavailable_reason() -> Option<String> {
    if available() {
        return None;
    }
    Some(
        "no audio model configured: call zk_cap_audio::configure(model), \
         configure_from_config(&zk_llm_rig::MediaConfig), or set \
         ZEOKEEL_AUDIO_API_KEY/ZEOKEEL_AUDIO_BASE_URL/ZEOKEEL_AUDIO_MODEL and call \
         configure_from_env()"
            .to_owned(),
    )
}

/// Install the audio model used by [`synthesize`].
///
/// # Errors
///
/// [`AudioError::Config`] when the model lock is poisoned.
pub fn configure(model: DynModel<AudioGeneration>) -> Result<(), AudioError> {
    let mut guard = model_slot()
        .lock()
        .map_err(|_| AudioError::Config("audio model lock poisoned".to_owned()))?;
    *guard = Some(model);
    Ok(())
}

/// Build and install an audio model from explicit [`MediaConfig`].
///
/// # Errors
///
/// As [`zk_llm_rig::audio_model_from_config`], plus [`AudioError::Config`]
/// for a poisoned model lock.
pub fn configure_from_config(config: &MediaConfig) -> Result<(), AudioError> {
    configure(audio_model_from_config(config).map_err(map_media_error)?)
}

/// Configure from `ZEOKEEL_AUDIO_*`; `Ok(false)` when no API key is configured.
///
/// # Errors
///
/// [`AudioError::Config`] for an invalid base URL/provider, and as
/// [`configure`] for a poisoned model lock.
pub fn configure_from_env() -> Result<bool, AudioError> {
    match audio_model() {
        Ok(model) => {
            configure(model)?;
            Ok(true)
        }
        Err(MediaError::Unconfigured(_)) => Ok(false),
        Err(error) => Err(map_media_error(error)),
    }
}

/// Remove the audio model, so [`available`] honestly returns false again.
///
/// # Errors
///
/// [`AudioError::Config`] when the model lock is poisoned.
pub fn unconfigure() -> Result<(), AudioError> {
    let mut guard = model_slot()
        .lock()
        .map_err(|_| AudioError::Config("audio model lock poisoned".to_owned()))?;
    *guard = None;
    Ok(())
}

/// Synthesize speech for `text` in provider `voice`.
///
/// # Errors
///
/// [`AudioError::Unavailable`] when no model is configured,
/// [`AudioError::Config`] for empty text/voice or a poisoned model lock,
/// [`AudioError::Provider`] when the provider call fails, and
/// [`AudioError::Runtime`] when the shared runtime cannot start.
pub fn synthesize(text: &str, voice: &str) -> Result<GeneratedAudio, AudioError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(AudioError::Config("audio text is empty".to_owned()));
    }
    let voice = voice.trim();
    if voice.is_empty() {
        return Err(AudioError::Config(
            "audio voice is empty (pass the provider voice id, e.g. `alloy`)".to_owned(),
        ));
    }
    let model = {
        let guard = model_slot()
            .lock()
            .map_err(|_| AudioError::Config("audio model lock poisoned".to_owned()))?;
        guard.clone()
    };
    let Some(model) = model else {
        return Err(AudioError::Unavailable(
            unavailable_reason().unwrap_or_default(),
        ));
    };
    let request = AudioGenerationRequestBuilder::new(text, voice).build();
    let response = block_on(model.call(request))
        .map_err(map_media_error)?
        .map_err(|error| AudioError::Provider(error.to_string()))?;
    Ok(GeneratedAudio {
        bytes: response.audio,
        provider: response.provider,
        model: response.model,
    })
}
