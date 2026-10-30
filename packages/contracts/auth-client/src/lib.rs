//! Client auth session contract marker (`auth.client`).
//!
//! @capability contract.auth-client
#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};

/// Authenticated client session (tokens kept in memory; persistence is a client-store concern).
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AuthSession {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub user_id: String,
    pub role: String,
    pub username: String,
}
