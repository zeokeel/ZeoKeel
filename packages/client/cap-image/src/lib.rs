//! Image generation capability (`media.image`).
//!
//! Backed by rig-core's OpenAI-compatible `image_generation` model; the
//! workspace enables rig's `image` feature. The capability is **dynamically
//! available**: [`available`] is true only after an image model is configured
//! ([`configure`], [`configure_from_config`], or [`configure_from_env`] with
//! `ZEOKEEL_IMAGE_*`). [`unavailable_reason`] gives the honest reason
//! otherwise, and [`UNAVAILABLE`] is the closed code hosts surface instead of
//! silently dropping the feature.
//!
//! The package stores no assets: [`generate`] returns the encoded bytes and
//! their detected [`ImageEncoding`] to the caller.
#![forbid(unsafe_code)]

use std::sync::{Mutex, OnceLock};

use rig_core::image_generation::ImageGenerationRequestBuilder;
use rig_core::operation::ImageGeneration;
use rig_core::DynModel;
use zk_llm_rig::{block_on, image_model, image_model_from_config, MediaConfig, MediaError};

/// Closed unavailability code hosts surface instead of silently dropping the feature.
pub const UNAVAILABLE: &str = "CAPABILITY_UNAVAILABLE";

/// Contract spelling of [`UNAVAILABLE`] (the `CAPABILITY_UNAVAILABLE` code).
pub const CAPABILITY_UNAVAILABLE: &str = "CAPABILITY_UNAVAILABLE";

/// Errors from the image-generation capability.
#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    /// No image model is configured.
    #[error("{UNAVAILABLE}: {0}")]
    Unavailable(String),
    /// Configuration is invalid (empty prompt, bad provider/env).
    #[error("image config: {0}")]
    Config(String),
    /// The image provider call failed.
    #[error("image provider: {0}")]
    Provider(String),
    /// The shared blocking runtime failed.
    #[error("image runtime: {0}")]
    Runtime(String),
}

/// Encoding detected from the generated image bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageEncoding {
    /// PNG (`\x89PNG\r\n\x1a\n`).
    Png,
    /// JPEG (`\xFF\xD8\xFF`).
    Jpeg,
    /// WebP (`RIFF....WEBP`).
    WebP,
    /// GIF (`GIF8`).
    Gif,
    /// A payload none of the known magic bytes describe.
    Unknown,
}

impl ImageEncoding {
    /// Detect the encoding from the payload's magic bytes.
    ///
    /// Providers normally return PNG or WebP; the detection keeps the
    /// capability honest instead of guessing `image/png`.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        if bytes.starts_with(&PNG_MAGIC) {
            Self::Png
        } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Self::Jpeg
        } else if bytes.starts_with(b"GIF8") {
            Self::Gif
        } else if bytes.starts_with(b"RIFF")
            && bytes
                .get(8..12)
                .is_some_and(|chunk| chunk.starts_with(b"WEBP"))
        {
            Self::WebP
        } else {
            Self::Unknown
        }
    }

    /// MIME type of the detected encoding.
    #[must_use]
    pub const fn mime_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::WebP => "image/webp",
            Self::Gif => "image/gif",
            Self::Unknown => "application/octet-stream",
        }
    }
}

/// One generated image, returned to the caller without storing an asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedImage {
    /// Encoded image bytes.
    pub bytes: Vec<u8>,
    /// Encoding detected from [`Self::bytes`].
    pub encoding: ImageEncoding,
    /// Provider descriptor name rig reports (`"openai"`).
    pub provider: String,
    /// Provider-reported model id, when the reply named one.
    pub model: Option<String>,
}

static MODEL: OnceLock<Mutex<Option<DynModel<ImageGeneration>>>> = OnceLock::new();

fn model_slot() -> &'static Mutex<Option<DynModel<ImageGeneration>>> {
    MODEL.get_or_init(|| Mutex::new(None))
}

fn map_media_error(error: MediaError) -> ImageError {
    match error {
        MediaError::Unconfigured(message) => ImageError::Unavailable(message),
        MediaError::Config(message) => ImageError::Config(message),
        MediaError::Runtime(message) => ImageError::Runtime(message),
    }
}

/// Whether an image model is configured.
#[must_use]
pub fn available() -> bool {
    model_slot()
        .lock()
        .map(|guard| guard.is_some())
        .unwrap_or(false)
}

/// Why image generation is unavailable, or `None` when it is available.
#[must_use]
pub fn unavailable_reason() -> Option<String> {
    if available() {
        return None;
    }
    Some(
        "no image model configured: call zk_cap_image::configure(model), \
         configure_from_config(&zk_llm_rig::MediaConfig), or set \
         ZEOKEEL_IMAGE_API_KEY/ZEOKEEL_IMAGE_BASE_URL/ZEOKEEL_IMAGE_MODEL and call \
         configure_from_env()"
            .to_owned(),
    )
}

/// Install the image model used by [`generate`].
///
/// # Errors
///
/// [`ImageError::Config`] when the model lock is poisoned.
pub fn configure(model: DynModel<ImageGeneration>) -> Result<(), ImageError> {
    let mut guard = model_slot()
        .lock()
        .map_err(|_| ImageError::Config("image model lock poisoned".to_owned()))?;
    *guard = Some(model);
    Ok(())
}

/// Build and install an image model from explicit [`MediaConfig`].
///
/// # Errors
///
/// As [`zk_llm_rig::image_model_from_config`], plus [`ImageError::Config`]
/// for a poisoned model lock.
pub fn configure_from_config(config: &MediaConfig) -> Result<(), ImageError> {
    configure(image_model_from_config(config).map_err(map_media_error)?)
}

/// Configure from `ZEOKEEL_IMAGE_*`; `Ok(false)` when no API key is configured.
///
/// # Errors
///
/// [`ImageError::Config`] for an invalid base URL/provider, and as
/// [`configure`] for a poisoned model lock.
pub fn configure_from_env() -> Result<bool, ImageError> {
    match image_model() {
        Ok(model) => {
            configure(model)?;
            Ok(true)
        }
        Err(MediaError::Unconfigured(_)) => Ok(false),
        Err(error) => Err(map_media_error(error)),
    }
}

/// Remove the image model, so [`available`] honestly returns false again.
///
/// # Errors
///
/// [`ImageError::Config`] when the model lock is poisoned.
pub fn unconfigure() -> Result<(), ImageError> {
    let mut guard = model_slot()
        .lock()
        .map_err(|_| ImageError::Config("image model lock poisoned".to_owned()))?;
    *guard = None;
    Ok(())
}

/// Generate one image from `prompt` at `width`×`height`.
///
/// # Errors
///
/// [`ImageError::Unavailable`] when no model is configured,
/// [`ImageError::Config`] for an empty prompt or a poisoned model lock,
/// [`ImageError::Provider`] when the provider call fails, and
/// [`ImageError::Runtime`] when the shared runtime cannot start.
pub fn generate(prompt: &str, width: u32, height: u32) -> Result<GeneratedImage, ImageError> {
    let prompt = prompt.trim();
    if prompt.is_empty() {
        return Err(ImageError::Config("image prompt is empty".to_owned()));
    }
    let model = {
        let guard = model_slot()
            .lock()
            .map_err(|_| ImageError::Config("image model lock poisoned".to_owned()))?;
        guard.clone()
    };
    let Some(model) = model else {
        return Err(ImageError::Unavailable(
            unavailable_reason().unwrap_or_default(),
        ));
    };
    let request = ImageGenerationRequestBuilder::new(prompt)
        .width(width)
        .height(height)
        .build();
    let response = block_on(model.call(request))
        .map_err(map_media_error)?
        .map_err(|error| ImageError::Provider(error.to_string()))?;
    Ok(GeneratedImage {
        encoding: ImageEncoding::from_bytes(&response.image),
        bytes: response.image,
        provider: response.provider,
        model: response.model,
    })
}
