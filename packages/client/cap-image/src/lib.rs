//! Image generation capability (`media.image`).
//!
//! @capability media.image
//!
//! Backed by the shared [`zk_kit_media`] adapter (rig-core `image_generation`,
//! OpenAI-compatible; the workspace enables rig's `image` feature). The
//! capability is **dynamically available**: [`available`] is true only after an
//! image model is configured ([`configure`], [`configure_from_config`], or
//! [`configure_from_env`] with `ZEOKEEL_IMAGE_*`). [`unavailable_reason`] gives
//! the honest reason otherwise, and [`UNAVAILABLE`] is the closed code hosts
//! surface instead of silently dropping the feature.
//!
//! Host wiring: the personal-edition boot calls the generated
//! `zk_composed::configure_capabilities_from_env()` (which calls
//! [`configure_from_env`]); the enterprise client never configures media
//! providers (AI only through the gateway, C7/ADR-031). The package stores no
//! assets: [`generate`] returns the encoded bytes and their detected
//! [`ImageEncoding`] to the caller.
#![forbid(unsafe_code)]

use zk_kit_media::{
    generate_image, image_model, image_model_from_config, ImageModel, MediaConfig, MediaError,
    MediaKind, ModelSlot,
};

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

static MODEL: ModelSlot<ImageModel> = ModelSlot::new();

fn map_media_error(error: MediaError) -> ImageError {
    match error {
        MediaError::Unconfigured(message) => ImageError::Unavailable(message),
        MediaError::Config(message) => ImageError::Config(message),
        MediaError::Provider(message) => ImageError::Provider(message),
        MediaError::Runtime(message) => ImageError::Runtime(message),
    }
}

/// Whether an image model is configured.
#[must_use]
pub fn available() -> bool {
    MODEL.is_configured()
}

/// Why image generation is unavailable, or `None` when it is available.
#[must_use]
pub fn unavailable_reason() -> Option<String> {
    if available() {
        return None;
    }
    Some(MediaKind::Image.unavailable_reason())
}

/// Install the image model used by [`generate`].
///
/// # Errors
///
/// [`ImageError::Config`] when the model slot lock is poisoned.
pub fn configure(model: ImageModel) -> Result<(), ImageError> {
    MODEL.set(model).map_err(map_media_error)
}

/// Build and install an image model from explicit [`MediaConfig`].
///
/// # Errors
///
/// As [`zk_kit_media::image_model_from_config`], plus [`ImageError::Config`]
/// for a poisoned model slot.
pub fn configure_from_config(config: &MediaConfig) -> Result<(), ImageError> {
    configure(image_model_from_config(config).map_err(map_media_error)?)
}

/// Configure from `ZEOKEEL_IMAGE_*`; `Ok(false)` when no API key is configured.
///
/// This is the entry point the personal-edition boot uses through
/// `zk_composed::configure_capabilities_from_env()`.
///
/// # Errors
///
/// [`ImageError::Config`] for an invalid base URL/provider, and as
/// [`configure`] for a poisoned model slot.
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
/// [`ImageError::Config`] when the model slot lock is poisoned.
pub fn unconfigure() -> Result<(), ImageError> {
    MODEL.clear().map_err(map_media_error)
}

/// Generate one image from `prompt` at `width`×`height`.
///
/// # Errors
///
/// [`ImageError::Unavailable`] when no model is configured,
/// [`ImageError::Config`] for an empty prompt, [`ImageError::Provider`] when
/// the provider call fails, and [`ImageError::Runtime`] when the shared
/// runtime cannot start.
pub fn generate(prompt: &str, width: u32, height: u32) -> Result<GeneratedImage, ImageError> {
    let prompt = prompt.trim();
    if prompt.is_empty() {
        return Err(ImageError::Config("image prompt is empty".to_owned()));
    }
    let Some(model) = MODEL.get().map_err(map_media_error)? else {
        return Err(ImageError::Unavailable(
            unavailable_reason().unwrap_or_default(),
        ));
    };
    let output = generate_image(&model, prompt, width, height).map_err(map_media_error)?;
    Ok(GeneratedImage {
        encoding: ImageEncoding::from_bytes(&output.bytes),
        bytes: output.bytes,
        provider: output.provider,
        model: output.model,
    })
}
