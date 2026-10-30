#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
#[test]
fn availability_is_dynamic_and_explicit() {
    if std::env::var("ZEOKEEL_AGENT_HOOKS_ENABLED").ok().as_deref() == Some("0") {
        assert!(!zk_cap_hooks::available());
        assert!(zk_cap_hooks::unavailable_reason().is_some());
    } else {
        assert!(zk_cap_hooks::available());
        assert_eq!(zk_cap_hooks::unavailable_reason(), None);
    }
    zk_cap_hooks::set_available(false, "test-host-disabled");
    assert!(!zk_cap_hooks::available());
    assert!(zk_cap_hooks::unavailable_reason().is_some());
    zk_cap_hooks::set_available(true, "");
    assert!(zk_cap_hooks::available());
    assert_eq!(zk_cap_hooks::UNAVAILABLE, "CAPABILITY_UNAVAILABLE");
}
