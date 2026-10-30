//! Speech-synthesis capability (`media.audio`).
//!
//! @capability media.audio
//!
//! Backed by the shared [`zk_kit_media`] adapter (rig-core `audio_generation`,
//! OpenAI-compatible TTS; the workspace enables rig's `audio` feature). The
//! capability is **dynamically available**: [`available`] is true only after an
//! audio model is configured ([`configure`], [`configure_from_config`], or
//! [`configure_from_env`] with `ZEOKEEL_AUDIO_*`). [`unavailable_reason`] gives
//! the honest reason otherwise, and [`UNAVAILABLE`] is the closed code hosts
//! surface instead of silently dropping the feature.
//!
//! Host wiring: the personal-edition boot calls the generated
//! `zk_composed::configure_capabilities_from_env()` (which calls
//! [`configure_from_env`]); the enterprise client never configures media
//! providers (AI only through the gateway, C7/ADR-031). The package stores no
//! assets: [`synthesize`] returns the audio bytes to the caller.
#![forbid(unsafe_code)]

use zk_kit_media::{
    audio_model, audio_model_from_config, synthesize_audio, AudioModel, MediaConfig, MediaError,
    MediaKind, ModelSlot,
};

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

static MODEL: ModelSlot<AudioModel> = ModelSlot::new();

fn map_media_error(error: MediaError) -> AudioError {
    match error {
        MediaError::Unconfigured(message) => AudioError::Unavailable(message),
        MediaError::Config(message) => AudioError::Config(message),
        MediaError::Provider(message) => AudioError::Provider(message),
        MediaError::Runtime(message) => AudioError::Runtime(message),
    }
}

/// Whether an audio model is configured.
#[must_use]
pub fn available() -> bool {
    MODEL.is_configured()
}

/// Why speech synthesis is unavailable, or `None` when it is available.
#[must_use]
pub fn unavailable_reason() -> Option<String> {
    if available() {
        return None;
    }
    Some(MediaKind::Audio.unavailable_reason())
}

/// Install the audio model used by [`synthesize`].
///
/// # Errors
///
/// [`AudioError::Config`] when the model slot lock is poisoned.
pub fn configure(model: AudioModel) -> Result<(), AudioError> {
    MODEL.set(model).map_err(map_media_error)
}

/// Build and install an audio model from explicit [`MediaConfig`].
///
/// # Errors
///
/// As [`zk_kit_media::audio_model_from_config`], plus [`AudioError::Config`]
/// for a poisoned model slot.
pub fn configure_from_config(config: &MediaConfig) -> Result<(), AudioError> {
    configure(audio_model_from_config(config).map_err(map_media_error)?)
}

/// Configure from `ZEOKEEL_AUDIO_*`; `Ok(false)` when no API key is configured.
///
/// This is the entry point the personal-edition boot uses through
/// `zk_composed::configure_capabilities_from_env()`.
///
/// # Errors
///
/// [`AudioError::Config`] for an invalid base URL/provider, and as
/// [`configure`] for a poisoned model slot.
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
/// [`AudioError::Config`] when the model slot lock is poisoned.
pub fn unconfigure() -> Result<(), AudioError> {
    MODEL.clear().map_err(map_media_error)
}

/// Synthesize speech for `text` in provider `voice`.
///
/// # Errors
///
/// [`AudioError::Unavailable`] when no model is configured,
/// [`AudioError::Config`] for empty text/voice, [`AudioError::Provider`] when
/// the provider call fails, and [`AudioError::Runtime`] when the shared
/// runtime cannot start.
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
    let Some(model) = MODEL.get().map_err(map_media_error)? else {
        return Err(AudioError::Unavailable(
            unavailable_reason().unwrap_or_default(),
        ));
    };
    let output = synthesize_audio(&model, text, voice).map_err(map_media_error)?;
    Ok(GeneratedAudio {
        bytes: output.bytes,
        provider: output.provider,
        model: output.model,
    })
}
