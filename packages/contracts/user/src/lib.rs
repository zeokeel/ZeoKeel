//! `user` domain contract marker (roles depend on this crate only).
#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};

/// Domain module descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainModule { pub id: String }

/// Create-user payload.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateUser {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub display_name: String,
    pub role: String,
}

/// Patch-user payload.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateUser {
    pub role: Option<String>,
    pub status: Option<String>,
    pub password: Option<String>,
}
