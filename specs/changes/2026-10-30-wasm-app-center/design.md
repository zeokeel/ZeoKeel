# 2026-10-30-wasm-app-center · 设计（M4）

- 状态：`draft`

## 1. 包/契约

- `packages/contracts/app`：App/AppRelease/AppOpen/Frame envelope、错误码；schemars schema。
- `packages/server/apps`（`zk-srv-apps`，role `srv.apps`）：迁移 `0012_apps`；管理/客户端路由；`runtime` 模块 wasmtime。
- `packages/server/app-runtime` 是否拆包：M4 先合在 `srv.apps`，避免过早拆；若 >800 行再拆 `srv.app-runtime`（C3 四件套）。
- `packages/admin/admin-apps`（`admin.page.apps`）。
- `packages/ui/ui-apps`（`ui.slot.apps`）：目录/打开/错误页；桌面独立窗口用 `window.open`/Tauri `WebviewWindow`；serve 用 sandbox iframe。
- 示例：`examples/wasm/forms-go`、`examples/wasm/data-go`（Go wasip1，复用 `experiments/wasm-poc/guest-go` 帧库）。

## 2. 运行时

- 复用 prototype：`experiments/wasm-poc/host` 的 linker/帧/SQLite 实现先抽到 `crates/zk-kit-wasm`（C17），`srv.apps` 与 prototype 都调用 kit；禁止复制。
- `zk-kit-wasm` 提供 `run(WasmInput) -> WasmOutput`、`limits`、`db` host；wasmtime 依赖只在 kit。
- SQL 闸门：只允许单条 SELECT/INSERT/UPDATE/DELETE/DDL 白名单；拒绝 ATTACH/PRAGMA 危险语句；表数 query `sqlite_master`；DB 文件大小用 fs metadata。
- 并发：每 app `tokio::sync::Semaphore(4)`；超时用 `tokio::time::timeout` + wasmtime epoch/fuel。
- 内存：`StoreLimitsBuilder::memory_size(64MB)`；`ResourceLimiter`。
- 审计：open/request/limits/delist 写 `audit_log`；DB/calls 只诊断。

## 3. 迁移/版本

- `0012_apps` 只新增表；不改 M3 迁移。
- 上传 release 永久占位、sha256、wasm magic（`\0asm`）；同版本 409。
- 上下架用 `content_deployments` + `contents.status`；退役 job 复用 `jobs` 模式（M4 先手动 `POST /api/admin/apps/{id}/purge`）。

## 4. 客户端隔离

- 桌面：Tauri `WebviewWindow` + 临时目录文件/自定义协议；URL 含 app id + session；窗口无 Node/preload 权限。
- serve：`<iframe sandbox="allow-scripts">` + `srcdoc` 隔离页；不共享 localStorage/cookie；请求经 postMessage 到 host，host 调 API。
- 下架/卸载：服务端 `open` 校验；已打开 host 轮询/收到 410 显示错误页。

## 5. 测试

- Rust：frame golden（Go/Rust guest）、SQL 闸门、限额、DB 隔离、迁移不可变。
- 集成：`scripts/apps-e2e.sh` 真 PG+server+guest 上传/运行/重启持久/下架。
- UI：admin apps 页面创建/上传/审批/下架 + axe；client apps 面板打开/错误页 + axe；截图。
- 独立评审：audit-arch 对 spec/实现 R1/R2 反证。

## 6. 风险/回滚

- wasmtime 编译/运行失败不影响其他域：srv.apps 独立包，可构建期 exclude/rollback 迁移只增不改。
- 若 M4 未完成 100MB/16 表强制，不得标 done；降级为 `APP_DB_QUOTA` fail-closed。
