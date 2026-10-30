//! rig model provider for personal edition.
//!
//! @capability llm.provider.rig
#![forbid(unsafe_code)]

use std::sync::OnceLock;

use rig_core::operation::{
    AudioGeneration, Completion, Embedding, ImageGeneration, Rerank, Transcription,
};
use rig_core::providers::openai::{OpenAI, OpenAIConfig};
use rig_core::providers::voyageai::VoyageAiConfig;
use rig_core::DynModel;
use zk_contract_llm::{LlmError, LlmProvider};

/// Provider configuration resolved from environment/settings.
#[derive(Debug, Clone, Default)]
pub struct RigConfig {
    /// Provider: openai | deepseek | anthropic | ollama.
    pub provider: String,
    /// OpenAI-compatible endpoint (empty => provider default).
    pub base_url: String,
    /// Bearer key.
    pub api_key: String,
    /// Model id.
    pub model: String,
}

impl RigConfig {
    /// Resolve from `ZEOKEEL_*` environment (live-dev fallback).
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            provider: {
                let provider = env_first(&["ZEOKEEL_RIG_PROVIDER"]);
                if provider.is_empty() {
                    "openai".to_owned()
                } else {
                    provider
                }
            },
            base_url: env_first(&["ZEOKEEL_OPENAI_BASE_URL", "ZEOKEEL_DEV_OPENAI_BASE_URL"]),
            api_key: env_first(&["ZEOKEEL_OPENAI_API_KEY", "ZEOKEEL_DEV_OPENAI_API_KEY"]),
            model: {
                let model = env_first(&["ZEOKEEL_OPENAI_MODEL", "ZEOKEEL_DEV_MODEL"]);
                if model.is_empty() {
                    "gpt-4o-mini".to_owned()
                } else {
                    model
                }
            },
        }
    }
}

fn env_first(keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| {
            std::env::var(key)
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_default()
}

/// Build a model from explicit fields (settings hot-swap).
pub fn model_from_config(
    provider: &str,
    base_url: &str,
    api_key: &str,
    model: &str,
) -> Result<DynModel<Completion>, LlmError> {
    model_from(&RigConfig {
        provider: provider.to_owned(),
        base_url: base_url.to_owned(),
        api_key: api_key.to_owned(),
        model: model.to_owned(),
    })
}

/// Build the rig completion model.
pub fn model_from(config: &RigConfig) -> Result<DynModel<Completion>, LlmError> {
    if config.base_url.trim().is_empty() && config.api_key.trim().is_empty() {
        return Err(LlmError::InvalidRequest(
            "rig provider not configured (base_url/api_key)".to_owned(),
        ));
    }
    if config.model.trim().is_empty() {
        return Err(LlmError::InvalidRequest("rig model is empty".to_owned()));
    }
    match config.provider.as_str() {
        "openai" => {
            let openai = OpenAIConfig::new(config.api_key.clone())
                .with_base_url(config.base_url.trim_end_matches('/').to_owned())
                .client();
            Ok(openai.chat(config.model.clone()).into())
        }
        "deepseek" => {
            let base = if config.base_url.trim().is_empty() {
                "https://api.deepseek.com".to_owned()
            } else {
                config.base_url.trim_end_matches('/').to_owned()
            };
            let client = OpenAIConfig::with_key(
                &rig_core::providers::openai::wire::DEEPSEEK,
                config.api_key.clone(),
            )
            .with_base_url(base)
            .client();
            Ok(client.chat(config.model.clone()).into())
        }
        "anthropic" => {
            if config.api_key.trim().is_empty() {
                return Err(LlmError::InvalidRequest(
                    "anthropic api_key is empty".to_owned(),
                ));
            }
            let mut anthropic = rig_core::providers::anthropic::AnthropicConfig::with_key(
                &rig_core::providers::anthropic::ANTHROPIC,
                config.api_key.clone(),
            );
            if !config.base_url.trim().is_empty() {
                anthropic = anthropic.with_base_url(config.base_url.trim_end_matches('/'));
            }
            Ok(anthropic.client().completion(config.model.clone()).into())
        }
        "ollama" => {
            let base = if config.base_url.trim().is_empty() {
                "http://127.0.0.1:11434".to_owned()
            } else {
                config.base_url.trim_end_matches('/').to_owned()
            };
            let client = rig_core::providers::ollama::OllamaConfig::new()
                .with_base_url(base)
                .with_api_key(config.api_key.clone())
                .client();
            Ok(client.completion(config.model.clone()).into())
        }
        other => Err(LlmError::InvalidRequest(format!(
            "unknown rig provider: {other}"
        ))),
    }
}

/// Base URL of the placeholder model used before a provider is configured.
///
/// It points at a closed local port so a turn started before configuration fails
/// loudly at the transport layer instead of silently using a mock model.
const UNCONFIGURED_BASE_URL: &str = "http://127.0.0.1:9/v1";

/// Build a placeholder model for an install without provider configuration.
///
/// The composed desktop cannot start without a `DynModel`; persisted settings are
/// applied immediately after `build()` via the settings hot-swap path (ADR-064).
pub fn unconfigured_model() -> Result<DynModel<Completion>, LlmError> {
    model_from(&RigConfig {
        provider: "openai".to_owned(),
        base_url: UNCONFIGURED_BASE_URL.to_owned(),
        api_key: "unconfigured".to_owned(),
        model: "unconfigured".to_owned(),
    })
}

/// Default factory used by generated glue.
///
/// Resolves the provider from `ZEOKEEL_*` env; when nothing is configured it
/// returns [`unconfigured_model`] so the desktop can boot and then apply persisted
/// settings (ADR-064). A partially configured provider still returns an error so
/// mistakes surface during startup.
pub fn model() -> Result<DynModel<Completion>, LlmError> {
    let config = RigConfig::from_env();
    if config.base_url.trim().is_empty() && config.api_key.trim().is_empty() {
        return unconfigured_model();
    }
    model_from(&config)
}

/// Default embedding model id for OpenAI-compatible endpoints.
pub const DEFAULT_EMBEDDING_MODEL: &str = "text-embedding-3-small";

/// Build an embedding model for an OpenAI-compatible endpoint.
///
/// `base_url` must be the API root (e.g. `https://api.openai.com/v1`); `dims`
/// asks providers that support shortening output vectors for that width.
///
/// # Errors
///
/// [`LlmError::InvalidRequest`] when the endpoint, key or model is empty.
pub fn embedding_model_from_config(
    base_url: &str,
    api_key: &str,
    model: &str,
    dims: Option<usize>,
) -> Result<DynModel<Embedding>, LlmError> {
    if base_url.trim().is_empty() {
        return Err(LlmError::InvalidRequest(
            "embedding base_url is empty (set ZEOKEEL_EMBEDDING_BASE_URL)".to_owned(),
        ));
    }
    if api_key.trim().is_empty() {
        return Err(LlmError::InvalidRequest(
            "embedding api_key is empty (set ZEOKEEL_EMBEDDING_API_KEY)".to_owned(),
        ));
    }
    if model.trim().is_empty() {
        return Err(LlmError::InvalidRequest(
            "embedding model is empty (set ZEOKEEL_EMBEDDING_MODEL)".to_owned(),
        ));
    }
    let client = OpenAIConfig::new(api_key.trim().to_owned())
        .with_base_url(base_url.trim_end_matches('/').to_owned())
        .client();
    Ok(client.embedding(model.trim().to_owned(), dims).into())
}

/// Build the personal-edition embedding model from `ZEOKEEL_EMBEDDING_*`
/// (falling back to the main provider's OpenAI-compatible endpoint/key).
///
/// # Errors
///
/// As [`embedding_model_from_config`] when no embedding endpoint/key/model is
/// configured, and [`LlmError::InvalidRequest`] for an invalid dims value.
pub fn embedding_model() -> Result<DynModel<Embedding>, LlmError> {
    let base_url = env_first(&[
        "ZEOKEEL_EMBEDDING_BASE_URL",
        "ZEOKEEL_OPENAI_BASE_URL",
        "ZEOKEEL_DEV_OPENAI_BASE_URL",
    ]);
    let api_key = env_first(&[
        "ZEOKEEL_EMBEDDING_API_KEY",
        "ZEOKEEL_OPENAI_API_KEY",
        "ZEOKEEL_DEV_OPENAI_API_KEY",
    ]);
    let model = {
        let model = env_first(&["ZEOKEEL_EMBEDDING_MODEL"]);
        if model.is_empty() {
            DEFAULT_EMBEDDING_MODEL.to_owned()
        } else {
            model
        }
    };
    let dims = {
        let value = env_first(&["ZEOKEEL_EMBEDDING_DIMS"]);
        if value.is_empty() {
            None
        } else {
            Some(value.parse::<usize>().map_err(|error| {
                LlmError::InvalidRequest(format!("ZEOKEEL_EMBEDDING_DIMS: {error}"))
            })?)
        }
    };
    embedding_model_from_config(&base_url, &api_key, &model, dims)
}

// ---------------------------------------------------------------------------
// Media models: image generation, speech, transcription, rerank.
//
// rig-core gates its image/audio wires behind the `image`/`audio` features
// (enabled for the workspace in the root `Cargo.toml`). Reranking has no
// OpenAI endpoint in rig-core (`RerankQuirks::unsupported`); its only real
// provider in 0.45 is Voyage AI, which is what `MediaKind::Rerank` defaults to.
// ---------------------------------------------------------------------------

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
}

/// Configuration for one media model (image/audio/transcription/rerank).
///
/// Empty `provider`/`model` fall back to the [`MediaKind`] defaults; an empty
/// `base_url` falls back to the provider's API root. `api_key` is required.
#[derive(Debug, Clone, Default)]
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

/// Typed errors from the media model builders.
#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    /// No media provider is configured (`<PREFIX>_API_KEY` is empty).
    #[error("media model not configured: {0}")]
    Unconfigured(String),
    /// The provider/base URL/unsupported-provider configuration is invalid.
    #[error("media model config: {0}")]
    Config(String),
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

/// Build an image-generation model from explicit [`MediaConfig`].
///
/// # Errors
///
/// [`MediaError::Unconfigured`] when `api_key` is empty,
/// [`MediaError::Config`] for an invalid base URL or a provider other than
/// `openai`.
pub fn image_model_from_config(
    config: &MediaConfig,
) -> Result<DynModel<ImageGeneration>, MediaError> {
    let resolved = resolve_media(MediaKind::Image, config)?;
    match resolved.provider {
        "openai" => Ok(openai_client(resolved.api_key, resolved.base_url)
            .image_generation(resolved.model.to_owned())
            .into()),
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
pub fn image_model() -> Result<DynModel<ImageGeneration>, MediaError> {
    image_model_from_config(&MediaConfig::from_env(MediaKind::Image))
}

/// Build a speech (text-to-speech) model from explicit [`MediaConfig`].
///
/// # Errors
///
/// As [`image_model_from_config`].
pub fn audio_model_from_config(
    config: &MediaConfig,
) -> Result<DynModel<AudioGeneration>, MediaError> {
    let resolved = resolve_media(MediaKind::Audio, config)?;
    match resolved.provider {
        "openai" => Ok(openai_client(resolved.api_key, resolved.base_url)
            .audio_generation(resolved.model.to_owned())
            .into()),
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
pub fn audio_model() -> Result<DynModel<AudioGeneration>, MediaError> {
    audio_model_from_config(&MediaConfig::from_env(MediaKind::Audio))
}

/// Build a transcription model from explicit [`MediaConfig`].
///
/// # Errors
///
/// As [`image_model_from_config`].
pub fn transcription_model_from_config(
    config: &MediaConfig,
) -> Result<DynModel<Transcription>, MediaError> {
    let resolved = resolve_media(MediaKind::Transcription, config)?;
    match resolved.provider {
        "openai" => Ok(openai_client(resolved.api_key, resolved.base_url)
            .transcription(resolved.model.to_owned())
            .into()),
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
pub fn transcription_model() -> Result<DynModel<Transcription>, MediaError> {
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
pub fn rerank_model_from_config(config: &MediaConfig) -> Result<DynModel<Rerank>, MediaError> {
    let resolved = resolve_media(MediaKind::Rerank, config)?;
    match resolved.provider {
        "voyageai" => {
            let client = VoyageAiConfig::new(resolved.api_key.to_owned())
                .with_base_url(resolved.base_url)
                .client();
            Ok(client.rerank(resolved.model.to_owned()).into())
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
pub fn rerank_model() -> Result<DynModel<Rerank>, MediaError> {
    rerank_model_from_config(&MediaConfig::from_env(MediaKind::Rerank))
}

static MEDIA_RUNTIME: OnceLock<Result<tokio::runtime::Runtime, String>> = OnceLock::new();

/// Run a buffered rig provider call from synchronous capability code.
///
/// The capability packages (`cap-image`/`cap-audio`/`cap-transcription`/
/// `cap-rerank`) expose synchronous APIs, while rig models are async; this is
/// the single shared bridge so the runtimes are not duplicated per capability.
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

fn env_prefixed(prefix: &str, suffix: &str) -> String {
    std::env::var(format!("{prefix}_{suffix}"))
        .map(|value| value.trim().to_owned())
        .unwrap_or_default()
}

/// Legacy provider adapter kept for package contract compatibility.
#[derive(Debug, Clone, Default)]
pub struct RigProvider;

impl LlmProvider for RigProvider {
    fn complete(&self, _prompt: &str) -> Result<String, LlmError> {
        Err(LlmError::InvalidRequest(
            "use rig model(), not legacy complete()".to_owned(),
        ))
    }
}

/// Legacy factory.
#[must_use]
pub fn provider() -> RigProvider {
    RigProvider
}
