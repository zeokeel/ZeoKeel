# 2026-10-30-wasm-app-center · 任务清单

> 每个任务必须能独立提交、独立验证；没有 `verify` 的任务不算任务。

## 1. 任务表

| # | 任务 | 写作用域（文件/目录） | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 0 | P2 原型证据归档 | `target/evidence/m0/wasm/`（不入库） | — | `experiments/wasm-poc/build.sh` && `python3 golden/run.py` 输出 `ok` | ✅ |
| 1 | `zk-contract-app` + schemas | `packages/contracts/app`, `shared/contracts` | — | `cargo test -p zk-contract-app export_schema`; `bash scripts/schema-check.sh` | ✅ |
| 2 | `zk-kit-wasm` 抽取 runtime/帧/SQL 闸门 | `crates/zk-kit-wasm`, `Cargo.toml` | 1 | `cargo test -p zk-kit-wasm` + Go/Rust golden | ✅ |
| 3 | `srv.apps`/`srv.app.runtime`/`srv.app.db` 骨架与迁移/组合根 | `packages/server/{apps,app-runtime,app-db}`, `apps/server/src/main.rs`, template, zk.toml, editions | 1,2 | `zk validate`, `zk compose --check`, `srv-store-test.sh` 表存在 | ✅ |
| 4 | 员工上传/owner/transfer + 管理审批/版本 | `packages/server/apps` | 3 | `scripts/srv-store-test.sh` 新增 apps 用例（owner 403/转交/pending） | ✅ |
| 5 | 客户端 open/request + 限额/审计 + `scripts/apps-e2e.sh` 骨架（fixture guest） | `packages/server/{apps,app-runtime,app-db}`, `scripts/apps-e2e.sh`, `e2e/fixtures/app-guest.wasm` | 4 | `bash scripts/apps-e2e.sh`（fixture 上传/open/request/重启） | ✅ |
| 6 | admin-apps 页面 + registry | `packages/admin/admin-apps`, `zk.toml`, editions, docs16 | 4 | `admin-e2e.sh` + axe | ✅ |
| 7 | `app_request` IPC/capability + ui-apps 面板/窗口/错误页 | `packages/contracts/ipc`, `packages/ui/ui-apps`, `apps/desktop`, `zk.toml`, `apps/desktop/capabilities/*` | 5 | `cargo test -p zk-contract-ipc`; Playwright + axe；窗口其他 IPC 403 负向 | ✅ |
| 8 | Go 示例应用（表单/数据）+ frame golden（第二个示例；脚本已在 5） | `examples/wasm`, `scripts/apps-e2e.sh` | 5 | `apps-e2e.sh`，`golden/run.py` | ✅ |
| 9 | 安全/限额负向（SQL/AI/network/file/DB/memory/fuel/time/concurrency） | tests, `scripts/apps-e2e.sh` | 5,8 | `cargo test` + `apps-e2e.sh` 每项错误码 | ✅ |
| 10 | 文档/验收/归档 | docs/12/16/21, acceptance, spec 归档 | 1–9 | `docs-check`, `spec-check`, `review-check` | ✅ |

## 2. 写作用域

- 本变更只写：上述目录、`scripts/{apps-e2e,srv-store-test,admin-e2e}.sh`、`e2e/*app*`、`docs/{04,07,12,13,16,21}`、`zk.toml`、`editions/enterprise.toml`、`tools/zk/templates/app_server.Cargo.toml`。
- 不写：M0–M3 迁移历史、`@generated`（由 compose 生成）、其他业务包。

## 3. 提交顺序

1. 0/1/2（证据/契约/kit）
2. 3/4（包/迁移/上传权限）
3. 5/6/7（API/UI）
4. 8/9（示例/安全）
5. 10（验收/归档）

## 4. 验收前检查

- [ ] `python3 scripts/spec-check.py`
- [ ] `python3 scripts/docs-check.py`
- [ ] 涉及的 T0/T1/T3 命令全绿（`apps-e2e.sh`/`admin-e2e.sh`/`cargo test`/`schema-check`）
- [ ] 无手改 `@generated`（`compose --check`）
- [ ] 复用检查（`zk find` + `zk capability --check`；`zk-kit-wasm` 已登记）
- [ ] `acceptance.md` 已填证据路径
