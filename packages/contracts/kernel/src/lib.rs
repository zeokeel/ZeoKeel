//! Kernel services contract (minimal M0 subset).
//!
//! @capability contract.kernel
#![forbid(unsafe_code)]

/// Wall-clock service (host provided; replaceable in tests).
pub trait Clock: Send + Sync {
    /// Milliseconds since UNIX epoch.
    fn now_ms(&self) -> u64;
}

/// Read-only configuration access for packages.
pub trait ConfigStore: Send + Sync {
    /// Get a configuration value by key.
    fn get(&self, key: &str) -> Option<String>;
}

/// Task registration surface (fiber/Registrar wiring lands in a later M0 task).
pub trait TaskScheduler: Send + Sync {
    /// Register a named task.
    fn schedule(&self, name: &str);
}
