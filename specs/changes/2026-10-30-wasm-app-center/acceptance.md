# 2026-10-30-wasm-app-center · 验收（M4）

- 状态：`in-progress`（spec 阶段）
- 验收人：pending
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

## 进展（续 3）

- task 7（部分）：新增 `AppSurface` sandbox iframe + `app_request` IPC/`gateway_credentials`；`login-e2e` 能打开隔离应用窗口并渲染静态 UI；应用内脚本→host `app_request` 的闭环尚未跑通。Tauri per-app scheme/partition/capability 未实现。

## 检查

- [ ] `bash scripts/apps-e2e.sh`
- [ ] `bash scripts/admin-e2e.sh`（含 apps）
- [ ] `cargo test --workspace`
- [ ] `zk validate` / `zk compose --check` / `schema-check` / `db lock --check`
- [ ] `docs-check` / `spec-check` / `review-check`
- [ ] 独立评审 APPROVED

## 说明

实现后填真实命令/结果/截图；未完成前不得标 done。
