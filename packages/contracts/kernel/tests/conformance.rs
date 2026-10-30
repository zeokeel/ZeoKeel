//! Contract conformance: traits are object-safe and usable.
#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use std::sync::Arc;
use zk_contract_kernel::{Clock, ConfigStore, TaskScheduler};

struct Services;

impl Clock for Services {
    fn now_ms(&self) -> u64 {
        0
    }
}
impl ConfigStore for Services {
    fn get(&self, _key: &str) -> Option<String> {
        None
    }
}
impl TaskScheduler for Services {
    fn schedule(&self, _name: &str) {}
}

#[test]
fn traits_are_object_safe() {
    let s: Arc<dyn Clock> = Arc::new(Services);
    let c: Arc<dyn ConfigStore> = Arc::new(Services);
    let t: Arc<dyn TaskScheduler> = Arc::new(Services);
    assert_eq!(s.now_ms(), 0);
    assert!(c.get("x").is_none());
    t.schedule("demo");
}
