//! JSON settings store with a sidecar revision file (`settings.meta.json`).
//!
//! @capability store.settings.json
//!
//! `settings.json` keeps the frozen `AppSettings` schema; the optimistic
//! concurrency fence (`revision`) and a `settings_hash` live in a **separate**
//! sidecar file next to it so an old client that rewrites `settings.json` can
//! never reset the revision to zero (spec `2026-11-01-client-settings-ia` R4).
//!
//! Save transaction: back up the old `settings.json` → write the new settings
//! (tmp+fsync+rename) → write the meta file (tmp+fsync+rename). If the meta
//! write fails the old settings are restored (failure does not land on disk);
//! if the restore fails too the meta file is removed and the error returned, so
//! the next load rebuilds `revision = 0` instead of trusting an unknown pair.
#![forbid(unsafe_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zk_contract_config::{AppSettings, ConfigError, SettingsStore};

/// Sidecar meta schema version (bumped only for an incompatible meta layout).
const META_SCHEMA: u64 = 1;

/// Sidecar revision metadata (`settings.meta.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SettingsMeta {
    /// Meta schema version.
    schema: u64,
    /// Monotonic save counter.
    revision: u64,
    /// SHA-256 (lowercase hex) of the exact `settings.json` text it describes.
    settings_hash: String,
}

/// JSON-file backed settings store.
pub struct JsonSettingsStore {
    path: PathBuf,
    /// Serializes writers/readers so a reader never observes the gap between the
    /// settings rename and the meta rename (one writer per process).
    lock: Mutex<()>,
    #[cfg(test)]
    faults: Faults,
}

/// Test-only fault injection knobs (never compiled into the product build).
#[cfg(test)]
#[derive(Debug, Clone, Copy, Default)]
struct Faults {
    /// Make the `settings.meta.json` write fail after the settings write landed.
    fail_meta: bool,
    /// Make the rollback restore of `settings.json` fail too.
    fail_restore: bool,
}

/// Default store location: `$ZEOKEEL_HOME/settings.json` or the platform data dir.
#[must_use]
pub fn store() -> JsonSettingsStore {
    JsonSettingsStore {
        path: default_path(),
        lock: Mutex::new(()),
        #[cfg(test)]
        faults: Faults::default(),
    }
}

/// Open a store at an explicit path.
#[must_use]
pub fn open(path: PathBuf) -> JsonSettingsStore {
    JsonSettingsStore {
        path,
        lock: Mutex::new(()),
        #[cfg(test)]
        faults: Faults::default(),
    }
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

fn invalid(error: impl std::fmt::Display) -> ConfigError {
    ConfigError::Invalid(error.to_string())
}

/// SHA-256 of the exact persisted settings text (lowercase hex).
fn settings_hash(text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

impl JsonSettingsStore {
    fn meta_path(&self) -> PathBuf {
        self.path.with_extension("meta.json")
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, ()>, ConfigError> {
        self.lock
            .lock()
            .map_err(|_| storage("settings store lock poisoned"))
    }

    /// Write `bytes` to `path` atomically (tmp + fsync + rename, 0600 on unix).
    fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(storage)?;
        }
        let tmp = path.with_extension(format!(
            "{}.tmp",
            path.extension()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("json")
        ));
        let result = (|| -> Result<(), ConfigError> {
            let mut file = std::fs::File::create(&tmp).map_err(storage)?;
            file.write_all(bytes).map_err(storage)?;
            file.sync_all().map_err(storage)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))
                    .map_err(storage)?;
            }
            std::fs::rename(&tmp, path).map_err(storage)
        })();
        if result.is_err() {
            // Best-effort cleanup: never leave a half-written tmp file behind.
            let _ = std::fs::remove_file(&tmp);
        }
        result
    }

    /// Read + validate the sidecar for `text`; `None` when missing/invalid/mismatched.
    fn meta_for(&self, text: &str) -> Option<SettingsMeta> {
        let raw = std::fs::read_to_string(self.meta_path()).ok()?;
        let meta: SettingsMeta = serde_json::from_str(&raw).ok()?;
        if meta.schema != META_SCHEMA {
            return None;
        }
        if meta.settings_hash != settings_hash(text) {
            return None;
        }
        Some(meta)
    }

    /// Current revision of the settings text on disk (no locking; callers hold it).
    fn revision_of(&self, text: &str) -> u64 {
        self.meta_for(text).map_or(0, |meta| meta.revision)
    }

    /// Rewrite the sidecar for `text` with `revision` (best effort on load).
    fn rebuild_meta(&self, text: &str, revision: u64) {
        let meta = SettingsMeta {
            schema: META_SCHEMA,
            revision,
            settings_hash: settings_hash(text),
        };
        if let Ok(raw) = serde_json::to_string_pretty(&meta) {
            let _ = Self::write_atomic(&self.meta_path(), raw.as_bytes());
        }
    }

    #[cfg(not(test))]
    fn fail_meta(&self) -> bool {
        false
    }

    #[cfg(test)]
    fn fail_meta(&self) -> bool {
        self.faults.fail_meta
    }

    #[cfg(not(test))]
    fn fail_restore(&self) -> bool {
        false
    }

    #[cfg(test)]
    fn fail_restore(&self) -> bool {
        self.faults.fail_restore
    }

    fn validate(settings: &AppSettings) -> Result<(), ConfigError> {
        if settings.schema != 1 {
            return Err(invalid(format!("unsupported schema {}", settings.schema)));
        }
        if !matches!(settings.theme.as_str(), "auto" | "light" | "dark") {
            return Err(invalid(format!("invalid theme {}", settings.theme)));
        }
        if !matches!(settings.locale.as_str(), "zh-CN" | "en-US") {
            return Err(invalid(format!("invalid locale {}", settings.locale)));
        }
        Ok(())
    }

    /// Restore the previous settings after a failed meta write.
    ///
    /// Returns the error to surface: either "rolled back" or "rollback failed
    /// too" (meta removed so the next load rebuilds `revision = 0`).
    fn rollback(&self, backup: Option<&[u8]>, meta_error: &ConfigError) -> ConfigError {
        let restored = if self.fail_restore() {
            Err(storage("injected: settings rollback failed"))
        } else {
            match backup {
                Some(bytes) => Self::write_atomic(&self.path, bytes),
                None => std::fs::remove_file(&self.path).map_err(storage),
            }
        };
        match restored {
            Ok(()) => storage(format!(
                "settings.meta.json write failed ({meta_error}); settings rolled back"
            )),
            Err(restore_error) => {
                // The settings/meta pair can no longer be trusted: drop the meta
                // so the next load starts at revision 0 and rebuilds it.
                let _ = std::fs::remove_file(self.meta_path());
                storage(format!(
                    "settings.meta.json write failed ({meta_error}) and settings rollback failed: {restore_error}"
                ))
            }
        }
    }

    /// Back up → write settings → write meta, with rollback on meta failure.
    fn write_transaction(
        &self,
        settings: &AppSettings,
        expected: Option<u64>,
    ) -> Result<u64, ConfigError> {
        Self::validate(settings)?;
        let _guard = self.lock()?;
        let current = match std::fs::read_to_string(&self.path) {
            Ok(text) => self.revision_of(&text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
            Err(error) => return Err(storage(error)),
        };
        if let Some(expected) = expected {
            if expected != current {
                return Err(ConfigError::Conflict(format!(
                    "expected revision {expected}, current revision {current}"
                )));
            }
        }
        let next = current
            .checked_add(1)
            .ok_or_else(|| storage("settings revision overflow"))?;
        let backup = match std::fs::read(&self.path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(storage(error)),
        };
        let text = serde_json::to_string_pretty(settings).map_err(invalid)?;
        Self::write_atomic(&self.path, text.as_bytes())?;
        let meta = SettingsMeta {
            schema: META_SCHEMA,
            revision: next,
            settings_hash: settings_hash(&text),
        };
        let meta_text = serde_json::to_string_pretty(&meta).map_err(invalid)?;
        let meta_result = if self.fail_meta() {
            Err(storage("injected: settings.meta.json write failed"))
        } else {
            Self::write_atomic(&self.meta_path(), meta_text.as_bytes())
        };
        match meta_result {
            Ok(()) => Ok(next),
            Err(error) => Err(self.rollback(backup.as_deref(), &error)),
        }
    }
}

impl SettingsStore for JsonSettingsStore {
    fn load(&self) -> Result<AppSettings, ConfigError> {
        let _guard = self.lock()?;
        match std::fs::read_to_string(&self.path) {
            Ok(text) => {
                let settings: AppSettings = serde_json::from_str(&text)
                    .map_err(|error| ConfigError::Invalid(error.to_string()))?;
                // Missing/corrupt/mismatched meta ⇒ revision 0, rebuilt on load
                // (spec R4: hash mismatch resets the fence instead of trusting it).
                if self.meta_for(&text).is_none() {
                    self.rebuild_meta(&text, 0);
                }
                Ok(settings)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(AppSettings::default())
            }
            Err(error) => Err(storage(error)),
        }
    }

    fn save(&self, settings: &AppSettings) -> Result<(), ConfigError> {
        self.write_transaction(settings, None).map(|_revision| ())
    }

    fn revision(&self) -> Result<u64, ConfigError> {
        let _guard = self.lock()?;
        match std::fs::read_to_string(&self.path) {
            Ok(text) => Ok(self.revision_of(&text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
            Err(error) => Err(storage(error)),
        }
    }

    fn save_with_revision(
        &self,
        settings: &AppSettings,
        expected: Option<u64>,
    ) -> Result<u64, ConfigError> {
        self.write_transaction(settings, expected)
    }
}

/// Path accessor for tests/diagnostics.
#[must_use]
pub fn path_of(store: &JsonSettingsStore) -> &std::path::Path {
    &store.path
}

/// Sidecar path accessor for tests/diagnostics.
#[must_use]
pub fn meta_path_of(store: &JsonSettingsStore) -> std::path::PathBuf {
    store.meta_path()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)] // test-only
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let unique = format!(
            "{}-{}-{}",
            std::process::id(),
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let dir = std::env::temp_dir().join(format!("zeokeel-meta-{unique}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn open_in(dir: &Path) -> JsonSettingsStore {
        let mut store = open(dir.join("settings.json"));
        store.faults = Faults::default();
        store
    }

    fn themed(theme: &str) -> AppSettings {
        AppSettings {
            theme: theme.to_owned(),
            ..AppSettings::default()
        }
    }

    #[test]
    fn first_save_starts_at_revision_one_and_increments() {
        let dir = temp_dir("increment");
        let store = open_in(&dir);
        assert_eq!(store.revision().unwrap(), 0);
        assert_eq!(store.save_with_revision(&themed("dark"), None).unwrap(), 1);
        assert_eq!(store.revision().unwrap(), 1);
        // Legacy `save` also bumps the fence (old clients may rewrite settings.json).
        store.save(&themed("light")).unwrap();
        assert_eq!(store.revision().unwrap(), 2);
        assert_eq!(store.load().unwrap().theme, "light");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn stale_expected_revision_is_rejected_without_writing() {
        let dir = temp_dir("conflict");
        let store = open_in(&dir);
        assert_eq!(store.save_with_revision(&themed("dark"), None).unwrap(), 1);
        let before = std::fs::read_to_string(&store.path).unwrap();
        let error = store
            .save_with_revision(&themed("light"), Some(0))
            .expect_err("stale fence must be rejected");
        assert!(matches!(error, ConfigError::Conflict(_)), "{error:?}");
        assert_eq!(std::fs::read_to_string(&store.path).unwrap(), before);
        assert_eq!(store.revision().unwrap(), 1);
        // Retrying with the current revision succeeds (idempotent retry is safe).
        assert_eq!(
            store.save_with_revision(&themed("light"), Some(1)).unwrap(),
            2
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_or_corrupt_meta_degrades_to_revision_zero_and_rebuilds() {
        let dir = temp_dir("corrupt-meta");
        let store = open_in(&dir);
        store.save(&themed("dark")).unwrap();
        assert_eq!(store.revision().unwrap(), 1);
        std::fs::write(store.meta_path(), b"{not json").unwrap();
        assert_eq!(store.revision().unwrap(), 0);
        // Loading rebuilds a valid sidecar at revision 0 (does not fail the load).
        assert_eq!(store.load().unwrap().theme, "dark");
        assert_eq!(store.revision().unwrap(), 0);
        assert!(store
            .meta_for(&std::fs::read_to_string(&store.path).unwrap())
            .is_some());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn settings_hash_mismatch_resets_the_fence() {
        let dir = temp_dir("hash-mismatch");
        let store = open_in(&dir);
        store.save(&themed("dark")).unwrap();
        assert_eq!(store.revision().unwrap(), 1);
        // A legacy client rewrites settings.json without touching the sidecar.
        std::fs::write(
            &store.path,
            serde_json::to_string_pretty(&themed("light")).unwrap(),
        )
        .unwrap();
        assert_eq!(store.revision().unwrap(), 0);
        assert_eq!(store.load().unwrap().theme, "light");
        assert_eq!(store.revision().unwrap(), 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn meta_write_failure_rolls_the_settings_back() {
        let dir = temp_dir("meta-failure");
        let mut store = open_in(&dir);
        store.save(&themed("dark")).unwrap();
        let before = std::fs::read_to_string(&store.path).unwrap();
        store.faults.fail_meta = true;
        let error = store
            .save_with_revision(&themed("light"), Some(1))
            .expect_err("meta failure must fail the save");
        assert!(matches!(error, ConfigError::Storage(_)), "{error:?}");
        // Failure does not land on disk: old settings + old revision survive.
        assert_eq!(std::fs::read_to_string(&store.path).unwrap(), before);
        assert_eq!(store.revision().unwrap(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn meta_write_failure_with_failed_rollback_drops_meta() {
        let dir = temp_dir("rollback-failure");
        let mut store = open_in(&dir);
        store.save(&themed("dark")).unwrap();
        store.faults.fail_meta = true;
        store.faults.fail_restore = true;
        let error = store
            .save_with_revision(&themed("light"), Some(1))
            .expect_err("double failure must fail the save");
        assert!(error.to_string().contains("rollback failed"), "{error:?}");
        // Meta removed (untrusted pair) → next load degrades to revision 0.
        assert!(!store.meta_path().exists());
        assert_eq!(store.revision().unwrap(), 0);
        let reloaded = store.load().unwrap();
        assert_eq!(reloaded.theme, "light");
        assert_eq!(store.revision().unwrap(), 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn meta_write_failure_on_first_save_removes_the_new_settings() {
        let dir = temp_dir("meta-failure-first");
        let mut store = open_in(&dir);
        store.faults.fail_meta = true;
        let error = store
            .save_with_revision(&themed("dark"), None)
            .expect_err("meta failure must fail the save");
        assert!(matches!(error, ConfigError::Storage(_)), "{error:?}");
        // No previous settings existed → the new file is removed (no orphan pair).
        assert!(!store.path.exists());
        assert_eq!(store.load().unwrap(), AppSettings::default());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn concurrent_saves_serialize_and_each_bumps_the_revision() {
        let dir = temp_dir("concurrent");
        let store = std::sync::Arc::new(open_in(&dir));
        let mut handles = Vec::new();
        for index in 0..8_u32 {
            let store = std::sync::Arc::clone(&store);
            handles.push(std::thread::spawn(move || {
                let theme = if index % 2 == 0 { "dark" } else { "light" };
                store.save(&themed(theme)).unwrap();
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }
        assert_eq!(store.revision().unwrap(), 8);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
