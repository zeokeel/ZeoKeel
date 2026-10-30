//! Platform bootstrap for both ZeoKeel apps (ADR-009).
//!
//! This is the single non-package entrypoint that owns the Cordis kernel root
//! (`zeokeel-cordis` 0.6): it creates one [`Context`], registers the host services
//! required by `zk-contract-kernel` (`Clock`/`ConfigStore`/`TaskScheduler`), lets the
//! app load plugins whose effects are kernel-owned, runs recurring host work as
//! kernel effects, and disposes everything through `Context::dispose_all()`.
//!
//! It contains no product logic (ADR-009): only create/configure/register/dispose.
//! Apps register their identity/composed runtime as a plugin or service on top.

#![forbid(unsafe_code)]

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use zk_contract_kernel::Clock as ContractClock;
use zk_contract_kernel::ConfigStore;
use zk_contract_kernel::TaskScheduler;

/// Result alias of the Cordis kernel.
pub use zeokeel_cordis::Result as CordisResult;
pub use zeokeel_cordis::{cleanup, Context, Fiber, FiberState, Plugin, ServiceKey};

/// Non-secret environment keys exposed through the `kernel.config` service.
///
/// Secrets (database URLs, API keys, tokens) are intentionally excluded: the
/// config snapshot is meant for diagnostics/services, not credential delivery.
pub const DEFAULT_CONFIG_KEYS: &[&str] = &[
    "ZEOKEEL_SERVER_ADDR",
    "ZEOKEEL_ADMIN_DIST",
    "ZEOKEEL_UPDATE_DIR",
    "ZEOKEEL_APP_DB_DIR",
    "ZEOKEEL_HOME",
];

/// Named service: wall-clock host implementation (`zk-contract-kernel::Clock`).
pub static CLOCK_SERVICE: ServiceKey<dyn ContractClock> = ServiceKey::new("kernel.clock");
/// Named service: read-only non-secret configuration snapshot.
pub static CONFIG_SERVICE: ServiceKey<dyn ConfigStore> = ServiceKey::new("kernel.config");
/// Named service: named-task registry (`zk-contract-kernel::TaskScheduler`).
pub static SCHEDULER_SERVICE: ServiceKey<dyn TaskScheduler> = ServiceKey::new("kernel.tasks");

/// Bootstrap/host lifecycle errors.
#[derive(Debug, thiserror::Error)]
pub enum BootstrapError {
    /// The kernel rejected an operation.
    #[error("kernel error: {0}")]
    Kernel(#[from] zeokeel_cordis::CordisError),
    /// A background task with the same name is already registered.
    #[error("background task `{0}` is already registered")]
    DuplicateTask(String),
    /// `spawn_task` was called outside a tokio runtime.
    #[error("background tasks require a tokio runtime")]
    NoRuntime,
}

/// Host implementation of `zk-contract-kernel::Clock` (wall clock, ms since epoch).
#[derive(Debug, Default, Clone, Copy)]
pub struct HostClock;

impl ContractClock for HostClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| {
                u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
            })
    }
}

/// Read-only snapshot of non-secret configuration values.
#[derive(Debug, Clone, Default)]
pub struct ConfigSnapshot {
    values: Arc<BTreeMap<String, String>>,
}

impl ConfigSnapshot {
    /// Build a snapshot from key/value pairs.
    #[must_use]
    pub fn new(pairs: impl IntoIterator<Item = (String, String)>) -> Self {
        Self {
            values: Arc::new(pairs.into_iter().collect()),
        }
    }

    /// Build a snapshot from the given environment variables (missing keys are skipped).
    #[must_use]
    pub fn from_env(keys: &[&str]) -> Self {
        Self::new(keys.iter().filter_map(|key| {
            std::env::var(key)
                .ok()
                .map(|value| ((*key).to_owned(), value))
        }))
    }

    /// Look up one value without cloning.
    #[must_use]
    pub fn get_value(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }
}

impl ConfigStore for ConfigSnapshot {
    fn get(&self, key: &str) -> Option<String> {
        self.values.get(key).cloned()
    }
}

/// Names of tasks registered with the bootstrap (and not yet disposed).
#[derive(Debug, Clone, Default)]
struct TaskRegistry {
    names: Arc<Mutex<BTreeSet<String>>>,
}

impl TaskRegistry {
    fn register(&self, name: &str) -> bool {
        self.names
            .lock()
            .map(|mut names| names.insert(name.to_owned()))
            .unwrap_or(false)
    }

    fn release(&self, name: &str) {
        if let Ok(mut names) = self.names.lock() {
            names.remove(name);
        }
    }

    fn snapshot(&self) -> Vec<String> {
        self.names
            .lock()
            .map(|names| names.iter().cloned().collect())
            .unwrap_or_default()
    }
}

/// Host implementation of `zk-contract-kernel::TaskScheduler`.
///
/// `schedule()` declares a named task; [`Bootstrap::spawn_task`] both declares and
/// runs one, and the name is released when the task is aborted/disposed.
#[derive(Debug, Clone, Default)]
pub struct Scheduler {
    registry: TaskRegistry,
}

impl TaskScheduler for Scheduler {
    fn schedule(&self, name: &str) {
        if !self.registry.register(name) {
            // Duplicate registration is a no-op for the minimal M0 contract surface.
        }
    }
}

/// Handle to one kernel-owned background task.
///
/// Dropping the handle detaches the task (like `tokio::spawn`); call [`TaskHandle::abort`]
/// or dispose the bootstrap to stop it. Disposal aborts the task and releases its name.
#[derive(Clone)]
pub struct TaskHandle {
    inner: Arc<TaskHandleInner>,
}

struct TaskHandleInner {
    name: String,
    join: Mutex<Option<tokio::task::JoinHandle<()>>>,
    registry: TaskRegistry,
}

impl TaskHandle {
    /// Task name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    /// Abort the task (idempotent); releases the registered name.
    pub fn abort(&self) {
        let handle = match self.inner.join.lock() {
            Ok(mut slot) => slot.take(),
            Err(poisoned) => poisoned.into_inner().take(),
        };
        if let Some(handle) = handle {
            handle.abort();
        }
        self.inner.registry.release(&self.inner.name);
    }

    /// Wait for the task to finish or be aborted.
    pub async fn wait(&self) {
        let handle = match self.inner.join.lock() {
            Ok(mut slot) => slot.take(),
            Err(poisoned) => poisoned.into_inner().take(),
        };
        if let Some(handle) = handle {
            let _ = handle.await;
        }
    }

    /// Whether the task has already finished (or this handle was aborted/waited on).
    #[must_use]
    pub fn is_finished(&self) -> bool {
        match self.inner.join.lock() {
            Ok(slot) => slot
                .as_ref()
                .is_none_or(tokio::task::JoinHandle::is_finished),
            Err(_) => true,
        }
    }
}

/// Kernel host: one Cordis root plus the host services both apps share.
pub struct Bootstrap {
    host: &'static str,
    root: Context,
    config: ConfigSnapshot,
    scheduler: Scheduler,
    shutdown: AtomicBool,
}

impl Bootstrap {
    /// Create the kernel root and register the host services.
    ///
    /// # Errors
    /// Returns [`BootstrapError::Kernel`] when the kernel rejects service registration.
    pub fn boot(host: &'static str) -> Result<Self, BootstrapError> {
        let root = Context::root();
        // Kernel clock (logger timestamps / monotonic idle tracking); the
        // contract `Clock` service below is the app-facing wall clock.
        root.set_clock(Arc::new(zeokeel_cordis::SystemClock))?;
        let config = ConfigSnapshot::from_env(DEFAULT_CONFIG_KEYS);
        let scheduler = Scheduler::default();
        root.provide_any(CLOCK_SERVICE, Arc::new(HostClock))?;
        root.provide_any(CONFIG_SERVICE, Arc::new(config.clone()))?;
        root.provide_any(SCHEDULER_SERVICE, Arc::new(scheduler.clone()))?;
        Ok(Self {
            host,
            root,
            config,
            scheduler,
            shutdown: AtomicBool::new(false),
        })
    }

    /// Host name passed to [`Bootstrap::boot`] (e.g. `zk-desktop`).
    #[must_use]
    pub fn host(&self) -> &'static str {
        self.host
    }

    /// Kernel root context.
    #[must_use]
    pub fn context(&self) -> &Context {
        &self.root
    }

    /// Non-secret configuration snapshot created at boot.
    #[must_use]
    pub fn config(&self) -> &ConfigSnapshot {
        &self.config
    }

    /// Named-task registry shared with [`Bootstrap::spawn_task`].
    #[must_use]
    pub fn scheduler(&self) -> &Scheduler {
        &self.scheduler
    }

    /// Load one plugin; its effects are owned by the kernel until dispose.
    ///
    /// # Errors
    /// Propagates kernel activation errors (config/injection failure).
    pub fn register_plugin(
        &self,
        plugin: Arc<dyn Plugin>,
        config: Value,
    ) -> Result<Fiber, BootstrapError> {
        Ok(self.root.plugin(plugin, config)?)
    }

    /// Register a typed service on the root context.
    ///
    /// # Errors
    /// Propagates kernel ownership/type errors.
    pub fn provide<T: Any + Send + Sync>(
        &self,
        key: ServiceKey<T>,
        value: Arc<T>,
    ) -> Result<(), BootstrapError> {
        self.root.provide(key, value)?;
        Ok(())
    }

    /// Register a trait-object service on the root context.
    ///
    /// # Errors
    /// Propagates kernel ownership/type errors.
    pub fn provide_any<T: ?Sized + Send + Sync + 'static>(
        &self,
        key: ServiceKey<T>,
        value: Arc<T>,
    ) -> Result<(), BootstrapError> {
        self.root.provide_any(key, value)?;
        Ok(())
    }

    /// Read a typed service.
    ///
    /// # Errors
    /// [`BootstrapError::Kernel`] when the service is missing or unavailable.
    pub fn require<T: Any + Send + Sync>(
        &self,
        key: ServiceKey<T>,
    ) -> Result<Arc<T>, BootstrapError> {
        Ok(self.root.require(key)?)
    }

    /// Read a trait-object service.
    ///
    /// # Errors
    /// [`BootstrapError::Kernel`] when the service is missing or unavailable.
    pub fn require_any<T: ?Sized + Send + Sync + 'static>(
        &self,
        key: ServiceKey<T>,
    ) -> Result<Arc<T>, BootstrapError> {
        Ok(self.root.require_any(key)?)
    }

    /// Host clock service.
    ///
    /// # Errors
    /// [`BootstrapError::Kernel`] when the service is not registered.
    pub fn clock(&self) -> Result<Arc<dyn ContractClock>, BootstrapError> {
        self.require_any(CLOCK_SERVICE)
    }

    /// Host config service.
    ///
    /// # Errors
    /// [`BootstrapError::Kernel`] when the service is not registered.
    pub fn config_store(&self) -> Result<Arc<dyn ConfigStore>, BootstrapError> {
        self.require_any(CONFIG_SERVICE)
    }

    /// Host task-scheduler service.
    ///
    /// # Errors
    /// [`BootstrapError::Kernel`] when the service is not registered.
    pub fn task_scheduler(&self) -> Result<Arc<dyn TaskScheduler>, BootstrapError> {
        self.require_any(SCHEDULER_SERVICE)
    }

    /// Run `future` as a kernel-owned effect; abort it when the bootstrap is disposed.
    ///
    /// The task name must be unique among active tasks. On abort/dispose the
    /// future is dropped (abort-on-dispose) and the name is released, so a task
    /// cannot leak past shutdown.
    ///
    /// # Errors
    /// [`BootstrapError::DuplicateTask`], [`BootstrapError::NoRuntime`], or a kernel error.
    pub fn spawn_task<F>(&self, name: &'static str, future: F) -> Result<TaskHandle, BootstrapError>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        if !self.scheduler.registry.register(name) {
            return Err(BootstrapError::DuplicateTask(name.to_owned()));
        }
        let runtime = match tokio::runtime::Handle::try_current() {
            Ok(handle) => handle,
            Err(_) => {
                self.scheduler.registry.release(name);
                return Err(BootstrapError::NoRuntime);
            }
        };
        let registry = self.scheduler.registry.clone();
        let tracked_name = name.to_owned();
        let join = runtime.spawn(async move {
            future.await;
            registry.release(&tracked_name);
        });
        let handle = TaskHandle {
            inner: Arc::new(TaskHandleInner {
                name: name.to_owned(),
                join: Mutex::new(Some(join)),
                registry: self.scheduler.registry.clone(),
            }),
        };
        let cleanup_handle = handle.clone();
        if let Err(error) = self
            .root
            .effect_labeled(name, move || vec![cleanup(move || cleanup_handle.abort())])
        {
            handle.abort();
            return Err(error.into());
        }
        Ok(handle)
    }

    /// Names of currently registered (not yet disposed) tasks.
    #[must_use]
    pub fn active_tasks(&self) -> Vec<String> {
        self.scheduler.registry.snapshot()
    }

    /// Gracefully dispose every plugin effect (reverse order) and the root fiber.
    ///
    /// Idempotent: the second and later calls are no-ops.
    ///
    /// # Errors
    /// Returns the first cleanup error after attempting every fiber.
    pub fn shutdown(&self) -> Result<(), BootstrapError> {
        if self.shutdown.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        self.root.dispose_all()?;
        Ok(())
    }

    /// Alias for [`Bootstrap::shutdown`] (kernel naming).
    ///
    /// # Errors
    /// See [`Bootstrap::shutdown`].
    pub fn dispose_all(&self) -> Result<(), BootstrapError> {
        self.shutdown()
    }
}

/// Create the process kernel root and register the host services.
///
/// # Errors
/// Returns [`BootstrapError::Kernel`] when the kernel rejects service registration.
pub fn boot(host: &'static str) -> Result<Bootstrap, BootstrapError> {
    Bootstrap::boot(host)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize};
    use std::time::Duration;

    static PROBE: ServiceKey<u32> = ServiceKey::new("test.probe");

    struct Recorder {
        log: Arc<Mutex<Vec<&'static str>>>,
    }

    impl Plugin for Recorder {
        fn name(&self) -> &'static str {
            "test.recorder"
        }

        fn apply(&self, ctx: Context, _config: Value) -> CordisResult<()> {
            let first = Arc::clone(&self.log);
            ctx.effect(move || vec![cleanup(move || push(&first, "first"))])?;
            let second = Arc::clone(&self.log);
            ctx.effect(move || vec![cleanup(move || push(&second, "second"))])?;
            Ok(())
        }
    }

    fn push(log: &Arc<Mutex<Vec<&'static str>>>, value: &'static str) {
        if let Ok(mut guard) = log.lock() {
            guard.push(value);
        }
    }

    fn cleanup_count(
        value: &'static str,
        counter: Arc<AtomicUsize>,
    ) -> impl FnOnce() + Send + 'static {
        move || {
            counter.fetch_add(1, Ordering::SeqCst);
            let _ = value;
        }
    }

    struct CountingPlugin {
        counter: Arc<AtomicUsize>,
    }

    impl Plugin for CountingPlugin {
        fn name(&self) -> &'static str {
            "test.counting"
        }

        fn apply(&self, ctx: Context, _config: Value) -> CordisResult<()> {
            let counter = Arc::clone(&self.counter);
            ctx.effect(move || vec![cleanup(cleanup_count("cleanup", counter))])?;
            Ok(())
        }
    }

    struct Consumer;

    impl Plugin for Consumer {
        fn name(&self) -> &'static str {
            "test.consumer"
        }

        fn inject(&self) -> &'static [&'static str] {
            &["test.probe"]
        }

        fn apply(&self, ctx: Context, _config: Value) -> CordisResult<()> {
            let value = ctx.require(PROBE)?;
            assert_eq!(*value, 7);
            Ok(())
        }
    }

    #[test]
    fn plugin_effects_run_in_reverse_order() {
        let bootstrap = Bootstrap::boot("test").unwrap();
        let log = Arc::new(Mutex::new(Vec::new()));
        bootstrap
            .register_plugin(
                Arc::new(Recorder {
                    log: Arc::clone(&log),
                }),
                Value::Null,
            )
            .unwrap();
        bootstrap.shutdown().unwrap();
        assert_eq!(*log.lock().unwrap(), vec!["second", "first"]);
    }

    #[test]
    fn shutdown_is_idempotent_and_runs_cleanup_once() {
        let bootstrap = Bootstrap::boot("test").unwrap();
        let counter = Arc::new(AtomicUsize::new(0));
        bootstrap
            .register_plugin(
                Arc::new(CountingPlugin {
                    counter: Arc::clone(&counter),
                }),
                Value::Null,
            )
            .unwrap();
        bootstrap.shutdown().unwrap();
        bootstrap.shutdown().unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn services_gate_plugin_injection() {
        let bootstrap = Bootstrap::boot("test").unwrap();
        let fiber = bootstrap
            .register_plugin(Arc::new(Consumer), Value::Null)
            .unwrap();
        assert_eq!(fiber.state(), FiberState::Pending);
        bootstrap.provide(PROBE, Arc::new(7u32)).unwrap();
        assert!(fiber.is_active());
        bootstrap.shutdown().unwrap();
    }

    #[test]
    fn host_services_are_registered() {
        let bootstrap = Bootstrap::boot("test").unwrap();
        assert!(bootstrap.clock().unwrap().now_ms() > 0);
        assert!(bootstrap
            .config()
            .get_value("ZEOKEEL_NOT_CONFIGURED_PROBE")
            .is_none());
        let scheduler = bootstrap.task_scheduler().unwrap();
        scheduler.schedule("declared.only");
        assert!(bootstrap
            .active_tasks()
            .contains(&"declared.only".to_owned()));
        bootstrap.shutdown().unwrap();
    }

    #[tokio::test]
    async fn task_is_aborted_on_shutdown() {
        struct DropGuard(Arc<AtomicBool>);
        impl Drop for DropGuard {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }

        let bootstrap = Bootstrap::boot("test").unwrap();
        let dropped = Arc::new(AtomicBool::new(false));
        let guard = DropGuard(Arc::clone(&dropped));
        bootstrap
            .spawn_task("test.pending", async move {
                let _guard = guard;
                std::future::pending::<()>().await;
            })
            .unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(!dropped.load(Ordering::SeqCst));
        bootstrap.shutdown().unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(dropped.load(Ordering::SeqCst));
        assert!(bootstrap.active_tasks().is_empty());
    }

    #[tokio::test]
    async fn duplicate_task_names_are_rejected() {
        let bootstrap = Bootstrap::boot("test").unwrap();
        let first = bootstrap
            .spawn_task("test.same", std::future::pending::<()>())
            .unwrap();
        assert!(matches!(
            bootstrap.spawn_task("test.same", std::future::pending::<()>()),
            Err(BootstrapError::DuplicateTask(_))
        ));
        first.abort();
        assert!(bootstrap
            .spawn_task("test.same", std::future::pending::<()>())
            .is_ok());
        bootstrap.shutdown().unwrap();
    }
}
