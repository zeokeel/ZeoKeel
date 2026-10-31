#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)] // test-only
//! Executable checks for the versioned `master.key` ring (docs/06 §5, docs/21 §3).
//!
//! `env_ring_drill` only runs when `scripts/master-key-rotate.sh` sets the
//! `ZEOKEEL_MASTER_KEY_DRILL_*` env vars; otherwise it prints a skip marker and returns.
use std::fs;
use std::path::{Path, PathBuf};

use zk_kit_server::master_key::{parse_key, KeyError, KeyRing, KEY_BYTES};

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("zk-master-key-{label}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).expect("create temp dir");
        Self(path)
    }

    fn path(&self) -> &Path {
        self.0.as_path()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn key(seed: u8) -> [u8; KEY_BYTES] {
    [seed; KEY_BYTES]
}

fn ring(pairs: Vec<(u32, [u8; KEY_BYTES])>) -> KeyRing {
    KeyRing::from_pairs(pairs).expect("valid test ring")
}

#[test]
fn roundtrip_with_highest_version() {
    let ring = ring(vec![(1, key(7)), (2, key(9))]);
    let ciphertext = ring.encrypt("sk-provider-secret").expect("encrypt");
    assert!(
        ciphertext.starts_with("v2:"),
        "highest version encrypts: {ciphertext}"
    );
    let decrypted = ring.decrypt(&ciphertext).expect("decrypt");
    assert_eq!(decrypted.value, "sk-provider-secret");
    assert!(!decrypted.legacy);
    assert_eq!(decrypted.version, Some(2));
    assert!(
        !ciphertext.contains("sk-provider-secret"),
        "plaintext must not appear in ciphertext"
    );
}

#[test]
fn tampered_ciphertext_is_rejected() {
    let ring = ring(vec![(1, key(3))]);
    let ciphertext = ring.encrypt("secret").expect("encrypt");
    let (prefix, body) = ciphertext.split_once(':').expect("v1: prefix");
    let mut chars: Vec<char> = body.chars().collect();
    let last = chars.len() - 1;
    chars[last] = if chars[last] == 'A' { 'B' } else { 'A' };
    let tampered: String = chars.into_iter().collect();
    let result = ring.decrypt(&format!("{prefix}:{tampered}"));
    assert_eq!(result, Err(KeyError::InvalidCiphertext));
    assert_eq!(ring.decrypt("v1:AAAA"), Err(KeyError::InvalidCiphertext));
}

#[test]
fn legacy_plaintext_is_readable_but_new_writes_are_encrypted() {
    let ring = ring(vec![(1, key(5))]);
    let decrypted = ring.decrypt("legacy-plaintext-key").expect("legacy read");
    assert_eq!(decrypted.value, "legacy-plaintext-key");
    assert!(decrypted.legacy);
    assert_eq!(decrypted.version, None);
    let migrated = ring
        .reencrypt("legacy-plaintext-key")
        .expect("re-encrypt legacy")
        .expect("some");
    assert!(migrated.starts_with("v1:"));
    assert_eq!(
        ring.decrypt(&migrated).expect("decrypt migrated").value,
        "legacy-plaintext-key"
    );
}

#[test]
fn missing_key_fails_closed_for_writes() {
    let empty = KeyRing::empty();
    assert!(empty.is_empty());
    assert_eq!(empty.encrypt("secret"), Err(KeyError::Missing));
    assert_eq!(empty.decrypt("v1:QUJD"), Err(KeyError::Missing));
    // Legacy plaintext remains readable even without a key (documented migration path).
    assert_eq!(
        empty.decrypt("plain-old-value").expect("legacy read").value,
        "plain-old-value"
    );
}

#[test]
fn rotation_keeps_old_ciphertext_readable_and_reselects_highest() {
    let old = ring(vec![(1, key(1))]);
    let old_ciphertext = old.encrypt("rotate-me").expect("v1 encrypt");
    assert!(old_ciphertext.starts_with("v1:"));
    let both = ring(vec![(1, key(1)), (2, key(2))]);
    assert_eq!(
        both.decrypt(&old_ciphertext).expect("v1 readable").version,
        Some(1)
    );
    let rotated = both
        .reencrypt(&old_ciphertext)
        .expect("reencrypt")
        .expect("some");
    assert!(rotated.starts_with("v2:"));
    assert_eq!(
        both.decrypt(&rotated).expect("v2 readable").value,
        "rotate-me"
    );
    let only_new = ring(vec![(2, key(2))]);
    assert_eq!(
        only_new.decrypt(&old_ciphertext),
        Err(KeyError::UnknownVersion(1))
    );
    assert_eq!(both.reencrypt(&rotated).expect("already newest"), None);
}

#[test]
fn key_parsing_accepts_hex_and_base64_and_rejects_short_values() {
    let hex = [0xab_u8; KEY_BYTES];
    let hex_value = hex
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(parse_key(&hex_value).expect("hex key"), hex);
    use base64::Engine;
    let base64_value = base64::engine::general_purpose::STANDARD.encode(hex);
    assert_eq!(parse_key(&base64_value).expect("base64 key"), hex);
    assert_eq!(parse_key("too-short"), Err(KeyError::InvalidConfig));
    assert_eq!(
        KeyRing::from_pairs(vec![(0, key(1))]),
        Err(KeyError::InvalidConfig)
    );
    assert_eq!(
        KeyRing::from_pairs(vec![(1, key(0))]),
        Err(KeyError::InvalidConfig)
    );
    assert_eq!(
        KeyRing::from_pairs(vec![(1, key(1)), (1, key(2))]),
        Err(KeyError::InvalidConfig)
    );
}

#[cfg(unix)]
#[test]
fn group_or_world_readable_key_file_is_refused() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new("perms");
    let path = dir.path().join("master.key.v1");
    fs::write(&path, "hex-or-base64").expect("write");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("chmod 644");
    assert_eq!(
        KeyRing::load_from_dir(dir.path()),
        Err(KeyError::Permissions)
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("chmod 600");
    assert!(
        KeyRing::load_from_dir(dir.path()).is_err(),
        "invalid key material still rejected"
    );
}

#[test]
fn directory_loading_reads_numbered_versions() {
    use base64::Engine;
    let dir = TempDir::new("load");
    for (version, seed) in [(1_u32, 11_u8), (2, 12)] {
        let path = dir.path().join(format!("master.key.v{version}"));
        fs::write(
            &path,
            base64::engine::general_purpose::STANDARD.encode(key(seed)),
        )
        .expect("write key");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("chmod 600");
        }
    }
    fs::write(dir.path().join("README.txt"), "ignored").expect("ignore file");
    let loaded = KeyRing::load_from_dir(dir.path()).expect("load ring");
    assert_eq!(loaded.versions(), vec![1, 2]);
    assert_eq!(loaded.highest_version(), Some(2));
}

/// Real rotation drill driven by `scripts/master-key-rotate.sh` (offline key files).
#[test]
fn env_ring_drill() {
    let Ok(both_dir) = std::env::var("ZEOKEEL_MASTER_KEY_DRILL_BOTH") else {
        eprintln!(
            "skipped: ZEOKEEL_MASTER_KEY_DRILL_BOTH unset (run scripts/master-key-rotate.sh)"
        );
        return;
    };
    let one_dir = std::env::var("ZEOKEEL_MASTER_KEY_DRILL_V1").expect("V1 dir");
    let bad_dir = std::env::var("ZEOKEEL_MASTER_KEY_DRILL_BAD").expect("bad dir");
    let both = KeyRing::load_from_dir(Path::new(&both_dir)).expect("load both ring");
    let v1 = KeyRing::load_from_dir(Path::new(&one_dir)).expect("load v1 ring");
    assert_eq!(both.versions(), vec![1, 2]);
    assert_eq!(v1.versions(), vec![1]);
    assert_eq!(
        KeyRing::load_from_dir(Path::new(&bad_dir)),
        Err(KeyError::Permissions)
    );
    println!("master-key 0644 key refused: OK");

    let v1_ciphertext = v1.encrypt("value-under-v1").expect("v1 encrypt");
    assert!(v1_ciphertext.starts_with("v1:"));
    let v2_ciphertext = both.encrypt("value-under-v2").expect("v2 encrypt");
    assert!(v2_ciphertext.starts_with("v2:"));
    assert_eq!(
        both.decrypt(&v1_ciphertext)
            .expect("read v1 with both")
            .value,
        "value-under-v1"
    );
    assert_eq!(
        both.decrypt(&v2_ciphertext).expect("read v2").version,
        Some(2)
    );

    let rotated = both
        .reencrypt(&v1_ciphertext)
        .expect("rotate v1")
        .expect("some");
    assert!(rotated.starts_with("v2:"));
    assert_eq!(
        both.decrypt(&rotated).expect("read rotated").value,
        "value-under-v1"
    );
    println!(
        "master-key env ring drill: OK (versions {:?}, rotation v1->v2 verified)",
        both.versions()
    );
}
