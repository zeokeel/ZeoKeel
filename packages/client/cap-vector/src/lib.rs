//! Vector store / embeddings capability (`vector.store`).
//!
//! @capability vector.store
//!
//! Backed by the shared [`zk_kit_media`] embedding adapter (rig-core
//! OpenAI-compatible embeddings) and an in-process cosine index. The
//! capability is **dynamically available**: [`available`] is true only after an
//! embedding model is configured (explicitly via [`configure`], or through
//! [`configure_from_env`] with an OpenAI-compatible embedding endpoint).
//! [`unavailable_reason`] gives the honest reason otherwise, and
//! [`UNAVAILABLE`] is the closed error code hosts surface.
//!
//! The rig adaptation lives in `crates/zk-kit-media` (`kit.media`), so this
//! package does not depend on `rig-core` or `zk-llm-rig` (C4/C17). Host wiring:
//! the personal-edition boot calls the generated
//! `zk_composed::configure_capabilities_from_env()` (which calls
//! [`configure_from_env`]); the enterprise client never configures providers
//! (C7/ADR-031; the gateway has no embeddings route), so it stays unavailable.
#![forbid(unsafe_code)]

use std::sync::{OnceLock, RwLock};

use zk_kit_media::{
    embed_text, embedding_model_from_config, EmbeddingConfig, EmbeddingError, EmbeddingModel,
    MediaError, ModelSlot,
};

/// Closed unavailability code.
pub const UNAVAILABLE: &str = "CAPABILITY_UNAVAILABLE";

/// Errors from the vector capability.
#[derive(Debug, thiserror::Error)]
pub enum VectorError {
    /// No embedding model is configured.
    #[error("{UNAVAILABLE}: {0}")]
    Unavailable(String),
    /// Provider configuration is invalid.
    #[error("vector config: {0}")]
    Config(String),
    /// The embedding provider failed.
    #[error("vector embedding: {0}")]
    Embedding(String),
    /// The in-process store is unavailable (poisoned lock).
    #[error("vector store: {0}")]
    Store(String),
}

/// One search hit.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    /// Caller-provided document id.
    pub id: String,
    /// The stored document text.
    pub text: String,
    /// Cosine similarity in `-1.0..=1.0` (higher is more similar).
    pub score: f64,
}

#[derive(Debug, Clone)]
struct StoredDocument {
    id: String,
    text: String,
    vector: Vec<f64>,
}

static MODEL: ModelSlot<EmbeddingModel> = ModelSlot::new();
static DOCUMENTS: OnceLock<RwLock<Vec<StoredDocument>>> = OnceLock::new();

fn documents() -> &'static RwLock<Vec<StoredDocument>> {
    DOCUMENTS.get_or_init(|| RwLock::new(Vec::new()))
}

fn map_media_error(error: MediaError) -> VectorError {
    match error {
        MediaError::Unconfigured(message) => VectorError::Unavailable(message),
        MediaError::Config(message) => VectorError::Config(message),
        MediaError::Provider(message) => VectorError::Embedding(message),
        MediaError::Runtime(message) => VectorError::Embedding(message),
    }
}

fn map_embedding_error(error: EmbeddingError) -> VectorError {
    match error {
        EmbeddingError::Unconfigured(message) => VectorError::Unavailable(message),
        EmbeddingError::Config(message) => VectorError::Config(message),
        EmbeddingError::Provider(message) => VectorError::Embedding(message),
        EmbeddingError::Runtime(message) => VectorError::Embedding(message),
    }
}

/// Whether an embedding model has been configured.
#[must_use]
pub fn available() -> bool {
    MODEL.is_configured()
}

/// Why the capability is unavailable, or `None` when it is available.
#[must_use]
pub fn unavailable_reason() -> Option<String> {
    if available() {
        return None;
    }
    Some(
        "no embedding model configured: call zk_cap_vector::configure(model) or set \
         ZEOKEEL_EMBEDDING_BASE_URL/ZEOKEEL_EMBEDDING_API_KEY/ZEOKEEL_EMBEDDING_MODEL and call \
         configure_from_env()"
            .to_owned(),
    )
}

/// Install the embedding model used for inserts and searches.
///
/// # Errors
///
/// [`VectorError::Config`] when the model cannot be stored.
pub fn configure(model: EmbeddingModel) -> Result<(), VectorError> {
    MODEL.set(model).map_err(map_media_error)
}

/// Remove the embedding model and drop stored documents.
///
/// Used when the provider changes or the user logs out, so [`available`]
/// honestly returns false again.
///
/// # Errors
///
/// [`VectorError::Config`] when the model lock is poisoned;
/// [`VectorError::Store`] when the store lock is poisoned.
pub fn unconfigure() -> Result<(), VectorError> {
    MODEL.clear().map_err(map_media_error)?;
    documents()
        .write()
        .map_err(|_| VectorError::Store("vector index lock poisoned".to_owned()))?
        .clear();
    Ok(())
}

/// Configure from an OpenAI-compatible embedding endpoint.
///
/// Reads `ZEOKEEL_EMBEDDING_BASE_URL`, `ZEOKEEL_EMBEDDING_API_KEY`,
/// `ZEOKEEL_EMBEDDING_MODEL` (default `text-embedding-3-small`) and the
/// optional `ZEOKEEL_EMBEDDING_DIMS`. Returns `Ok(false)` when the endpoint is
/// not configured (the capability stays unavailable with
/// [`unavailable_reason`]).
///
/// This is the entry point the personal-edition boot uses through
/// `zk_composed::configure_capabilities_from_env()`.
///
/// # Errors
///
/// [`VectorError::Config`] for an invalid base URL/dimensions, and
/// [`VectorError::Config`] for a poisoned model slot.
pub fn configure_from_env() -> Result<bool, VectorError> {
    let config = EmbeddingConfig::from_env().map_err(map_embedding_error)?;
    if !config.is_configured() {
        return Ok(false);
    }
    configure(embedding_model_from_config(&config).map_err(map_embedding_error)?)?;
    Ok(true)
}

/// Insert (or replace) one document, embedding it with the configured model.
///
/// # Errors
///
/// [`VectorError::Unavailable`] when no embedding model is configured;
/// [`VectorError::Embedding`] when the provider call fails;
/// [`VectorError::Store`] when the store lock is poisoned.
pub fn insert(id: &str, text: &str) -> Result<(), VectorError> {
    if id.trim().is_empty() {
        return Err(VectorError::Config("document id is empty".to_owned()));
    }
    let vector = embed(text)?;
    let mut guard = documents()
        .write()
        .map_err(|_| VectorError::Store("vector index lock poisoned".to_owned()))?;
    guard.retain(|document| document.id != id);
    guard.push(StoredDocument {
        id: id.to_owned(),
        text: text.to_owned(),
        vector,
    });
    Ok(())
}

/// Insert several documents in one call.
///
/// # Errors
///
/// As [`insert`]; the first failing document stops the batch (already inserted
/// documents stay).
pub fn insert_many(
    documents: impl IntoIterator<Item = (String, String)>,
) -> Result<(), VectorError> {
    for (id, text) in documents {
        insert(&id, &text)?;
    }
    Ok(())
}

/// Search the in-process index for the `limit` most similar documents.
///
/// # Errors
///
/// As [`insert`]; an empty index returns an empty result.
pub fn search(query: &str, limit: usize) -> Result<Vec<SearchHit>, VectorError> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let query_vector = embed(query)?;
    let guard = documents()
        .read()
        .map_err(|_| VectorError::Store("vector index lock poisoned".to_owned()))?;
    let mut hits: Vec<SearchHit> = guard
        .iter()
        .map(|document| SearchHit {
            id: document.id.clone(),
            text: document.text.clone(),
            score: cosine(&query_vector, &document.vector),
        })
        .collect();
    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits.truncate(limit);
    Ok(hits)
}

/// Drop every stored document (the embedding model stays configured).
///
/// # Errors
///
/// [`VectorError::Store`] when the store lock is poisoned.
pub fn clear() -> Result<(), VectorError> {
    documents()
        .write()
        .map_err(|_| VectorError::Store("vector index lock poisoned".to_owned()))?
        .clear();
    Ok(())
}

/// Number of stored documents.
#[must_use]
pub fn len() -> usize {
    documents().read().map(|guard| guard.len()).unwrap_or(0)
}

fn embed(text: &str) -> Result<Vec<f64>, VectorError> {
    let Some(model) = MODEL.get().map_err(map_media_error)? else {
        return Err(VectorError::Unavailable(
            unavailable_reason().unwrap_or_default(),
        ));
    };
    embed_text(&model, text).map_err(map_embedding_error)
}

/// Cosine similarity; `0.0` when either vector is empty or not the same width.
// Float similarity math is bounded to [-1, 1]; there is no integer overflow
// surface here, so the workspace arithmetic lint is relaxed locally only.
#[allow(clippy::arithmetic_side_effects)]
fn cosine(left: &[f64], right: &[f64]) -> f64 {
    if left.is_empty() || left.len() != right.len() {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut left_norm = 0.0;
    let mut right_norm = 0.0;
    for (left_value, right_value) in left.iter().zip(right.iter()) {
        dot += left_value * right_value;
        left_norm += left_value * left_value;
        right_norm += right_value * right_value;
    }
    if left_norm <= 0.0 || right_norm <= 0.0 {
        return 0.0;
    }
    dot / (left_norm.sqrt() * right_norm.sqrt())
}
