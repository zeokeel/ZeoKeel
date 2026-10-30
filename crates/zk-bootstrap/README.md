# zk-bootstrap

Platform **bootstrap** crate (ADR-009): the one non-package piece of code that creates the
Cordis kernel root and gives both apps (`apps/desktop`, `apps/server`) the same lifecycle.

- Creates one `zeokeel_cordis::Context::root()` per process and keeps it alive.
- Registers plugins/fibers (`register_plugin`) so their effects are owned by the kernel.
- Provides the host implementations of the kernel contract services
  (`zk-contract-kernel::{Clock, ConfigStore, TaskScheduler}`) as named services
  (`"kernel.clock"`, `"kernel.config"`, `"kernel.tasks"`) that plugins may `inject`.
- Runs recurring host work as kernel effects (`spawn_task` + abort-on-dispose) instead of
  bare `tokio::spawn` calls.
- Shuts down through `Context::dispose_all()`: plugin effects run in reverse order, exactly
  once, and shutdown is idempotent.

It contains **no product logic** (ADR-009): only create/configure/register/dispose. App
identity (edition, version, composed packages, server state) is registered by the app
entrypoint as a plugin/service.

Docs: `docs/05` §1–§2 (client host), `docs/06` §1 (server host), `docs/24` §6 (shutdown),
`docs/10` §1 (repository layout).
