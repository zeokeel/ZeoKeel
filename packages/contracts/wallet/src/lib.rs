//! `wallet` domain contract marker (roles depend on this crate only).
#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};

/// Domain module descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainModule {
    pub id: String,
}

/// Admin adjustment payload.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AdjustRequest {
    pub delta_micros: i64,
    pub reason: String,
    pub idempotency_key: String,
}
