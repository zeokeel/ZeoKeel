//! WASM execution adapter (`srv.app.runtime`).
#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

pub use zk_kit_wasm::WasmError;
use zk_kit_wasm::{run, Limits, RunInput, RunOutput};

/// Headroom: one run.
pub fn execute(wasm: &[u8], db: &Path, request: serde_json::Value, fuel: u64, memory_bytes: usize, assets: &HashMap<String, Vec<u8>>, db_quota_bytes: u64) -> Result<RunOutput, WasmError> {
    run(RunInput {
        wasm,
        db_path: db,
        request,
        limits: Limits { fuel, timeout: Duration::from_secs(30), memory_bytes },
        assets,
        db_quota_bytes,
    })
}

/// Map runtime error to API code.
#[must_use]
pub fn error_code(error: &WasmError) -> &'static str {
    match error {
        WasmError::Fuel => "APP_LIMIT_FUEL",
        WasmError::Timeout => "APP_LIMIT_TIME",
        WasmError::Memory => "APP_LIMIT_MEMORY",
        WasmError::Db(_) | WasmError::Sql(_) => "APP_BAD_REQUEST",
        WasmError::BadRequest | WasmError::BadFrame => "APP_BAD_REQUEST",
        WasmError::BadWasm | WasmError::Internal(_) => "APP_BAD_WASM",
    }
}
