#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use std::path::PathBuf;
use zk_contract_config::{AppSettings, ProviderKind, SettingsStore};
use zk_store_settings_json::{open, path_of};

fn temp_path(name: &str) -> PathBuf {
    let unique = format!(
        "{}-{}-{}",
        std::process::id(),
        name,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    std::env::temp_dir().join(format!("zeokeel-settings-{unique}.json"))
}

#[test]
fn defaults_round_trip_and_atomic_write() {
    let path = temp_path("settings");
    let store = open(path.clone());
    assert_eq!(store.load().unwrap(), AppSettings::default());
    let mut settings = AppSettings {
        provider: zk_contract_config::ProviderConfig {
            kind: ProviderKind::OpenAiCompatible,
            ..AppSettings::default().provider
        },
        ..AppSettings::default()
    };
    settings.provider.base_url = "https://example.com/v1".to_owned();
    settings.provider.model = "m".to_owned();
    settings.theme = "dark".to_owned();
    store.save(&settings).unwrap();
    assert_eq!(store.load().unwrap(), settings);
    assert_eq!(path_of(&store), path.as_path());
}

#[test]
fn corrupt_file_is_preserved() {
    let path = temp_path("corrupt");
    std::fs::write(&path, b"{not json").unwrap();
    let store = open(path.clone());
    assert!(store.load().is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{not json");
}

#[test]
fn invalid_values_are_rejected() {
    let store = open(temp_path("invalid"));
    let settings = AppSettings {
        theme: "neon".to_owned(),
        ..AppSettings::default()
    };
    assert!(store.save(&settings).is_err());
}
