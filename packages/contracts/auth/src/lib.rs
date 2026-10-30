//! `auth` domain contract marker (roles depend on this crate only).
#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};

/// Domain module descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainModule { pub id: String }
