//! Document-reranking capability (`media.rerank`).
//!
//! @capability media.rerank
//!
//! Backed by the shared [`zk_kit_media`] adapter. rig-core 0.45 has no OpenAI
//! rerank endpoint (`RerankQuirks::unsupported`), so `MediaKind::Rerank`
//! defaults to Voyage AI, the only provider it ships a rerank wire for; the
//! OpenAI-compatible path returns a typed configuration error instead of
//! pretending to work.
//!
//! The capability is **dynamically available**: [`available`] is true only
//! after a rerank model is configured ([`configure`], [`configure_from_config`],
//! or [`configure_from_env`] with `ZEOKEEL_RERANK_*`). [`unavailable_reason`]
//! gives the honest reason otherwise, and [`UNAVAILABLE`] is the closed code
//! hosts surface instead of silently dropping the feature.
//!
//! Host wiring: the personal-edition boot calls the generated
//! `zk_composed::configure_capabilities_from_env()` (which calls
//! [`configure_from_env`]); the enterprise client never configures media
//! providers (AI only through the gateway, C7/ADR-031).
#![forbid(unsafe_code)]

use zk_kit_media::{
    rerank_documents, rerank_model, rerank_model_from_config, MediaConfig, MediaError, MediaKind,
    ModelSlot, RerankModel,
};

/// Closed unavailability code hosts surface instead of silently dropping the feature.
pub const UNAVAILABLE: &str = "CAPABILITY_UNAVAILABLE";

/// Contract spelling of [`UNAVAILABLE`] (the `CAPABILITY_UNAVAILABLE` code).
pub const CAPABILITY_UNAVAILABLE: &str = "CAPABILITY_UNAVAILABLE";

/// Errors from the rerank capability.
#[derive(Debug, thiserror::Error)]
pub enum RerankError {
    /// No rerank model is configured.
    #[error("{UNAVAILABLE}: {0}")]
    Unavailable(String),
    /// Configuration/input is invalid (empty query, bad provider/env).
    #[error("rerank config: {0}")]
    Config(String),
    /// The rerank provider call failed.
    #[error("rerank provider: {0}")]
    Provider(String),
    /// The shared blocking runtime failed.
    #[error("rerank runtime: {0}")]
    Runtime(String),
}

/// One reranked document, in provider rank order (highest relevance first).
#[derive(Debug, Clone, PartialEq)]
pub struct RerankHit {
    /// Index of the document in the caller's input list.
    pub index: usize,
    /// Provider relevance score; higher is more relevant within this response.
    pub score: f64,
    /// The document text, when the provider echoed it back.
    pub document: Option<String>,
}

static MODEL: ModelSlot<RerankModel> = ModelSlot::new();

fn map_media_error(error: MediaError) -> RerankError {
    match error {
        MediaError::Unconfigured(message) => RerankError::Unavailable(message),
        MediaError::Config(message) => RerankError::Config(message),
        MediaError::Provider(message) => RerankError::Provider(message),
        MediaError::Runtime(message) => RerankError::Runtime(message),
    }
}

/// Whether a rerank model is configured.
#[must_use]
pub fn available() -> bool {
    MODEL.is_configured()
}

/// Why reranking is unavailable, or `None` when it is available.
#[must_use]
pub fn unavailable_reason() -> Option<String> {
    if available() {
        return None;
    }
    Some(MediaKind::Rerank.unavailable_reason())
}

/// Install the rerank model used by [`rerank`].
///
/// # Errors
///
/// [`RerankError::Config`] when the model slot lock is poisoned.
pub fn configure(model: RerankModel) -> Result<(), RerankError> {
    MODEL.set(model).map_err(map_media_error)
}

/// Build and install a rerank model from explicit [`MediaConfig`].
///
/// # Errors
///
/// As [`zk_kit_media::rerank_model_from_config`], plus [`RerankError::Config`]
/// for a poisoned model slot.
pub fn configure_from_config(config: &MediaConfig) -> Result<(), RerankError> {
    configure(rerank_model_from_config(config).map_err(map_media_error)?)
}

/// Configure from `ZEOKEEL_RERANK_*`; `Ok(false)` when no API key is configured.
///
/// This is the entry point the personal-edition boot uses through
/// `zk_composed::configure_capabilities_from_env()`.
///
/// # Errors
///
/// [`RerankError::Config`] for an invalid base URL/provider, and as
/// [`configure`] for a poisoned model slot.
pub fn configure_from_env() -> Result<bool, RerankError> {
    match rerank_model() {
        Ok(model) => {
            configure(model)?;
            Ok(true)
        }
        Err(MediaError::Unconfigured(_)) => Ok(false),
        Err(error) => Err(map_media_error(error)),
    }
}

/// Remove the rerank model, so [`available`] honestly returns false again.
///
/// # Errors
///
/// [`RerankError::Config`] when the model slot lock is poisoned.
pub fn unconfigure() -> Result<(), RerankError> {
    MODEL.clear().map_err(map_media_error)
}

/// Rank `documents` by relevance to `query`, highest first.
///
/// An empty document list returns an empty result without calling the provider.
///
/// # Errors
///
/// [`RerankError::Unavailable`] when no model is configured,
/// [`RerankError::Config`] for an empty query, [`RerankError::Provider`] when
/// the provider call fails, and [`RerankError::Runtime`] when the shared
/// runtime cannot start.
pub fn rerank(query: &str, documents: &[String]) -> Result<Vec<RerankHit>, RerankError> {
    let query = query.trim();
    if query.is_empty() {
        return Err(RerankError::Config("rerank query is empty".to_owned()));
    }
    if documents.is_empty() {
        return Ok(Vec::new());
    }
    let Some(model) = MODEL.get().map_err(map_media_error)? else {
        return Err(RerankError::Unavailable(
            unavailable_reason().unwrap_or_default(),
        ));
    };
    let outputs = rerank_documents(&model, query, documents).map_err(map_media_error)?;
    Ok(outputs
        .into_iter()
        .map(|output| RerankHit {
            index: output.index,
            score: output.score,
            document: output.document,
        })
        .collect())
}
