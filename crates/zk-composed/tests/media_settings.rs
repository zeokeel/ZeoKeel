//! Generated-glue media configuration (personal baseline): one shared key/base
//! drives image/audio/transcription/rerank with per-kind model defaults; empty
//! key is a no-op (env fallback) and failures aggregate into `Err`.
#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use zk_contract_config::MediaSettings;

fn available() -> [bool; 4] {
    [
        zk_cap_image::available(),
        zk_cap_audio::available(),
        zk_cap_transcription::available(),
        zk_cap_rerank::available(),
    ]
}

#[test]
fn media_settings_configure_the_capabilities() {
    // Empty key: no-op, env fallback stays authoritative.
    zk_composed::configure_media_from_settings(&MediaSettings::default()).unwrap();
    assert_eq!(available(), [false, false, false, false]);

    // Configured: one shared key/base, per-kind models default inside the kit.
    let settings = MediaSettings {
        base_url: "https://api.example.invalid/v1".to_owned(),
        api_key: "test-key".to_owned(),
        image_model: String::new(),
        audio_model: String::new(),
        transcription_model: String::new(),
        rerank_model: String::new(),
    };
    zk_composed::configure_media_from_settings(&settings).unwrap();
    assert_eq!(available(), [true, true, true, true]);

    // Partial failure aggregates errors and returns Err; earlier kinds stay
    // configured (documented partial-success semantics).
    let bad = MediaSettings {
        base_url: "ftp://api.example.invalid/v1".to_owned(),
        api_key: "test-key".to_owned(),
        ..MediaSettings::default()
    };
    let error = zk_composed::configure_media_from_settings(&bad).unwrap_err();
    assert!(error.contains("image:"), "{error}");
    assert!(error.contains("rerank:"), "{error}");
    assert_eq!(available(), [true, true, true, true]);
}
