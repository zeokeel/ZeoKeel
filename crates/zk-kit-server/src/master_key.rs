//! Versioned `master.key` ring for encrypting secrets at rest (`docs/06` §5, `docs/21` §3).
//!
//! Stored format for an encrypted value: `v<version>:<base64(nonce || ciphertext+tag)>`
//! using AES-256-GCM (`ring`). The highest loaded version is used for new writes;
//! decryption selects the key by the version prefix (so old ciphertext stays readable
//! during a rotation). Values without a `v<version>:` prefix are legacy plaintext and are
//! returned with `legacy = true` so the caller can warn and re-encrypt on the next write.
//!
//! Key material sources (both may be combined):
//! - files `<ZEOKEEL_MASTER_KEY_DIR>/master.key.v<N>` (or `master.key` = version 1),
//!   must be mode 0600 on unix (a world/group-accessible file is refused);
//! - env `ZEOKEEL_MASTER_KEY` (32-byte key as hex or base64) with optional
//!   `ZEOKEEL_MASTER_KEY_VERSION` (default 1).
//!
//! `ZEOKEEL_MASTER_KEY_DIR` defaults to `/data` (deployment volume). No key loaded means
//! new writes fail closed (`KeyError::Missing`); legacy plaintext reads still succeed.
//!
//! This module is part of the existing `kit.server` shared infrastructure; the single
//! capability registration lives in `crates/zk-kit-server/src/lib.rs`.
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
use base64::Engine;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
use ring::rand::{SecureRandom, SystemRandom};

/// AES-256 key length in bytes.
pub const KEY_BYTES: usize = 32;

/// Errors from master-key loading and AEAD operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyError {
    /// No key material is configured; writes are refused (fail-closed).
    Missing,
    /// Key material/format is invalid (not 32 bytes, conflicting version, ...).
    InvalidConfig,
    /// Key/ciphertext file could not be read.
    Io,
    /// A key file is group/world-accessible (required: 0600).
    Permissions,
    /// Ciphertext references a key version that is not loaded.
    UnknownVersion(u32),
    /// Ciphertext is malformed or its AEAD tag did not verify (tampered).
    InvalidCiphertext,
}

impl fmt::Display for KeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => write!(f, "MASTER_KEY missing: new secret writes fail closed"),
            Self::InvalidConfig => write!(f, "MASTER_KEY invalid: expected 32-byte hex/base64"),
            Self::Io => write!(f, "master.key I/O error"),
            Self::Permissions => write!(f, "master.key permissions must be 0600"),
            Self::UnknownVersion(version) => write!(f, "master.key.v{version} not loaded"),
            Self::InvalidCiphertext => write!(f, "ciphertext tampered or malformed"),
        }
    }
}

impl std::error::Error for KeyError {}

/// Result of decrypting a stored secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecryptedSecret {
    /// Plaintext secret.
    pub value: String,
    /// `true` when the stored value was legacy plaintext (no `vN:` prefix).
    pub legacy: bool,
    /// Key version used, `None` for legacy plaintext.
    pub version: Option<u32>,
}

/// Loaded key ring; keys are ordered by ascending version.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeyRing {
    keys: Vec<(u32, [u8; KEY_BYTES])>,
}

impl KeyRing {
    /// Empty ring (no key configured); new writes fail with [`KeyError::Missing`].
    #[must_use]
    pub fn empty() -> Self {
        Self { keys: Vec::new() }
    }

    /// Build a ring from explicit `(version, key)` pairs.
    ///
    /// # Errors
    /// Returns [`KeyError::InvalidConfig`] for version 0, duplicate versions, or an
    /// all-zero key.
    pub fn from_pairs(pairs: Vec<(u32, [u8; KEY_BYTES])>) -> Result<Self, KeyError> {
        let mut map: BTreeMap<u32, [u8; KEY_BYTES]> = BTreeMap::new();
        for (version, key) in pairs {
            if version == 0 || key.iter().all(|byte| *byte == 0) {
                return Err(KeyError::InvalidConfig);
            }
            if map.insert(version, key).is_some() {
                return Err(KeyError::InvalidConfig);
            }
        }
        Ok(Self {
            keys: map.into_iter().collect(),
        })
    }

    /// `true` when no key is loaded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Loaded key versions, ascending.
    #[must_use]
    pub fn versions(&self) -> Vec<u32> {
        self.keys.iter().map(|(version, _)| *version).collect()
    }

    /// Highest loaded version (used for encryption), `None` when empty.
    #[must_use]
    pub fn highest_version(&self) -> Option<u32> {
        self.keys.iter().map(|(version, _)| *version).max()
    }

    /// Load the ring from `<dir>` (`master.key` = version 1, `master.key.vN` = version N).
    ///
    /// # Errors
    /// Returns [`KeyError`] for unreadable files, invalid key material, duplicate
    /// versions, or a key file that is not `0600` on unix. A missing directory is not an
    /// error (empty ring).
    pub fn load_from_dir(dir: &Path) -> Result<Self, KeyError> {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::empty());
            }
            Err(_) => return Err(KeyError::Io),
        };
        let mut pairs: Vec<(u32, [u8; KEY_BYTES])> = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|_| KeyError::Io)?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let Some(version) = version_from_file_name(name) else {
                continue;
            };
            let path = entry.path();
            check_owner_only_permissions(&entry)?;
            let text = std::fs::read_to_string(&path).map_err(|_| KeyError::Io)?;
            let key = parse_key(&text)?;
            if pairs.iter().any(|(existing, _)| *existing == version) {
                return Err(KeyError::InvalidConfig);
            }
            pairs.push((version, key));
        }
        Self::from_pairs(pairs)
    }

    /// Load the configured ring: key files from `ZEOKEEL_MASTER_KEY_DIR` (default `/data`)
    /// plus the `ZEOKEEL_MASTER_KEY` env key (version from `ZEOKEEL_MASTER_KEY_VERSION`).
    ///
    /// # Errors
    /// Returns [`KeyError`] for unreadable/invalid key files or conflicting versions.
    /// A deployment with no key configured returns an empty ring (writes fail closed).
    pub fn load_default() -> Result<Self, KeyError> {
        let dir = std::env::var("ZEOKEEL_MASTER_KEY_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/data"));
        let files = Self::load_from_dir(&dir)?;
        let mut pairs: Vec<(u32, [u8; KEY_BYTES])> = Vec::new();
        for version in files.versions() {
            if let Some((_, key)) = files.keys.iter().find(|(v, _)| *v == version) {
                pairs.push((version, *key));
            }
        }
        if let Ok(value) = std::env::var("ZEOKEEL_MASTER_KEY") {
            let version = match std::env::var("ZEOKEEL_MASTER_KEY_VERSION") {
                Ok(raw) => raw.parse::<u32>().map_err(|_| KeyError::InvalidConfig)?,
                Err(_) => 1,
            };
            let key = parse_key(&value)?;
            if pairs.iter().any(|(existing, _)| *existing == version) {
                return Err(KeyError::InvalidConfig);
            }
            pairs.push((version, key));
        }
        Self::from_pairs(pairs)
    }

    /// Encrypt `plaintext` with the highest loaded key version.
    ///
    /// # Errors
    /// Returns [`KeyError::Missing`] when no key is loaded (fail-closed for new writes)
    /// and [`KeyError::Io`] when the OS random source fails.
    pub fn encrypt(&self, plaintext: &str) -> Result<String, KeyError> {
        let (version, key) = self.keys.iter().next_back().ok_or(KeyError::Missing)?;
        let mut nonce_bytes = [0_u8; NONCE_LEN];
        SystemRandom::new()
            .fill(&mut nonce_bytes)
            .map_err(|_| KeyError::Io)?;
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);
        let key = LessSafeKey::new(
            UnboundKey::new(&AES_256_GCM, key).map_err(|_| KeyError::InvalidConfig)?,
        );
        let mut payload = plaintext.as_bytes().to_vec();
        key.seal_in_place_append_tag(nonce, Aad::empty(), &mut payload)
            .map_err(|_| KeyError::Io)?;
        let mut envelope = vec![0_u8; NONCE_LEN];
        envelope.copy_from_slice(&nonce_bytes);
        envelope.extend_from_slice(&payload);
        Ok(format!("v{version}:{}", STANDARD.encode(envelope)))
    }

    /// Decrypt a stored secret; values without a `vN:` prefix are legacy plaintext.
    ///
    /// # Errors
    /// Returns [`KeyError::UnknownVersion`] when the version prefix has no loaded key,
    /// [`KeyError::InvalidCiphertext`] on malformed/tampered data, and
    /// [`KeyError::Missing`] when encrypted data exists but no key is configured.
    pub fn decrypt(&self, stored: &str) -> Result<DecryptedSecret, KeyError> {
        let Some((version, encoded)) = split_version_prefix(stored) else {
            return Ok(DecryptedSecret {
                value: stored.to_owned(),
                legacy: true,
                version: None,
            });
        };
        if self.keys.is_empty() {
            return Err(KeyError::Missing);
        }
        let (_, key) = self
            .keys
            .iter()
            .find(|(candidate, _)| *candidate == version)
            .ok_or(KeyError::UnknownVersion(version))?;
        let decoded = STANDARD
            .decode(encoded)
            .or_else(|_| STANDARD_NO_PAD.decode(encoded))
            .map_err(|_| KeyError::InvalidCiphertext)?;
        if decoded.len() <= NONCE_LEN {
            return Err(KeyError::InvalidCiphertext);
        }
        let (nonce_bytes, ciphertext) = decoded.split_at(NONCE_LEN);
        let nonce_array: [u8; NONCE_LEN] = nonce_bytes
            .try_into()
            .map_err(|_| KeyError::InvalidCiphertext)?;
        let key = LessSafeKey::new(
            UnboundKey::new(&AES_256_GCM, key).map_err(|_| KeyError::InvalidConfig)?,
        );
        let mut in_out = ciphertext.to_vec();
        let plaintext = key
            .open_in_place(
                Nonce::assume_unique_for_key(nonce_array),
                Aad::empty(),
                &mut in_out,
            )
            .map_err(|_| KeyError::InvalidCiphertext)?;
        let value = std::str::from_utf8(plaintext)
            .map_err(|_| KeyError::InvalidCiphertext)?
            .to_owned();
        Ok(DecryptedSecret {
            value,
            legacy: false,
            version: Some(version),
        })
    }

    /// Re-encrypt `stored` with the highest key version when it is legacy plaintext or
    /// encrypted under an older version.
    ///
    /// # Errors
    /// Propagates [`KeyError`] from [`Self::decrypt`] and [`Self::encrypt`]; returns
    /// `Ok(None)` when the value is already encrypted with the highest version.
    pub fn reencrypt(&self, stored: &str) -> Result<Option<String>, KeyError> {
        let decrypted = self.decrypt(stored)?;
        let highest = self.highest_version();
        match decrypted.version {
            None => Ok(Some(self.encrypt(&decrypted.value)?)),
            Some(version) if highest.is_some_and(|top| version < top) => {
                Ok(Some(self.encrypt(&decrypted.value)?))
            }
            Some(_) => Ok(None),
        }
    }
}

/// Parse key material: 64 hex chars or standard base64 (padded or not) of 32 bytes.
///
/// # Errors
/// Returns [`KeyError::InvalidConfig`] when the value is not a 32-byte key.
pub fn parse_key(value: &str) -> Result<[u8; KEY_BYTES], KeyError> {
    let trimmed = value.trim();
    if trimmed.len() == KEY_BYTES.saturating_mul(2)
        && trimmed.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        let mut key = [0_u8; KEY_BYTES];
        for (index, chunk) in trimmed.as_bytes().as_chunks::<2>().0.iter().enumerate() {
            let pair =
                std::str::from_utf8(chunk.as_slice()).map_err(|_| KeyError::InvalidConfig)?;
            let byte = u8::from_str_radix(pair, 16).map_err(|_| KeyError::InvalidConfig)?;
            let slot = key.get_mut(index).ok_or(KeyError::InvalidConfig)?;
            *slot = byte;
        }
        return Ok(key);
    }
    let decoded = STANDARD
        .decode(trimmed)
        .or_else(|_| STANDARD_NO_PAD.decode(trimmed))
        .map_err(|_| KeyError::InvalidConfig)?;
    decoded
        .as_slice()
        .try_into()
        .map_err(|_| KeyError::InvalidConfig)
}

fn version_from_file_name(name: &str) -> Option<u32> {
    if name == "master.key" {
        return Some(1);
    }
    let rest = name.strip_prefix("master.key.v")?;
    let version = rest.parse::<u32>().ok()?;
    if version >= 1 {
        Some(version)
    } else {
        None
    }
}

fn split_version_prefix(stored: &str) -> Option<(u32, &str)> {
    let rest = stored.strip_prefix('v')?;
    let (digits, encoded) = rest.split_once(':')?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let version = digits.parse::<u32>().ok()?;
    if version >= 1 && !encoded.is_empty() {
        Some((version, encoded))
    } else {
        None
    }
}

fn check_owner_only_permissions(entry: &std::fs::DirEntry) -> Result<(), KeyError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = entry.metadata().map_err(|_| KeyError::Io)?;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(KeyError::Permissions);
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = entry;
        Ok(())
    }
}
