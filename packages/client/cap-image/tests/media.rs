#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test-only
//! Image capability tests with the kit's deterministic in-process model (no
//! network, no provider): prompt/size reach the model and bytes/encoding/
//! provider come back.

use std::sync::{Mutex, MutexGuard};

use zk_cap_image::{GeneratedImage, ImageEncoding, ImageError};
use zk_kit_media::test_support::{fake_image_model, FAKE_PROVIDER};
use zk_kit_media::MediaConfig;

static LOCK: Mutex<()> = Mutex::new(());

fn guard() -> MutexGuard<'static, ()> {
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn png_with(prompt: &str, width: u32, height: u32) -> Vec<u8> {
    let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    bytes.extend_from_slice(format!("{prompt}:{width}x{height}").as_bytes());
    bytes
}

#[test]
fn configured_capability_generates_image_bytes() {
    let _guard = guard();
    zk_cap_image::configure(fake_image_model()).unwrap();
    assert!(zk_cap_image::available());
    assert_eq!(zk_cap_image::unavailable_reason(), None);

    let generated: GeneratedImage = zk_cap_image::generate("a mountain lake", 64, 32).unwrap();
    assert_eq!(generated.encoding, ImageEncoding::Png);
    assert_eq!(generated.encoding.mime_type(), "image/png");
    assert_eq!(generated.provider, FAKE_PROVIDER);
    assert_eq!(generated.model.as_deref(), Some("fake-image-model"));
    // The fake echoes prompt/size into the bytes, proving the request reached
    // the model.
    assert!(
        generated
            .bytes
            .windows(b"a mountain lake:64x32".len())
            .any(|window| window == b"a mountain lake:64x32"),
        "request facts missing from {:?}",
        String::from_utf8_lossy(&generated.bytes)
    );

    zk_cap_image::unconfigure().unwrap();
    assert!(!zk_cap_image::available());
    assert!(zk_cap_image::unavailable_reason().is_some());
}

#[test]
fn empty_prompt_is_rejected_before_the_model() {
    let _guard = guard();
    zk_cap_image::configure(fake_image_model()).unwrap();
    match zk_cap_image::generate("   ", 128, 128).unwrap_err() {
        ImageError::Config(message) => assert!(message.contains("prompt is empty"), "{message}"),
        other => panic!("expected Config, got {other:?}"),
    }
    zk_cap_image::unconfigure().unwrap();
}

#[test]
fn configure_from_env_is_optional() {
    let _guard = guard();
    if std::env::var("ZEOKEEL_IMAGE_API_KEY").is_ok() {
        return;
    }
    zk_cap_image::unconfigure().unwrap();
    assert!(!zk_cap_image::configure_from_env().unwrap());
    assert!(!zk_cap_image::available());
}

#[test]
fn configure_from_config_maps_model_errors() {
    let _guard = guard();
    let error = zk_cap_image::configure_from_config(&MediaConfig::default())
        .expect_err("unconfigured media config must fail");
    match error {
        ImageError::Unavailable(message) => {
            assert!(message.contains("ZEOKEEL_IMAGE_API_KEY"), "{message}");
        }
        other => panic!("expected Unavailable, got {other:?}"),
    }
    assert!(!zk_cap_image::available());
}

#[test]
fn encoding_detection_covers_known_magic_bytes() {
    assert_eq!(
        ImageEncoding::from_bytes(&png_with("x", 1, 1)),
        ImageEncoding::Png
    );
    assert_eq!(
        ImageEncoding::from_bytes(&[0xFF, 0xD8, 0xFF, 0xE0]),
        ImageEncoding::Jpeg
    );
    assert_eq!(
        ImageEncoding::from_bytes(b"GIF89a-rest"),
        ImageEncoding::Gif
    );
    let mut webp = b"RIFF".to_vec();
    webp.extend_from_slice(&[0, 0, 0, 0]);
    webp.extend_from_slice(b"WEBP");
    webp.extend_from_slice(b"VP8 ");
    assert_eq!(ImageEncoding::from_bytes(&webp), ImageEncoding::WebP);
    assert_eq!(
        ImageEncoding::from_bytes(b"not an image"),
        ImageEncoding::Unknown
    );
    assert_eq!(
        ImageEncoding::Unknown.mime_type(),
        "application/octet-stream"
    );
}
