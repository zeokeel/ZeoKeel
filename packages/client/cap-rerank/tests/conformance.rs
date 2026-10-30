#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test-only
//! Capability conformance: honest availability plus the closed error code.

use zk_cap_rerank::{RerankError, CAPABILITY_UNAVAILABLE, UNAVAILABLE};

#[test]
fn unavailable_is_honest_and_explicit() {
    zk_cap_rerank::unconfigure().unwrap();
    assert!(!zk_cap_rerank::available());
    let reason = zk_cap_rerank::unavailable_reason().expect("reason");
    assert!(reason.contains("ZEOKEEL_RERANK_API_KEY"), "{reason}");
    assert_eq!(UNAVAILABLE, "CAPABILITY_UNAVAILABLE");
    assert_eq!(CAPABILITY_UNAVAILABLE, "CAPABILITY_UNAVAILABLE");
    let documents = vec!["a".to_owned()];
    match zk_cap_rerank::rerank("query", &documents).unwrap_err() {
        RerankError::Unavailable(message) => {
            assert!(message.contains("ZEOKEEL_RERANK_API_KEY"), "{message}");
        }
        other => panic!("expected Unavailable, got {other:?}"),
    }
}
