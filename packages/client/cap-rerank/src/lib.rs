//! Document-reranking capability (`media.rerank`).
//!
//! Backed by rig-core's rerank model. rig-core 0.45 has no OpenAI rerank
//! endpoint (`RerankQuirks::unsupported`), so `MediaKind::Rerank` defaults to
//! Voyage AI, the only provider it ships a rerank wire for; the OpenAI-compatible
//! path returns a typed configuration error instead of pretending to work.
//!
//! The capability is **dynamically available**: [`available`] is true only
//! after a rerank model is configured ([`configure`], [`configure_from_config`],
//! or [`configure_from_env`] with `ZEOKEEL_RERANK_*`). [`unavailable_reason`]
//! gives the honest reason otherwise, and [`UNAVAILABLE`] is the closed code
//! hosts surface instead of silently dropping the feature.
#![forbid(unsafe_code)]

use std::sync::{Mutex, OnceLock};

use rig_core::operation::{Rerank, RerankRequest};
use rig_core::DynModel;
use zk_llm_rig::{block_on, rerank_model, rerank_model_from_config, MediaConfig, MediaError};

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

static MODEL: OnceLock<Mutex<Option<DynModel<Rerank>>>> = OnceLock::new();

fn model_slot() -> &'static Mutex<Option<DynModel<Rerank>>> {
    MODEL.get_or_init(|| Mutex::new(None))
}

fn map_media_error(error: MediaError) -> RerankError {
    match error {
        MediaError::Unconfigured(message) => RerankError::Unavailable(message),
        MediaError::Config(message) => RerankError::Config(message),
        MediaError::Runtime(message) => RerankError::Runtime(message),
    }
}

/// Whether a rerank model is configured.
#[must_use]
pub fn available() -> bool {
    model_slot()
        .lock()
        .map(|guard| guard.is_some())
        .unwrap_or(false)
}

/// Why reranking is unavailable, or `None` when it is available.
#[must_use]
pub fn unavailable_reason() -> Option<String> {
    if available() {
        return None;
    }
    Some(
        "no rerank model configured: call zk_cap_rerank::configure(model), \
         configure_from_config(&zk_llm_rig::MediaConfig), or set \
         ZEOKEEL_RERANK_API_KEY/ZEOKEEL_RERANK_BASE_URL/ZEOKEEL_RERANK_MODEL \
         (provider voyageai) and call configure_from_env()"
            .to_owned(),
    )
}

/// Install the rerank model used by [`rerank`].
///
/// # Errors
///
/// [`RerankError::Config`] when the model lock is poisoned.
pub fn configure(model: DynModel<Rerank>) -> Result<(), RerankError> {
    let mut guard = model_slot()
        .lock()
        .map_err(|_| RerankError::Config("rerank model lock poisoned".to_owned()))?;
    *guard = Some(model);
    Ok(())
}

/// Build and install a rerank model from explicit [`MediaConfig`].
///
/// # Errors
///
/// As [`zk_llm_rig::rerank_model_from_config`], plus [`RerankError::Config`]
/// for a poisoned model lock.
pub fn configure_from_config(config: &MediaConfig) -> Result<(), RerankError> {
    configure(rerank_model_from_config(config).map_err(map_media_error)?)
}

/// Configure from `ZEOKEEL_RERANK_*`; `Ok(false)` when no API key is configured.
///
/// # Errors
///
/// [`RerankError::Config`] for an invalid base URL/provider, and as
/// [`configure`] for a poisoned model lock.
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
/// [`RerankError::Config`] when the model lock is poisoned.
pub fn unconfigure() -> Result<(), RerankError> {
    let mut guard = model_slot()
        .lock()
        .map_err(|_| RerankError::Config("rerank model lock poisoned".to_owned()))?;
    *guard = None;
    Ok(())
}

/// Rank `documents` by relevance to `query`, highest first.
///
/// An empty document list returns an empty result without calling the provider.
///
/// # Errors
///
/// [`RerankError::Unavailable`] when no model is configured,
/// [`RerankError::Config`] for an empty query or a poisoned model lock,
/// [`RerankError::Provider`] when the provider call fails, and
/// [`RerankError::Runtime`] when the shared runtime cannot start.
pub fn rerank(query: &str, documents: &[String]) -> Result<Vec<RerankHit>, RerankError> {
    let query = query.trim();
    if query.is_empty() {
        return Err(RerankError::Config("rerank query is empty".to_owned()));
    }
    if documents.is_empty() {
        return Ok(Vec::new());
    }
    let model = {
        let guard = model_slot()
            .lock()
            .map_err(|_| RerankError::Config("rerank model lock poisoned".to_owned()))?;
        guard.clone()
    };
    let Some(model) = model else {
        return Err(RerankError::Unavailable(
            unavailable_reason().unwrap_or_default(),
        ));
    };
    let request = RerankRequest {
        query: query.to_owned(),
        documents: documents.to_vec(),
    };
    let response = block_on(model.call(request))
        .map_err(map_media_error)?
        .map_err(|error| RerankError::Provider(error.to_string()))?;
    Ok(response
        .results
        .into_iter()
        .map(|result| RerankHit {
            index: result.index,
            score: result.relevance_score,
            document: result.document,
        })
        .collect())
}
