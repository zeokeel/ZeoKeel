//! Vector store / embeddings capability (`vector.store`).
//!
//! Backed by rig-core embedding models and an in-process cosine index. The
//! capability is **dynamically available**: [`available`] is true only after an
//! embedding model is configured (explicitly via [`configure`], or through
//! [`configure_from_env`] with an OpenAI-compatible embedding endpoint).
//! [`unavailable_reason`] gives the honest reason otherwise, and
//! [`UNAVAILABLE`] is the closed error code hosts surface.
//!
//! This package is an adapter: rig types stay out of the contracts and out of
//! the agent/tool packages (ADR-007), but this is a third rig adapter next to
//! `zk-llm-rig` / `zk-agent-rig`; the ADR's "only two adapters" wording needs an
//! amendment (reported to the Lead).
#![forbid(unsafe_code)]

use std::sync::{Mutex, OnceLock, RwLock};

use rig_core::embeddings::Embedding as Vector;
use rig_core::operation::Embedding;
use rig_core::providers::openai::OpenAIConfig;
use rig_core::DynModel;

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

static MODEL: OnceLock<Mutex<Option<DynModel<Embedding>>>> = OnceLock::new();
static DOCUMENTS: OnceLock<RwLock<Vec<StoredDocument>>> = OnceLock::new();
static RUNTIME: OnceLock<Result<tokio::runtime::Runtime, String>> = OnceLock::new();

fn model_slot() -> &'static Mutex<Option<DynModel<Embedding>>> {
    MODEL.get_or_init(|| Mutex::new(None))
}

fn documents() -> &'static RwLock<Vec<StoredDocument>> {
    DOCUMENTS.get_or_init(|| RwLock::new(Vec::new()))
}

fn runtime() -> Result<&'static tokio::runtime::Runtime, VectorError> {
    RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|error| format!("tokio runtime: {error}"))
        })
        .as_ref()
        .map_err(|error| VectorError::Embedding(error.clone()))
}

/// Whether an embedding model has been configured.
#[must_use]
pub fn available() -> bool {
    model_slot()
        .lock()
        .map(|guard| guard.is_some())
        .unwrap_or(false)
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
pub fn configure(model: DynModel<Embedding>) -> Result<(), VectorError> {
    let mut guard = model_slot()
        .lock()
        .map_err(|_| VectorError::Config("vector model lock poisoned".to_owned()))?;
    *guard = Some(model);
    Ok(())
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
    let mut model = model_slot()
        .lock()
        .map_err(|_| VectorError::Config("vector model lock poisoned".to_owned()))?;
    *model = None;
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
/// # Errors
///
/// [`VectorError::Config`] for an invalid base URL/dimensions or a poisoned
/// model slot.
pub fn configure_from_env() -> Result<bool, VectorError> {
    let base_url = env("ZEOKEEL_EMBEDDING_BASE_URL");
    let api_key = env("ZEOKEEL_EMBEDDING_API_KEY");
    let model = env("ZEOKEEL_EMBEDDING_MODEL");
    if base_url.is_empty() || api_key.is_empty() || model.is_empty() {
        return Ok(false);
    }
    if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
        return Err(VectorError::Config(format!(
            "ZEOKEEL_EMBEDDING_BASE_URL is not an http(s) URL: `{base_url}`"
        )));
    }
    let dims =
        match env("ZEOKEEL_EMBEDDING_DIMS") {
            value if value.is_empty() => None,
            value => Some(value.parse::<usize>().map_err(|error| {
                VectorError::Config(format!("ZEOKEEL_EMBEDDING_DIMS: {error}"))
            })?),
        };
    let client = OpenAIConfig::new(api_key)
        .with_base_url(base_url.trim_end_matches('/').to_owned())
        .client();
    configure(client.embedding(model, dims).into())?;
    Ok(true)
}

fn env(key: &str) -> String {
    std::env::var(key)
        .map(|value| value.trim().to_owned())
        .unwrap_or_default()
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
    let model = {
        let guard = model_slot()
            .lock()
            .map_err(|_| VectorError::Config("vector model lock poisoned".to_owned()))?;
        guard.clone()
    };
    let Some(model) = model else {
        return Err(VectorError::Unavailable(
            unavailable_reason().unwrap_or_default(),
        ));
    };
    let vector: Vector = runtime()?
        .block_on(async move { model.embed_text(text).await })
        .map_err(|error| VectorError::Embedding(error.to_string()))?;
    Ok(vector.vec)
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
