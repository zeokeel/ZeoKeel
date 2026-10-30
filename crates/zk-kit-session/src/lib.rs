//! Process-wide current session store (for tools that read session history).
//!
//! @capability kit.session
#![forbid(unsafe_code)]
use std::sync::{Arc, Mutex, OnceLock};
use zk_contract_session::SessionStore;

static STORE: OnceLock<Mutex<Option<Arc<dyn SessionStore>>>> = OnceLock::new();
fn slot() -> &'static Mutex<Option<Arc<dyn SessionStore>>> { STORE.get_or_init(|| Mutex::new(None)) }

/// Install the desktop session store.
pub fn set_global(store: Arc<dyn SessionStore>) { if let Ok(mut guard) = slot().lock() { *guard = Some(store); } }

/// Resolve the current session store.
#[must_use] pub fn global() -> Option<Arc<dyn SessionStore>> { slot().lock().ok().and_then(|guard| guard.clone()) }
