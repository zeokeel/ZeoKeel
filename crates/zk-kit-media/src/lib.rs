//! Shared rig adapter for media models (image/audio/transcription/rerank) and
//! OpenAI-compatible embeddings.
//!
//! @capability kit.media
//!
//! This kit owns the rig-core model wires, the provider configuration, the
//! blocking bridge and the model-slot plumbing, so the capability packages
//! (`cap-{image,audio,transcription,rerank}` and `cap-vector`) never depend on
//! `rig-core`, `zk-llm-rig` or on each other (C4/C17).
//!
//! Providers: image/audio/transcription use rig-core's OpenAI-compatible wire;
//! rerank uses Voyage AI because rig-core 0.45 has no OpenAI rerank endpoint
//! (`RerankQuirks::unsupported`). The workspace enables rig's `audio`/`image`
//! features only (`pdf` needs uncached `lopdf 0.44`; `epub` is GPL-3.0).
#![forbid(unsafe_code)]

use std::fmt;
use std::sync::{Mutex, OnceLock};

use rig_core::audio_generation::AudioGenerationRequestBuilder;
use rig_core::image_generation::ImageGenerationRequestBuilder;
use rig_core::operation::{
    AudioGeneration, Embedding, ImageGeneration, Rerank, RerankRequest, Transcription,
};
use rig_core::providers::openai::{OpenAI, OpenAIConfig};
use rig_core::providers::voyageai::VoyageAiConfig;
use rig_core::transcription::TranscriptionRequestBuilder;
use rig_core::DynModel;

/// A modality rig-core exposes as a buffered (non-streaming) model operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    /// Image generation (`rig_core::operation::ImageGeneration`).
    Image,
    /// Speech synthesis (`rig_core::operation::AudioGeneration`).
    Audio,
    /// Audio transcription (`rig_core::operation::Transcription`).
    Transcription,
    /// Document reranking (`rig_core::operation::Rerank`).
    Rerank,
}

impl MediaKind {
    /// Environment-variable prefix this kind reads (`ZEOKEEL_IMAGE`, ...).
    #[must_use]
    pub const fn env_prefix(self) -> &'static str {
        match self {
            Self::Image => "ZEOKEEL_IMAGE",
            Self::Audio => "ZEOKEEL_AUDIO",
            Self::Transcription => "ZEOKEEL_TRANSCRIPTION",
            Self::Rerank => "ZEOKEEL_RERANK",
        }
    }

    /// Provider used when `<PREFIX>_PROVIDER` is unset.
    #[must_use]
    pub const fn default_provider(self) -> &'static str {
        match self {
            Self::Image | Self::Audio | Self::Transcription => "openai",
            Self::Rerank => "voyageai",
        }
    }

    /// Provider API root used when `<PREFIX>_BASE_URL` is unset.
    #[must_use]
    pub const fn default_base_url(self) -> &'static str {
        match self {
            Self::Image | Self::Audio | Self::Transcription => "https://api.openai.com/v1",
            Self::Rerank => "https://api.voyageai.com/v1",
        }
    }

    /// Model id used when `<PREFIX>_MODEL` is unset.
    #[must_use]
    pub const fn default_model(self) -> &'static str {
        match self {
            Self::Image => "gpt-image-1",
            Self::Audio => "tts-1",
            Self::Transcription => "whisper-1",
            Self::Rerank => "rerank-2.5",
        }
    }

    /// Kind name used in diagnostics and `unavailable_reason` text.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Audio => "audio",
            Self::Transcription => "transcription",
            Self::Rerank => "rerank",
        }
    }

    /// Honest reason a capability of this kind is unavailable, for hosts to
    /// surface verbatim next to `CAPABILITY_UNAVAILABLE`.
    #[must_use]
    pub fn unavailable_reason(self) -> String {
        let prefix = self.env_prefix();
        format!(
            "no {} model configured: call the capability's configure(model), \
             configure_from_config(&MediaConfig), or set {prefix}_API_KEY/\
             {prefix}_BASE_URL/{prefix}_MODEL and call configure_from_env()",
            self.label()
        )
    }
}

/// Configuration for one media model (image/audio/transcription/rerank).
///
/// Empty `provider`/`model` fall back to the [`MediaKind`] defaults; an empty
/// `base_url` falls back to the provider's API root. `api_key` is required.
/// The [`Debug`] implementation redacts `api_key` so logging a config never
/// leaks the bearer key.
#[derive(Clone, Default)]
pub struct MediaConfig {
    /// Provider: `openai` (image/audio/transcription) or `voyageai` (rerank).
    pub provider: String,
    /// API root, e.g. `https://api.openai.com/v1` (empty => provider default).
    pub base_url: String,
    /// Bearer key (required).
    pub api_key: String,
    /// Model id (empty => kind default).
    pub model: String,
}

impl MediaConfig {
    /// Explicit configuration; empty provider/model/base_url fall back to defaults.
    #[must_use]
    pub fn new(
        provider: impl Into<String>,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            provider: provider.into(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }

    /// Resolve from `<PREFIX>_{PROVIDER,BASE_URL,API_KEY,MODEL}`.
    #[must_use]
    pub fn from_env(kind: MediaKind) -> Self {
        let prefix = kind.env_prefix();
        let provider = env_prefixed(prefix, "PROVIDER");
        let model = env_prefixed(prefix, "MODEL");
        Self {
            provider: if provider.is_empty() {
                kind.default_provider().to_owned()
            } else {
                provider
            },
            base_url: env_prefixed(prefix, "BASE_URL"),
            api_key: env_prefixed(prefix, "API_KEY"),
            model: if model.is_empty() {
                kind.default_model().to_owned()
            } else {
                model
            },
        }
    }
}

impl fmt::Debug for MediaConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MediaConfig")
            .field("provider", &self.provider)
            .field("base_url", &self.base_url)
            .field("api_key", &redacted_key(&self.api_key))
            .field("model", &self.model)
            .finish()
    }
}

fn redacted_key(api_key: &str) -> &'static str {
    if api_key.is_empty() {
        "<unset>"
    } else {
        "<redacted>"
    }
}

/// Typed errors from the media model builders and calls.
#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    /// No media provider is configured (`<PREFIX>_API_KEY` is empty).
    #[error("media model not configured: {0}")]
    Unconfigured(String),
    /// The provider/base URL/unsupported-provider configuration is invalid.
    #[error("media model config: {0}")]
    Config(String),
    /// The provider call failed.
    #[error("media provider: {0}")]
    Provider(String),
    /// The shared blocking runtime could not be created.
    #[error("media runtime: {0}")]
    Runtime(String),
}

struct ResolvedMedia<'a> {
    provider: &'a str,
    base_url: &'a str,
    api_key: &'a str,
    model: &'a str,
}

fn resolve_media(kind: MediaKind, config: &MediaConfig) -> Result<ResolvedMedia<'_>, MediaError> {
    let prefix = kind.env_prefix();
    let provider = config.provider.trim();
    let provider = if provider.is_empty() {
        kind.default_provider()
    } else {
        provider
    };
    let api_key = config.api_key.trim();
    if api_key.is_empty() {
        return Err(MediaError::Unconfigured(format!(
            "set {prefix}_API_KEY (and optionally {prefix}_BASE_URL / {prefix}_MODEL), \
             or call the capability's configure_from_env()"
        )));
    }
    let model = config.model.trim();
    let model = if model.is_empty() {
        kind.default_model()
    } else {
        model
    };
    let configured_base_url = config.base_url.trim().trim_end_matches('/');
    let base_url = if configured_base_url.is_empty() {
        kind.default_base_url()
    } else {
        configured_base_url
    };
    if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
        return Err(MediaError::Config(format!(
            "{prefix}_BASE_URL is not an http(s) URL: `{base_url}`"
        )));
    }
    Ok(ResolvedMedia {
        provider,
        base_url,
        api_key,
        model,
    })
}

fn openai_client(api_key: &str, base_url: &str) -> OpenAI {
    OpenAIConfig::new(api_key.to_owned())
        .with_base_url(base_url.to_owned())
        .client()
}

/// Opaque image-generation model handle (rig types stay inside this kit).
#[derive(Clone)]
pub struct ImageModel {
    inner: DynModel<ImageGeneration>,
}

/// Opaque speech-synthesis model handle (rig types stay inside this kit).
#[derive(Clone)]
pub struct AudioModel {
    inner: DynModel<AudioGeneration>,
}

/// Opaque transcription model handle (rig types stay inside this kit).
#[derive(Clone)]
pub struct TranscriptionModel {
    inner: DynModel<Transcription>,
}

/// Opaque rerank model handle (rig types stay inside this kit).
#[derive(Clone)]
pub struct RerankModel {
    inner: DynModel<Rerank>,
}

impl fmt::Debug for ImageModel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ImageModel")
    }
}

impl fmt::Debug for AudioModel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AudioModel")
    }
}

impl fmt::Debug for TranscriptionModel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("TranscriptionModel")
    }
}

impl fmt::Debug for RerankModel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RerankModel")
    }
}

/// Build an image-generation model from explicit [`MediaConfig`].
///
/// # Errors
///
/// [`MediaError::Unconfigured`] when `api_key` is empty,
/// [`MediaError::Config`] for an invalid base URL or a provider other than
/// `openai`.
pub fn image_model_from_config(config: &MediaConfig) -> Result<ImageModel, MediaError> {
    let resolved = resolve_media(MediaKind::Image, config)?;
    match resolved.provider {
        "openai" => Ok(ImageModel {
            inner: openai_client(resolved.api_key, resolved.base_url)
                .image_generation(resolved.model.to_owned())
                .into(),
        }),
        other => Err(MediaError::Config(format!(
            "image provider `{other}` is unsupported (rig-core openai wire); \
             set ZEOKEEL_IMAGE_PROVIDER=openai"
        ))),
    }
}

/// Build the image-generation model from `ZEOKEEL_IMAGE_*`.
///
/// # Errors
///
/// As [`image_model_from_config`].
pub fn image_model() -> Result<ImageModel, MediaError> {
    image_model_from_config(&MediaConfig::from_env(MediaKind::Image))
}

/// Build a speech (text-to-speech) model from explicit [`MediaConfig`].
///
/// # Errors
///
/// As [`image_model_from_config`].
pub fn audio_model_from_config(config: &MediaConfig) -> Result<AudioModel, MediaError> {
    let resolved = resolve_media(MediaKind::Audio, config)?;
    match resolved.provider {
        "openai" => Ok(AudioModel {
            inner: openai_client(resolved.api_key, resolved.base_url)
                .audio_generation(resolved.model.to_owned())
                .into(),
        }),
        other => Err(MediaError::Config(format!(
            "audio provider `{other}` is unsupported (rig-core openai wire); \
             set ZEOKEEL_AUDIO_PROVIDER=openai"
        ))),
    }
}

/// Build the speech model from `ZEOKEEL_AUDIO_*`.
///
/// # Errors
///
/// As [`audio_model_from_config`].
pub fn audio_model() -> Result<AudioModel, MediaError> {
    audio_model_from_config(&MediaConfig::from_env(MediaKind::Audio))
}

/// Build a transcription model from explicit [`MediaConfig`].
///
/// # Errors
///
/// As [`image_model_from_config`].
pub fn transcription_model_from_config(
    config: &MediaConfig,
) -> Result<TranscriptionModel, MediaError> {
    let resolved = resolve_media(MediaKind::Transcription, config)?;
    match resolved.provider {
        "openai" => Ok(TranscriptionModel {
            inner: openai_client(resolved.api_key, resolved.base_url)
                .transcription(resolved.model.to_owned())
                .into(),
        }),
        other => Err(MediaError::Config(format!(
            "transcription provider `{other}` is unsupported (rig-core openai wire); \
             set ZEOKEEL_TRANSCRIPTION_PROVIDER=openai"
        ))),
    }
}

/// Build the transcription model from `ZEOKEEL_TRANSCRIPTION_*`.
///
/// # Errors
///
/// As [`transcription_model_from_config`].
pub fn transcription_model() -> Result<TranscriptionModel, MediaError> {
    transcription_model_from_config(&MediaConfig::from_env(MediaKind::Transcription))
}

/// Build a rerank model from explicit [`MediaConfig`].
///
/// rig-core 0.45 has no OpenAI rerank endpoint (`RerankQuirks::unsupported`);
/// Voyage AI is the only provider it ships a rerank wire for, so `voyageai` is
/// the supported (and default) provider.
///
/// # Errors
///
/// [`MediaError::Unconfigured`] when `api_key` is empty,
/// [`MediaError::Config`] for an invalid base URL, `openai` (no endpoint), or
/// any other provider.
pub fn rerank_model_from_config(config: &MediaConfig) -> Result<RerankModel, MediaError> {
    let resolved = resolve_media(MediaKind::Rerank, config)?;
    match resolved.provider {
        "voyageai" => {
            let client = VoyageAiConfig::new(resolved.api_key.to_owned())
                .with_base_url(resolved.base_url)
                .client();
            Ok(RerankModel {
                inner: client.rerank(resolved.model.to_owned()).into(),
            })
        }
        "openai" => Err(MediaError::Config(
            "rig-core's openai wire exposes no rerank endpoint (RerankQuirks::unsupported); \
             set ZEOKEEL_RERANK_PROVIDER=voyageai"
                .to_owned(),
        )),
        other => Err(MediaError::Config(format!(
            "rerank provider `{other}` is unsupported (supported: voyageai); \
             set ZEOKEEL_RERANK_PROVIDER=voyageai"
        ))),
    }
}

/// Build the rerank model from `ZEOKEEL_RERANK_*`.
///
/// # Errors
///
/// As [`rerank_model_from_config`].
pub fn rerank_model() -> Result<RerankModel, MediaError> {
    rerank_model_from_config(&MediaConfig::from_env(MediaKind::Rerank))
}

/// One generated image, as the provider returned it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageOutput {
    /// Encoded image bytes.
    pub bytes: Vec<u8>,
    /// Provider descriptor name rig reports (`"openai"`).
    pub provider: String,
    /// Provider-reported model id, when the reply named one.
    pub model: Option<String>,
}

/// One synthesized audio clip, as the provider returned it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioOutput {
    /// Encoded audio bytes (provider container, e.g. MP3).
    pub bytes: Vec<u8>,
    /// Provider descriptor name rig reports (`"openai"`).
    pub provider: String,
    /// Provider-reported model id, when the reply named one.
    pub model: Option<String>,
}

/// One transcript, as the provider returned it.
#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptionOutput {
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

/// One reranked document, in provider rank order (highest relevance first).
#[derive(Debug, Clone, PartialEq)]
pub struct RerankOutput {
    /// Index of the document in the caller's input list.
    pub index: usize,
    /// Provider relevance score; higher is more relevant within this response.
    pub score: f64,
    /// The document text, when the provider echoed it back.
    pub document: Option<String>,
}

/// Generate one image from `prompt` at `width`×`height`.
///
/// # Errors
///
/// [`MediaError::Provider`] when the provider call fails, and
/// [`MediaError::Runtime`] when the shared runtime cannot start.
pub fn generate_image(
    model: &ImageModel,
    prompt: &str,
    width: u32,
    height: u32,
) -> Result<ImageOutput, MediaError> {
    let request = ImageGenerationRequestBuilder::new(prompt)
        .width(width)
        .height(height)
        .build();
    let response = block_on(model.inner.call(request))?
        .map_err(|error| MediaError::Provider(error.to_string()))?;
    Ok(ImageOutput {
        bytes: response.image,
        provider: response.provider,
        model: response.model,
    })
}

/// Synthesize speech for `text` in provider `voice`.
///
/// # Errors
///
/// [`MediaError::Provider`] when the provider call fails, and
/// [`MediaError::Runtime`] when the shared runtime cannot start.
pub fn synthesize_audio(
    model: &AudioModel,
    text: &str,
    voice: &str,
) -> Result<AudioOutput, MediaError> {
    let request = AudioGenerationRequestBuilder::new(text, voice).build();
    let response = block_on(model.inner.call(request))?
        .map_err(|error| MediaError::Provider(error.to_string()))?;
    Ok(AudioOutput {
        bytes: response.audio,
        provider: response.provider,
        model: response.model,
    })
}

/// Transcribe encoded audio bytes; `filename` names the audio for the provider.
///
/// # Errors
///
/// [`MediaError::Provider`] when the provider call fails, and
/// [`MediaError::Runtime`] when the shared runtime cannot start.
pub fn transcribe_audio(
    model: &TranscriptionModel,
    data: &[u8],
    filename: &str,
) -> Result<TranscriptionOutput, MediaError> {
    let request = TranscriptionRequestBuilder::new(data.to_vec())
        .filename(filename.to_owned())
        .build();
    let response = block_on(model.inner.call(request))?
        .map_err(|error| MediaError::Provider(error.to_string()))?;
    Ok(TranscriptionOutput {
        text: response.text,
        language: response.language,
        duration: response.duration,
        provider: response.provider,
        model: response.model,
    })
}

/// Rank `documents` by relevance to `query`, highest first.
///
/// # Errors
///
/// [`MediaError::Provider`] when the provider call fails, and
/// [`MediaError::Runtime`] when the shared runtime cannot start.
pub fn rerank_documents(
    model: &RerankModel,
    query: &str,
    documents: &[String],
) -> Result<Vec<RerankOutput>, MediaError> {
    let request = RerankRequest {
        query: query.to_owned(),
        documents: documents.to_vec(),
    };
    let response = block_on(model.inner.call(request))?
        .map_err(|error| MediaError::Provider(error.to_string()))?;
    Ok(response
        .results
        .into_iter()
        .map(|result| RerankOutput {
            index: result.index,
            score: result.relevance_score,
            document: result.document,
        })
        .collect())
}

static MEDIA_RUNTIME: OnceLock<Result<tokio::runtime::Runtime, String>> = OnceLock::new();

/// Run a buffered rig provider call from synchronous capability code.
///
/// The capability packages expose synchronous APIs, while rig models are
/// async; this is the single shared bridge so the capabilities do not each
/// build their own tokio runtime.
///
/// # Errors
///
/// [`MediaError::Runtime`] when the shared tokio runtime cannot be created.
///
/// # Panics
///
/// Tokio refuses to start a runtime from inside another runtime; call this
/// from a synchronous host path, not from an async task.
pub fn block_on<F: std::future::Future>(future: F) -> Result<F::Output, MediaError> {
    let runtime = MEDIA_RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|error| format!("tokio runtime: {error}"))
        })
        .as_ref()
        .map_err(|error| MediaError::Runtime(error.clone()))?;
    Ok(runtime.block_on(future))
}

/// One process-wide model slot a capability package installs its model into.
///
/// This is shared plumbing: configure/unconfigure/availability semantics live
/// here once instead of being copied into every `cap-*` package (C17).
pub struct ModelSlot<M> {
    cell: OnceLock<Mutex<Option<M>>>,
}

impl<M> ModelSlot<M> {
    /// An empty slot.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            cell: OnceLock::new(),
        }
    }

    fn slot(&self) -> &Mutex<Option<M>> {
        self.cell.get_or_init(|| Mutex::new(None))
    }
}

impl<M> Default for ModelSlot<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Clone> ModelSlot<M> {
    /// Whether a model is installed right now (false when the lock is poisoned).
    #[must_use]
    pub fn is_configured(&self) -> bool {
        self.slot()
            .lock()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }

    /// A clone of the installed model, or `None` when unconfigured.
    ///
    /// # Errors
    ///
    /// [`MediaError::Config`] when the slot lock is poisoned.
    pub fn get(&self) -> Result<Option<M>, MediaError> {
        self.slot()
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| MediaError::Config("media model slot lock poisoned".to_owned()))
    }

    /// Install `model`.
    ///
    /// # Errors
    ///
    /// [`MediaError::Config`] when the slot lock is poisoned.
    pub fn set(&self, model: M) -> Result<(), MediaError> {
        let mut guard = self
            .slot()
            .lock()
            .map_err(|_| MediaError::Config("media model slot lock poisoned".to_owned()))?;
        *guard = Some(model);
        Ok(())
    }

    /// Remove the installed model.
    ///
    /// # Errors
    ///
    /// [`MediaError::Config`] when the slot lock is poisoned.
    pub fn clear(&self) -> Result<(), MediaError> {
        let mut guard = self
            .slot()
            .lock()
            .map_err(|_| MediaError::Config("media model slot lock poisoned".to_owned()))?;
        *guard = None;
        Ok(())
    }
}

/// Default embedding model id used when `ZEOKEEL_EMBEDDING_MODEL` is unset.
pub const DEFAULT_EMBEDDING_MODEL: &str = "text-embedding-3-small";

/// Configuration for one OpenAI-compatible embedding model.
///
/// The [`Debug`] implementation redacts `api_key` so logging a config never
/// leaks the bearer key.
#[derive(Clone, Default)]
pub struct EmbeddingConfig {
    /// API root, e.g. `https://api.openai.com/v1` (required).
    pub base_url: String,
    /// Bearer key (required).
    pub api_key: String,
    /// Model id; `embedding_model()` falls back to [`DEFAULT_EMBEDDING_MODEL`].
    pub model: String,
    /// Requested output width, when the endpoint supports shortening vectors.
    pub dims: Option<usize>,
}

impl EmbeddingConfig {
    /// Explicit configuration.
    #[must_use]
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
        dims: Option<usize>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: api_key.into(),
            model: model.into(),
            dims,
        }
    }

    /// Resolve from `ZEOKEEL_EMBEDDING_{BASE_URL,API_KEY,MODEL,DIMS}`.
    ///
    /// Unset variables stay empty (`is_configured` is then false); an invalid
    /// `ZEOKEEL_EMBEDDING_DIMS` is a typed config error.
    ///
    /// # Errors
    ///
    /// [`EmbeddingError::Config`] when `ZEOKEEL_EMBEDDING_DIMS` is not a `usize`.
    pub fn from_env() -> Result<Self, EmbeddingError> {
        let dims = match env_prefixed("ZEOKEEL_EMBEDDING", "DIMS") {
            value if value.is_empty() => None,
            value => Some(value.parse::<usize>().map_err(|error| {
                EmbeddingError::Config(format!("ZEOKEEL_EMBEDDING_DIMS: {error}"))
            })?),
        };
        Ok(Self {
            base_url: env_prefixed("ZEOKEEL_EMBEDDING", "BASE_URL"),
            api_key: env_prefixed("ZEOKEEL_EMBEDDING", "API_KEY"),
            model: env_prefixed("ZEOKEEL_EMBEDDING", "MODEL"),
            dims,
        })
    }

    /// Whether base URL, key and model are all present.
    #[must_use]
    pub fn is_configured(&self) -> bool {
        !(self.base_url.trim().is_empty()
            || self.api_key.trim().is_empty()
            || self.model.trim().is_empty())
    }
}

impl fmt::Debug for EmbeddingConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EmbeddingConfig")
            .field("base_url", &self.base_url)
            .field("api_key", &redacted_key(&self.api_key))
            .field("model", &self.model)
            .field("dims", &self.dims)
            .finish()
    }
}

/// Typed errors from the embedding adapter.
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    /// No embedding endpoint is configured.
    #[error("embedding model not configured: {0}")]
    Unconfigured(String),
    /// The endpoint/model/dimensions configuration is invalid.
    #[error("embedding config: {0}")]
    Config(String),
    /// The embedding provider call failed.
    #[error("embedding provider: {0}")]
    Provider(String),
    /// The shared blocking runtime could not be created.
    #[error("embedding runtime: {0}")]
    Runtime(String),
}

impl From<MediaError> for EmbeddingError {
    fn from(error: MediaError) -> Self {
        match error {
            MediaError::Unconfigured(message) => Self::Unconfigured(message),
            MediaError::Config(message) => Self::Config(message),
            MediaError::Provider(message) => Self::Provider(message),
            MediaError::Runtime(message) => Self::Runtime(message),
        }
    }
}

/// Opaque embedding-model handle (rig types stay inside this kit).
#[derive(Clone)]
pub struct EmbeddingModel {
    inner: DynModel<Embedding>,
}

impl fmt::Debug for EmbeddingModel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("EmbeddingModel")
    }
}

/// Build an OpenAI-compatible embedding model from explicit [`EmbeddingConfig`].
///
/// # Errors
///
/// [`EmbeddingError::Unconfigured`] when base URL/key/model are empty,
/// [`EmbeddingError::Config`] when the base URL is not http(s).
pub fn embedding_model_from_config(
    config: &EmbeddingConfig,
) -> Result<EmbeddingModel, EmbeddingError> {
    let base_url = config.base_url.trim().trim_end_matches('/');
    let api_key = config.api_key.trim();
    let model = config.model.trim();
    if base_url.is_empty() || api_key.is_empty() || model.is_empty() {
        return Err(EmbeddingError::Unconfigured(
            "set ZEOKEEL_EMBEDDING_BASE_URL/ZEOKEEL_EMBEDDING_API_KEY/ZEOKEEL_EMBEDDING_MODEL \
             (and optionally ZEOKEEL_EMBEDDING_DIMS), or call the capability's \
             configure_from_env()"
                .to_owned(),
        ));
    }
    if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
        return Err(EmbeddingError::Config(format!(
            "ZEOKEEL_EMBEDDING_BASE_URL is not an http(s) URL: `{base_url}`"
        )));
    }
    let client = OpenAIConfig::new(api_key.to_owned())
        .with_base_url(base_url.to_owned())
        .client();
    Ok(EmbeddingModel {
        inner: client.embedding(model.to_owned(), config.dims).into(),
    })
}

/// Build the embedding model from `ZEOKEEL_EMBEDDING_*`.
///
/// `ZEOKEEL_EMBEDDING_MODEL` defaults to [`DEFAULT_EMBEDDING_MODEL`].
///
/// # Errors
///
/// As [`embedding_model_from_config`].
pub fn embedding_model() -> Result<EmbeddingModel, EmbeddingError> {
    let mut config = EmbeddingConfig::from_env()?;
    if config.model.trim().is_empty() {
        config.model = DEFAULT_EMBEDDING_MODEL.to_owned();
    }
    embedding_model_from_config(&config)
}

/// Embed one text with `model`.
///
/// # Errors
///
/// [`EmbeddingError::Provider`] when the provider call fails, and
/// [`EmbeddingError::Runtime`] when the shared runtime cannot start.
pub fn embed_text(model: &EmbeddingModel, text: &str) -> Result<Vec<f64>, EmbeddingError> {
    let vector = block_on(model.inner.embed_text(text))?
        .map_err(|error| EmbeddingError::Provider(error.to_string()))?;
    Ok(vector.vec)
}

fn env_prefixed(prefix: &str, suffix: &str) -> String {
    std::env::var(format!("{prefix}_{suffix}"))
        .map(|value| value.trim().to_owned())
        .unwrap_or_default()
}

/// Deterministic in-process models for capability tests.
///
/// No network and no provider: each rig `Local<Op>` wire is paired with a fake
/// transport that echoes the request facts into the reply, so a capability test
/// can prove the request reached the model and its reply was mapped correctly.
#[cfg(any(test, feature = "test-support"))]
pub mod test_support {
    use futures::stream;
    use rig_core::audio_generation::{AudioGenerationRequest, AudioGenerationResponse};
    use rig_core::driver::{Exchange, Local, Model, Opened, Opening, Step, Transport};
    use rig_core::embeddings::{Embedding as Vector, EmbeddingResponse};
    use rig_core::image_generation::{ImageGenerationRequest, ImageGenerationResponse};
    use rig_core::operation::{
        AudioGeneration, Embedding, ImageGeneration, Rerank, RerankRequest, Transcription,
    };
    use rig_core::rerank::{RerankResponse, RerankResult};
    use rig_core::transcription::{TranscriptionRequest, TranscriptionResponse};
    use rig_core::DynModel;

    use super::{AudioModel, EmbeddingModel, ImageModel, RerankModel, TranscriptionModel};

    /// Provider name the fake wires report.
    pub const FAKE_PROVIDER: &str = "fake-provider";

    #[derive(Clone)]
    struct FakeImageTransport;

    impl Transport<Local<ImageGeneration>> for FakeImageTransport {
        fn send(
            &self,
            payload: ImageGenerationRequest,
            _exchange: Exchange,
        ) -> Opening<Step<ImageGeneration>> {
            let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
            bytes.extend_from_slice(
                format!("{}:{}x{}", payload.prompt, payload.width, payload.height).as_bytes(),
            );
            let mut response = ImageGenerationResponse::new(bytes);
            response.model = Some("fake-image-model".to_owned());
            Opening::ready(Opened::new(stream::once(
                async move { Ok(Step::End(response)) },
            )))
        }
    }

    #[derive(Clone)]
    struct FakeAudioTransport;

    impl Transport<Local<AudioGeneration>> for FakeAudioTransport {
        fn send(
            &self,
            payload: AudioGenerationRequest,
            _exchange: Exchange,
        ) -> Opening<Step<AudioGeneration>> {
            let mut audio = format!("{}|{}", payload.text, payload.voice).into_bytes();
            audio.extend_from_slice(&[0x00, 0x01]);
            let mut response = AudioGenerationResponse::new(audio);
            response.model = Some("fake-tts-model".to_owned());
            Opening::ready(Opened::new(stream::once(
                async move { Ok(Step::End(response)) },
            )))
        }
    }

    #[derive(Clone)]
    struct FakeTranscriptionTransport;

    impl Transport<Local<Transcription>> for FakeTranscriptionTransport {
        fn send(
            &self,
            payload: TranscriptionRequest,
            _exchange: Exchange,
        ) -> Opening<Step<Transcription>> {
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

    #[derive(Clone)]
    struct FakeRerankTransport;

    impl Transport<Local<Rerank>> for FakeRerankTransport {
        fn send(&self, payload: RerankRequest, _exchange: Exchange) -> Opening<Step<Rerank>> {
            let mut results: Vec<RerankResult> = payload
                .documents
                .iter()
                .enumerate()
                .map(|(index, document)| RerankResult {
                    index,
                    document: Some(document.clone()),
                    relevance_score: if document.contains(&payload.query) {
                        1.0
                    } else {
                        0.0
                    },
                })
                .collect();
            results.sort_by(|left, right| right.relevance_score.total_cmp(&left.relevance_score));
            let mut response = RerankResponse::new(results);
            response.model = Some("fake-rerank-model".to_owned());
            Opening::ready(Opened::new(stream::once(
                async move { Ok(Step::End(response)) },
            )))
        }
    }

    impl ImageModel {
        fn fake() -> Self {
            let model: DynModel<ImageGeneration> = Model::new(
                Local::<ImageGeneration>::new(FAKE_PROVIDER),
                FakeImageTransport,
            )
            .into();
            Self { inner: model }
        }
    }

    impl AudioModel {
        fn fake() -> Self {
            let model: DynModel<AudioGeneration> = Model::new(
                Local::<AudioGeneration>::new(FAKE_PROVIDER),
                FakeAudioTransport,
            )
            .into();
            Self { inner: model }
        }
    }

    impl TranscriptionModel {
        fn fake() -> Self {
            let model: DynModel<Transcription> = Model::new(
                Local::<Transcription>::new(FAKE_PROVIDER),
                FakeTranscriptionTransport,
            )
            .into();
            Self { inner: model }
        }
    }

    impl RerankModel {
        fn fake() -> Self {
            let model: DynModel<Rerank> =
                Model::new(Local::<Rerank>::new(FAKE_PROVIDER), FakeRerankTransport).into();
            Self { inner: model }
        }
    }

    /// A deterministic image model (PNG magic + `prompt:WxH` payload).
    #[must_use]
    pub fn fake_image_model() -> ImageModel {
        ImageModel::fake()
    }

    /// A deterministic speech model (audio bytes are `text|voice\0\x01`).
    #[must_use]
    pub fn fake_audio_model() -> AudioModel {
        AudioModel::fake()
    }

    /// A deterministic transcription model (text is `filename:data`).
    #[must_use]
    pub fn fake_transcription_model() -> TranscriptionModel {
        TranscriptionModel::fake()
    }

    /// A deterministic rerank model (1.0 when a document contains the query).
    #[must_use]
    pub fn fake_rerank_model() -> RerankModel {
        RerankModel::fake()
    }

    #[derive(Clone)]
    struct FakeEmbeddingTransport;

    impl Transport<Local<Embedding>> for FakeEmbeddingTransport {
        fn send(&self, payload: Vec<String>, _exchange: Exchange) -> Opening<Step<Embedding>> {
            let vectors = payload
                .into_iter()
                .map(|text| Vector {
                    document: text.clone(),
                    vec: fake_embedding_vector(&text),
                })
                .collect();
            Opening::ready(Opened::new(stream::once(async move {
                Ok(Step::End(EmbeddingResponse::new(vectors)))
            })))
        }
    }

    fn fake_embedding_vector(text: &str) -> Vec<f64> {
        let alpha = f64::from(u8::from(text.contains("alpha")));
        let beta = f64::from(u8::from(text.contains("beta")));
        vec![alpha, beta, 1.0]
    }

    impl EmbeddingModel {
        fn fake() -> Self {
            let model: DynModel<Embedding> = Model::new(
                Local::<Embedding>::new("fake-embed"),
                FakeEmbeddingTransport,
            )
            .into();
            Self { inner: model }
        }
    }

    /// A deterministic embedding model (`[alpha, beta, 1.0]` from the text).
    #[must_use]
    pub fn fake_embedding_model() -> EmbeddingModel {
        EmbeddingModel::fake()
    }
}

#[cfg(test)]
mod tests;
