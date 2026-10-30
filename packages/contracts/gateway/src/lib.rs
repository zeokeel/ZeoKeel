//! `gateway` domain contract marker (roles depend on this crate only).
//!
//! @capability contract.gateway
#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};

/// Domain module descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct DomainModule {
    pub id: String,
}

/// Per-model price row (admin pricing API and `/v1/models` display).
///
/// Amounts are integer micro-units; `*_multiplier_bps` are basis points
/// (10_000 = 1.0x) for peak/off-peak billing (`docs/22` §4). Mirrors the wire
/// shape currently emitted by `srv.pricing` (T22 contract-first follow-up).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ModelPrice {
    /// Model id (e.g. `deepseek-chat`).
    pub model: String,
    /// Input price per 1K tokens in micro-units.
    pub input_per_1k_micros: i64,
    /// Output price per 1K tokens in micro-units.
    pub output_per_1k_micros: i64,
    /// Cached-input price per 1K tokens in micro-units.
    #[serde(default)]
    pub cached_per_1k_micros: i64,
    /// Peak-hour multiplier in basis points.
    #[serde(default = "default_bps")]
    pub peak_multiplier_bps: i32,
    /// Off-peak multiplier in basis points.
    #[serde(default = "default_bps")]
    pub offpeak_multiplier_bps: i32,
    /// Whether the model is billable/available.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

/// Token usage reported by the upstream Responses API (`docs/22` §4).
///
/// Settlement reads the final `response.completed` usage block; `usage` rows and
/// holds are settled in one transaction. Zero values are valid (unknown counts).
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
pub struct UsageTotals {
    /// Fresh input tokens.
    #[serde(default)]
    pub input_tokens: i64,
    /// Cached input tokens.
    #[serde(default)]
    pub cached_tokens: i64,
    /// Output tokens.
    #[serde(default)]
    pub output_tokens: i64,
    /// Reasoning tokens (subset of output).
    #[serde(default)]
    pub reasoning_tokens: i64,
}

fn default_bps() -> i32 {
    10_000
}

fn default_enabled() -> bool {
    true
}
