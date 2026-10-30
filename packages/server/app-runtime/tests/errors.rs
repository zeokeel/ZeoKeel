#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_kit_wasm::WasmError;
use zk_srv_app_runtime::error_code;

#[test]
fn maps_limit_errors_to_app_codes() {
    assert_eq!(error_code(&WasmError::Fuel), "APP_LIMIT_FUEL");
    assert_eq!(error_code(&WasmError::Timeout), "APP_LIMIT_TIME");
    assert_eq!(error_code(&WasmError::Memory), "APP_LIMIT_MEMORY");
    assert_eq!(error_code(&WasmError::BadWasm), "APP_BAD_WASM");
}
