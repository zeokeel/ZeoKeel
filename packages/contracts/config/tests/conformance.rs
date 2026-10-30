//! Contract conformance: settings round-trip and defaults.
#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use zk_contract_config::{AppSettings, ProviderKind};

#[test]
fn settings_round_trip_and_defaults() {
    let s = AppSettings::default();
    assert_eq!(s.provider.kind, ProviderKind::Mock);
    let json = serde_json::to_string(&s).unwrap();
    assert_eq!(s, serde_json::from_str::<AppSettings>(&json).unwrap());
}
