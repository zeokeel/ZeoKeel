//! `auth` domain contract marker (roles depend on this crate only).
//!
//! @capability contract.auth
#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};

/// Domain module descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainModule {
    pub id: String,
}

/// Client login payload (public endpoints).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub device_id: String,
}

/// Refresh payload.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct RefreshRequest {
    pub refresh_token: String,
    pub device_id: String,
}

/// Admin cookie login payload.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AdminLoginRequest {
    pub username: String,
    pub password: String,
}

/// Auth methods response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AuthMethods {
    pub mode: String,
    pub providers: Vec<String>,
    pub min_client_version: String,
}
