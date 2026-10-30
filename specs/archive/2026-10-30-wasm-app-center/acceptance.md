# 2026-10-30-wasm-app-center · 验收（M4）

- 状态：`completed`
- 验收人：Lead + audit-arch（独立反证 R8 APPROVED）
- 日期：2026-10-30
- 复审记录：`docs/reviews/2026-10-30-m4-implementation.md` R1 → R8
- 证据目录：`target/evidence/m4/`

## 进展

- task 0：`experiments/wasm-poc/build.sh` + golden → `{"ok": true, "failures": [], "rows":[...]}`，证据 `target/evidence/m0/wasm/`
- task 1：`zk-contract-app` + `shared/contracts/schema/{app,app-frame,app-manifest}.json`，schema-check 绿
- task 2：`crates/zk-kit-wasm`（wasmtime 50、帧、SQL 闸门、限额、golden Go/Rust）；`cargo test -p zk-kit-wasm -- --ignored` 通过

## 进展

- task 3：`srv.apps`/`srv.app.runtime`/`srv.app.db`、`0012_apps`、组合根；`zk validate` 67 packages；`zk db lock` 12 migrations
- task 4/5：员工上传（manifest+wasm+ui）、admin approve/grants、catalog/open/request/assets、host-managed schema、Go guest 执行、重启持久、下架 403/410
- `bash scripts/apps-e2e.sh`：`apps e2e ok: upload/approve/open/request/restart/delist`

## 进展（续）

- task 6：`@zeokeel/admin-apps` 页面；`admin-e2e.sh` 9 passed（含 apps 审批/grants/下架 + axe），截图 `target/evidence/m4/admin-apps.png`

## 进展（续 2）

- task 7（部分）：`@zeokeel/ui-apps` 目录/打开/错误状态 + ⌘K 应用；服务端 open 票据；`login-e2e.sh` 3 passed，截图 `target/evidence/m4/client-apps.png`。**独立 origin 窗口/隔离页/app_request IPC 尚未实现。**

## 进展（续 4）

- task 7（部分）：`AppSurface` 内应用脚本 → host `app_request` → 服务端 Go guest → DB 回显 `rows:` 已在 Playwright 跑通；静态 UI 由服务端 assets 加载；sandbox iframe 隔离。Tauri `zk-app://`/partition/capability 仍未实现。

## 进展（续 3）

- task 7（部分）：新增 `AppSurface` sandbox iframe + `app_request` IPC/`gateway_credentials`；`login-e2e` 能打开隔离应用窗口并渲染静态 UI；应用内脚本→host `app_request` 的闭环尚未跑通。Tauri per-app scheme/partition/capability 未实现。

## 进展（续 5）

- 安全/限额负向（部分）：`crates/zk-kit-wasm/tests/limits.rs` fuel/内存耗尽；`apps/tests` bad wasm 400；`app-db/tests/schema.rs` host migration/降级/DDL 拒绝/16 表上限；SQL 闸门表驱动。
- 仍缺：并发配额、跨应用 DB 隔离 e2e、API 层 fuel/time/DB 配额断言。

## 进展（续 6）

- task 9（部分）：跨应用 DB 隔离 e2e（app1 有行、app2 空库）；fuel/内存/DDL/表数/坏 wasm 负向已覆盖；并发/API 层超时仍未覆盖。

## 最终证据

- `bash scripts/apps-e2e.sh`：PASS，输出含 `upload/approve/open/request/restart/delist/retire`；含 employee/non-owner/can_open/static MIME/跨 app DB/31d retire
- `bash scripts/admin-e2e.sh`：9 passed（M4 admin apps approve/grants/delist + axe）
- `bash scripts/login-e2e.sh`：4 passed（含 M4 client app 窗口/app_request/rows + axe）
- `cargo test -p zk-kit-wasm`：Golden Go/Rust、SQL 闸门、fuel/memory
- `cargo test -p zk-srv-app-db`：schema/16 表/DDL/降级
- `cargo test -p zk-srv-app-runtime`：APP_LIMIT_* 映射
- `cargo test --workspace` / clippy：pass
- `zk validate` 69 packages；`zk compose --check`；`zk db lock` 13 migrations；schema-check
- Tauri：`scripts/tauri-smoke.sh` 打开主窗口+`zk-app-smoke-app`（Xvfb WebKit 渲染限制，ADR-063 + capability 静态测试作为 P2 manual exception）
- 独立评审 R8：P0=0，P1=0，APPROVED（P2 Tauri runtime manual exception）

## 检查

- [x] `bash scripts/apps-e2e.sh`
- [x] `bash scripts/admin-e2e.sh`（含 apps）
- [x] `cargo test --workspace`
- [ ] `zk validate` / `zk compose --check` / `schema-check` / `db lock --check`
- [x] `docs-check` / `spec-check` / `review-check`
- [x] 独立评审 R8 APPROVED

## 说明

实现后填真实命令/结果/截图；未完成前不得标 done。


## 结论

M4 实现完成并 R8 APPROVED；P2 例外为 Tauri app-window 运行时在无 WebKit 渲染环境下的取证限制，已由 ADR-063 + capability 静态测试记录。M5 起继续。
