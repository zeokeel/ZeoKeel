//! JSON settings store (minimal M1 implementation).
//!
//! @capability store.settings.json
#![forbid(unsafe_code)]

use std::io::Write;
use std::path::PathBuf;

use zk_contract_config::{AppSettings, ConfigError, SettingsStore};

/// JSON-file backed settings store.
pub struct JsonSettingsStore {
    path: PathBuf,
}

/// Default store location: `$ZEOKEEL_HOME/settings.json` or the platform data dir.
#[must_use]
pub fn store() -> JsonSettingsStore {
    JsonSettingsStore {
        path: default_path(),
    }
}

/// Open a store at an explicit path.
#[must_use]
pub fn open(path: PathBuf) -> JsonSettingsStore {
    JsonSettingsStore { path }
}

fn default_path() -> PathBuf {
    if let Ok(home) = std::env::var("ZEOKEEL_HOME") {
        if !home.is_empty() {
            return PathBuf::from(home).join("settings.json");
        }
    }
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("zeokeel").join("settings.json")
}

fn storage(error: impl std::fmt::Display) -> ConfigError {
    ConfigError::Storage(error.to_string())
}

impl SettingsStore for JsonSettingsStore {
    fn load(&self) -> Result<AppSettings, ConfigError> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => {
                serde_json::from_str(&text).map_err(|error| ConfigError::Invalid(error.to_string()))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(AppSettings::default())
            }
            Err(error) => Err(storage(error)),
        }
    }

    fn save(&self, settings: &AppSettings) -> Result<(), ConfigError> {
        if settings.schema != 1 {
            return Err(ConfigError::Invalid(format!(
                "unsupported schema {}",
                settings.schema
            )));
        }
        if !matches!(settings.theme.as_str(), "auto" | "light" | "dark") {
            return Err(ConfigError::Invalid(format!(
                "invalid theme {}",
                settings.theme
            )));
        }
        if !matches!(settings.locale.as_str(), "zh-CN" | "en-US") {
            return Err(ConfigError::Invalid(format!(
                "invalid locale {}",
                settings.locale
            )));
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(storage)?;
        }
        let text = serde_json::to_string_pretty(settings)
            .map_err(|error| ConfigError::Invalid(error.to_string()))?;
        let tmp = self.path.with_extension("json.tmp");
        let mut file = std::fs::File::create(&tmp).map_err(storage)?;
        file.write_all(text.as_bytes()).map_err(storage)?;
        file.sync_all().map_err(storage)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))
                .map_err(storage)?;
        }
        std::fs::rename(&tmp, &self.path).map_err(storage)?;
        Ok(())
    }
}

/// Path accessor for tests/diagnostics.
#[must_use]
pub fn path_of(store: &JsonSettingsStore) -> &std::path::Path {
    &store.path
}
