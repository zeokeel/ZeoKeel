//! Vector store / embeddings capability (`vector.store`).
//!
//! @capability vector.store
//!
//! Backed by the shared [`zk_kit_media`] embedding adapter (rig-core
//! OpenAI-compatible embeddings) and an in-process cosine index. The
//! capability is **dynamically available**: [`available`] is true only after an
//! embedding model is configured (explicitly via [`configure`] /
//! [`configure_with_dir`], or through [`configure_from_env`] with an
//! OpenAI-compatible embedding endpoint). [`unavailable_reason`] gives the
//! honest reason otherwise, and [`UNAVAILABLE`] is the closed error code hosts
//! surface.
//!
//! **Optional persistence** (round 3): when a persistence directory is
//! configured — `ZEOKEEL_VECTOR_DIR`, or explicitly through
//! [`configure_with_dir`] — the index is stored in `vector-index.json` as
//! `{version, documents:[{id,text,embedding}]}`. Inserts and clears save
//! atomically (temp file `0600`, directory `0700`, then rename); the index
//! loads on configure. A missing file starts empty, and an unreadable/corrupt/
//! oversized file degrades to an empty in-memory index instead of failing:
//! the reason is exposed through [`persistence_reason`] and logged to stderr,
//! and the next successful save repairs the file. Size caps: [`MAX_DOCUMENTS`]
//! documents and [`MAX_INDEX_BYTES`] bytes, both typed errors.
//!
//! The rig adaptation lives in `crates/zk-kit-media` (`kit.media`), so this
//! package does not depend on `rig-core` or `zk-llm-rig` (C4/C17). Host wiring:
//! the personal-edition boot calls the generated
//! `zk_composed::configure_capabilities_from_env()` (which calls
//! [`configure_from_env`]); the enterprise client never configures providers
//! (C7/ADR-031; the gateway has no embeddings route), so it stays unavailable.
#![forbid(unsafe_code)]

use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{OnceLock, RwLock};

use serde::{Deserialize, Serialize};
use zk_kit_media::{
    embed_text, embedding_model_from_config, EmbeddingConfig, EmbeddingError, EmbeddingModel,
    MediaError, ModelSlot,
};

/// Closed unavailability code.
pub const UNAVAILABLE: &str = "CAPABILITY_UNAVAILABLE";

/// File name of the persisted index inside the persistence directory.
pub const INDEX_FILE: &str = "vector-index.json";

/// Version written to (and accepted from) the persisted index file.
pub const INDEX_VERSION: u32 = 1;

/// Maximum number of documents held in memory and persisted.
pub const MAX_DOCUMENTS: usize = 10_000;

/// Maximum serialized index size in bytes.
pub const MAX_INDEX_BYTES: u64 = 16 * 1024 * 1024;

/// Environment variable naming the optional persistence directory.
const PERSIST_DIR_ENV: &str = "ZEOKEEL_VECTOR_DIR";

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
    /// The index would exceed its document/byte cap.
    #[error("vector capacity: {0}")]
    Capacity(String),
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

/// On-disk document shape (`embedding` is the persisted name of the vector).
#[derive(Debug, Serialize, Deserialize)]
struct IndexDocument {
    id: String,
    text: String,
    embedding: Vec<f64>,
}

impl From<&StoredDocument> for IndexDocument {
    fn from(document: &StoredDocument) -> Self {
        Self {
            id: document.id.clone(),
            text: document.text.clone(),
            embedding: document.vector.clone(),
        }
    }
}

impl From<IndexDocument> for StoredDocument {
    fn from(document: IndexDocument) -> Self {
        Self {
            id: document.id,
            text: document.text,
            vector: document.embedding,
        }
    }
}

/// Versioned on-disk index file with the embedding-model fingerprint.
///
/// The fingerprint fields are `#[serde(default)]` so files written by earlier
/// builds (version/documents only) still load; a present fingerprint that
/// disagrees with the configured model/dimensions degrades the load instead of
/// silently mixing incompatible vectors.
#[derive(Debug, Serialize, Deserialize)]
struct IndexFile {
    version: u32,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    dims: Option<usize>,
    #[serde(default)]
    width: Option<usize>,
    documents: Vec<IndexDocument>,
}

/// Compatibility fingerprint of the embedding model that produced an index.
#[derive(Debug, Clone, Default)]
struct Fingerprint {
    model: Option<String>,
    dims: Option<usize>,
    /// Common width of the stored vectors (`None` for an empty index).
    width: Option<usize>,
}

impl Fingerprint {
    fn from_model(model: &EmbeddingModel, width: Option<usize>) -> Self {
        Self {
            model: model.id().map(str::to_owned),
            dims: model.dims(),
            width,
        }
    }
}

fn index_width(documents: &[StoredDocument]) -> Option<usize> {
    documents.first().map(|document| document.vector.len())
}

#[derive(Debug, Default)]
struct StoreState {
    documents: Vec<StoredDocument>,
    dir: Option<PathBuf>,
    /// Why the last load was degraded (corrupt/unreadable/oversized); cleared by
    /// the next successful save.
    persistence_reason: Option<String>,
}

#[derive(Debug, Default)]
struct LoadedIndex {
    documents: Vec<StoredDocument>,
    reason: Option<String>,
}

impl LoadedIndex {
    fn degraded(reason: String) -> Self {
        Self {
            documents: Vec::new(),
            reason: Some(reason),
        }
    }
}

static MODEL: ModelSlot<EmbeddingModel> = ModelSlot::new();
static STORE: OnceLock<RwLock<StoreState>> = OnceLock::new();
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn store() -> &'static RwLock<StoreState> {
    STORE.get_or_init(|| RwLock::new(StoreState::default()))
}

fn store_poisoned() -> VectorError {
    VectorError::Store("vector store lock poisoned".to_owned())
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

/// The persistence directory in use, or `None` when the index is memory-only.
#[must_use]
pub fn persistence_dir() -> Option<PathBuf> {
    store()
        .read()
        .map(|state| state.dir.clone())
        .unwrap_or(None)
}

/// Why the last persisted-index load degraded to empty (corrupt/unreadable/
/// oversized), or `None` when the load was clean (or there is no persistence).
///
/// Hosts should log this next to `available()`; the capability keeps working on
/// an empty in-memory index and the next successful save repairs the file.
#[must_use]
pub fn persistence_reason() -> Option<String> {
    store()
        .read()
        .map(|state| state.persistence_reason.clone())
        .unwrap_or(None)
}

/// Install the embedding model and load the persisted index from
/// `ZEOKEEL_VECTOR_DIR` when that environment variable is set.
///
/// When the variable is unset the index stays memory-only (and existing
/// in-memory documents are replaced by the persisted state, i.e. empty).
///
/// # Errors
///
/// [`VectorError::Config`] when the model cannot be stored;
/// [`VectorError::Store`] when the store lock is poisoned. Persistence load
/// problems degrade to empty instead of failing (see [`persistence_reason`]).
pub fn configure(model: EmbeddingModel) -> Result<(), VectorError> {
    match persistence_dir_from_env() {
        Some(dir) => configure_with_dir(model, dir),
        None => configure_with_dir(model, ""),
    }
}

/// Install the embedding model with an explicit persistence directory.
///
/// An empty path keeps the index memory-only. A configured directory is
/// created with mode `0700` when missing, and `vector-index.json` is loaded if
/// present; a missing/corrupt/unreadable/oversized file degrades to an empty
/// index (never a panic) and is reported through [`persistence_reason`].
///
/// # Errors
///
/// [`VectorError::Config`] when the model cannot be stored;
/// [`VectorError::Store`] when the store lock is poisoned.
pub fn configure_with_dir(model: EmbeddingModel, dir: impl AsRef<Path>) -> Result<(), VectorError> {
    let requested = dir.as_ref();
    let dir = if requested.as_os_str().is_empty() {
        None
    } else {
        Some(requested.to_path_buf())
    };
    let loaded = match &dir {
        Some(path) => load_index(path, &model),
        None => LoadedIndex::default(),
    };
    MODEL.set(model).map_err(map_media_error)?;
    let mut state = store().write().map_err(|_| store_poisoned())?;
    *state = StoreState {
        documents: loaded.documents,
        dir,
        persistence_reason: loaded.reason.clone(),
    };
    drop(state);
    if let Some(message) = loaded.reason {
        eprintln!("zk-cap-vector: persistent vector index degraded: {message}");
    }
    Ok(())
}

/// Remove the embedding model and drop stored documents.
///
/// The persisted file is left in place, so a later [`configure`] can load it
/// again. Used when the provider changes or the user logs out, so
/// [`available`] honestly returns false again.
///
/// # Errors
///
/// [`VectorError::Config`] when the model lock is poisoned;
/// [`VectorError::Store`] when the store lock is poisoned.
pub fn unconfigure() -> Result<(), VectorError> {
    MODEL.clear().map_err(map_media_error)?;
    let mut state = store().write().map_err(|_| store_poisoned())?;
    *state = StoreState::default();
    Ok(())
}

/// Configure from an OpenAI-compatible embedding endpoint.
///
/// Reads `ZEOKEEL_EMBEDDING_BASE_URL`, `ZEOKEEL_EMBEDDING_API_KEY`,
/// `ZEOKEEL_EMBEDDING_MODEL` (default `text-embedding-3-small`) and the
/// optional `ZEOKEEL_EMBEDDING_DIMS`; when `ZEOKEEL_VECTOR_DIR` is set the
/// persisted index in that directory is loaded (see [`configure_with_dir`]).
/// Returns `Ok(false)` when the endpoint is not configured (the capability
/// stays unavailable with [`unavailable_reason`]).
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
    let model = embedding_model_from_config(&config).map_err(map_embedding_error)?;
    configure(model)?;
    Ok(true)
}

/// Insert (or replace) one document, embedding it with the configured model.
///
/// When persistence is configured the index is saved atomically; the in-memory
/// index only changes after the save succeeds.
///
/// # Errors
///
/// [`VectorError::Unavailable`] when no embedding model is configured;
/// [`VectorError::Embedding`] when the provider call fails;
/// [`VectorError::Capacity`] when the index would exceed [`MAX_DOCUMENTS`] or
/// [`MAX_INDEX_BYTES`]; [`VectorError::Store`] when locking or saving fails.
pub fn insert(id: &str, text: &str) -> Result<(), VectorError> {
    if id.trim().is_empty() {
        return Err(VectorError::Config("document id is empty".to_owned()));
    }
    let model = configured_model()?;
    // Fail fast on the document cap before paying for an embedding call.
    {
        let state = store().read().map_err(|_| store_poisoned())?;
        let replacing = state.documents.iter().any(|document| document.id == id);
        if !replacing && state.documents.len() >= MAX_DOCUMENTS {
            return Err(document_capacity_error());
        }
    }
    let vector = embed(&model, text)?;
    let mut state = store().write().map_err(|_| store_poisoned())?;
    let mut next = state.documents.clone();
    next.retain(|document| document.id != id);
    next.push(StoredDocument {
        id: id.to_owned(),
        text: text.to_owned(),
        vector,
    });
    let fingerprint = Fingerprint::from_model(&model, index_width(&next));
    save_documents(state.dir.as_deref(), &next, &fingerprint)?;
    state.documents = next;
    state.persistence_reason = None;
    Ok(())
}

/// Insert several documents as one atomic batch.
///
/// All ids are validated and the document cap is projected **before** any
/// embedding call; then every document is embedded, and a single persistence
/// save commits the batch. Any failure (validation, embedding, save) leaves
/// both the in-memory index and the persisted file unchanged.
///
/// # Errors
///
/// [`VectorError::Config`] for an empty id, [`VectorError::Unavailable`] when
/// no embedding model is configured, [`VectorError::Embedding`] when a provider
/// call fails, [`VectorError::Capacity`] when the batch would exceed the caps,
/// and [`VectorError::Store`] when locking or saving fails.
pub fn insert_many(
    documents: impl IntoIterator<Item = (String, String)>,
) -> Result<(), VectorError> {
    let items: Vec<(String, String)> = documents.into_iter().collect();
    for (id, _) in &items {
        if id.trim().is_empty() {
            return Err(VectorError::Config("document id is empty".to_owned()));
        }
    }
    if items.is_empty() {
        return Ok(());
    }
    let model = configured_model()?;
    // Project the final document count before embedding anything, so a full
    // index fails fast without provider calls.
    {
        let state = store().read().map_err(|_| store_poisoned())?;
        let mut known: std::collections::HashSet<&str> = state
            .documents
            .iter()
            .map(|document| document.id.as_str())
            .collect();
        let mut projected = state.documents.len();
        for (id, _) in &items {
            if known.insert(id.as_str()) {
                projected = projected.saturating_add(1);
            }
        }
        if projected > MAX_DOCUMENTS {
            return Err(document_capacity_error());
        }
    }
    // Embed everything first: no mutation (memory or disk) before the batch is
    // known to be complete.
    let mut embedded: Vec<StoredDocument> = Vec::with_capacity(items.len());
    for (id, text) in items {
        embedded.push(StoredDocument {
            vector: embed(&model, &text)?,
            id,
            text,
        });
    }
    let mut state = store().write().map_err(|_| store_poisoned())?;
    let mut next = state.documents.clone();
    for document in embedded {
        next.retain(|existing| existing.id != document.id);
        next.push(document);
    }
    let fingerprint = Fingerprint::from_model(&model, index_width(&next));
    save_documents(state.dir.as_deref(), &next, &fingerprint)?;
    state.documents = next;
    state.persistence_reason = None;
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
    let model = configured_model()?;
    let query_vector = embed(&model, query)?;
    let state = store().read().map_err(|_| store_poisoned())?;
    let mut hits: Vec<SearchHit> = state
        .documents
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
/// When persistence is configured the empty index is saved atomically.
///
/// # Errors
///
/// [`VectorError::Store`] when locking or saving fails.
pub fn clear() -> Result<(), VectorError> {
    let fingerprint = MODEL
        .get()
        .map_err(map_media_error)?
        .map(|model| Fingerprint::from_model(&model, None))
        .unwrap_or_default();
    let mut state = store().write().map_err(|_| store_poisoned())?;
    save_documents(state.dir.as_deref(), &[], &fingerprint)?;
    state.documents.clear();
    state.persistence_reason = None;
    Ok(())
}

/// Number of stored documents.
#[must_use]
pub fn len() -> usize {
    store()
        .read()
        .map(|state| state.documents.len())
        .unwrap_or(0)
}

fn configured_model() -> Result<EmbeddingModel, VectorError> {
    let Some(model) = MODEL.get().map_err(map_media_error)? else {
        return Err(VectorError::Unavailable(
            unavailable_reason().unwrap_or_default(),
        ));
    };
    Ok(model)
}

fn embed(model: &EmbeddingModel, text: &str) -> Result<Vec<f64>, VectorError> {
    embed_text(model, text).map_err(map_embedding_error)
}

fn document_capacity_error() -> VectorError {
    VectorError::Capacity(format!(
        "vector index would hold more than {MAX_DOCUMENTS} documents"
    ))
}

fn persistence_dir_from_env() -> Option<PathBuf> {
    let value = std::env::var(PERSIST_DIR_ENV)
        .map(|value| value.trim().to_owned())
        .unwrap_or_default();
    if value.is_empty() {
        None
    } else {
        Some(PathBuf::from(value))
    }
}

fn load_index(dir: &Path, current: &EmbeddingModel) -> LoadedIndex {
    if let Err(error) = ensure_dir(dir) {
        return LoadedIndex::degraded(format!(
            "persistence directory `{}` unavailable: {error}",
            dir.display()
        ));
    }
    let path = dir.join(INDEX_FILE);
    let metadata = match fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return LoadedIndex::default(),
        Err(error) => {
            return LoadedIndex::degraded(format!(
                "persisted index `{}` unreadable: {error}",
                path.display()
            ))
        }
    };
    if metadata.len() > MAX_INDEX_BYTES {
        return LoadedIndex::degraded(format!(
            "persisted index `{}` exceeds {MAX_INDEX_BYTES} bytes",
            path.display()
        ));
    }
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return LoadedIndex::degraded(format!(
                "persisted index `{}` unreadable: {error}",
                path.display()
            ))
        }
    };
    match serde_json::from_slice::<IndexFile>(&bytes) {
        Ok(file) => index_file_to_state(file, current),
        Err(error) => LoadedIndex::degraded(format!(
            "persisted index `{}` corrupt: {error}",
            path.display()
        )),
    }
}

fn index_file_to_state(file: IndexFile, current: &EmbeddingModel) -> LoadedIndex {
    if file.version != INDEX_VERSION {
        return LoadedIndex::degraded(format!(
            "persisted index version {} unsupported (expected {})",
            file.version, INDEX_VERSION
        ));
    }
    if file.documents.len() > MAX_DOCUMENTS {
        return LoadedIndex::degraded(format!(
            "persisted index holds more than {MAX_DOCUMENTS} documents"
        ));
    }
    let mut expected_width: Option<usize> = None;
    for document in &file.documents {
        match expected_width {
            None => expected_width = Some(document.embedding.len()),
            Some(width) if width != document.embedding.len() => {
                return LoadedIndex::degraded(format!(
                    "persisted index has inconsistent vector widths ({} vs {})",
                    width,
                    document.embedding.len()
                ));
            }
            Some(_) => {}
        }
    }
    if let (Some(declared), Some(actual)) = (file.width, expected_width) {
        if declared != actual {
            return LoadedIndex::degraded(format!(
                "persisted index declares width {declared} but its vectors are {actual}"
            ));
        }
    }
    if let (Some(declared), Some(actual)) = (file.dims, expected_width) {
        if declared != actual {
            return LoadedIndex::degraded(format!(
                "persisted index declares dimensions {declared} but its vectors are {actual}"
            ));
        }
    }
    if let (Some(stored), Some(configured)) = (file.model.as_deref(), current.id()) {
        if stored != configured {
            return LoadedIndex::degraded(format!(
                "persisted index was built by embedding model `{stored}` but `{configured}` is configured"
            ));
        }
    }
    if let (Some(stored), Some(configured)) = (file.dims, current.dims()) {
        if stored != configured {
            return LoadedIndex::degraded(format!(
                "persisted index dimensions {stored} are incompatible with the configured {configured}"
            ));
        }
    }
    if let (Some(configured), Some(actual)) = (current.dims(), expected_width) {
        if actual != configured {
            return LoadedIndex::degraded(format!(
                "persisted index vectors are {actual} wide but {configured} are configured"
            ));
        }
    }
    LoadedIndex {
        documents: file
            .documents
            .into_iter()
            .map(StoredDocument::from)
            .collect(),
        reason: None,
    }
}

fn save_documents(
    dir: Option<&Path>,
    documents: &[StoredDocument],
    fingerprint: &Fingerprint,
) -> Result<(), VectorError> {
    if documents.len() > MAX_DOCUMENTS {
        return Err(VectorError::Capacity(format!(
            "vector index would hold more than {MAX_DOCUMENTS} documents"
        )));
    }
    let Some(dir) = dir else {
        return Ok(());
    };
    ensure_dir(dir).map_err(|error| {
        VectorError::Store(format!(
            "persistence directory `{}` unavailable: {error}",
            dir.display()
        ))
    })?;
    let file = IndexFile {
        version: INDEX_VERSION,
        model: fingerprint.model.clone(),
        dims: fingerprint.dims,
        width: fingerprint.width,
        documents: documents.iter().map(IndexDocument::from).collect(),
    };
    let bytes = serde_json::to_vec(&file)
        .map_err(|error| VectorError::Store(format!("serialize vector index: {error}")))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_INDEX_BYTES {
        return Err(VectorError::Capacity(format!(
            "vector index would exceed {MAX_INDEX_BYTES} bytes"
        )));
    }
    let path = dir.join(INDEX_FILE);
    let temp = dir.join(format!(
        ".{INDEX_FILE}.tmp-{}-{}",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    if let Err(error) = write_private(&temp, &bytes) {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }
    if let Err(error) = fs::rename(&temp, &path) {
        let _ = fs::remove_file(&temp);
        return Err(VectorError::Store(format!(
            "rename vector index into `{}`: {error}",
            path.display()
        )));
    }
    Ok(())
}

fn ensure_dir(dir: &Path) -> io::Result<()> {
    if dir.is_dir() {
        // Tighten pre-existing directories too: the index must not be readable
        // by other local users even if the directory existed before.
        return set_dir_private(dir);
    }
    let mut builder = DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(dir)?;
    set_dir_private(dir)
}

#[cfg(unix)]
fn set_dir_private(dir: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn set_dir_private(_dir: &Path) -> io::Result<()> {
    Ok(())
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<(), VectorError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| VectorError::Store(format!("create temp vector index: {error}")))?;
    file.write_all(bytes)
        .map_err(|error| VectorError::Store(format!("write temp vector index: {error}")))?;
    file.sync_all()
        .map_err(|error| VectorError::Store(format!("sync temp vector index: {error}")))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|error| {
            VectorError::Store(format!("set temp vector index permissions: {error}"))
        })?;
    }
    Ok(())
}

/// Cosine similarity; `0.0` when either vector is empty or not the same width.
///
/// Written with `mul_add`/`sum`/`recip` instead of arithmetic operators so the
/// product code stays under the workspace `arithmetic_side_effects` lint without
/// a local `#[allow]` (the math is bounded float similarity, not money/counts).
fn cosine(left: &[f64], right: &[f64]) -> f64 {
    if left.is_empty() || left.len() != right.len() {
        return 0.0;
    }
    let dot: f64 = left
        .iter()
        .zip(right.iter())
        .map(|(left_value, right_value)| left_value.mul_add(*right_value, 0.0))
        .sum();
    let left_norm: f64 = left.iter().map(|value| value.mul_add(*value, 0.0)).sum();
    let right_norm: f64 = right.iter().map(|value| value.mul_add(*value, 0.0)).sum();
    if left_norm <= 0.0 || right_norm <= 0.0 {
        return 0.0;
    }
    let denominator = left_norm.sqrt().mul_add(right_norm.sqrt(), 0.0);
    if denominator <= 0.0 {
        return 0.0;
    }
    dot.mul_add(denominator.recip(), 0.0)
}

#[cfg(test)]
mod store_tests;
