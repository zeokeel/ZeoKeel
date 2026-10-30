# 2026-10-30-wasm-app-center · 任务（M4）

| # | 任务 | 写作用域 | verify |
| --- | --- | --- | --- |
| 0 | P2 原型证据归档 | `target/evidence/m0/wasm/`（不入库） | `experiments/wasm-poc/build.sh` + `golden/run.py` |
| 1 | `zk-contract-app` + schema | `packages/contracts/app`, `shared/contracts` | `cargo test -p zk-contract-app`, `schema-check` |
| 2 | `crates/zk-kit-wasm`（抽取 prototype runtime） | `crates/zk-kit-wasm` | `cargo test -p zk-kit-wasm` + golden |
| 3 | `srv.apps` 迁移/组合根 | `packages/server/apps`, `apps/server/src/main.rs`, template, zk.toml, editions | `zk validate`, `compose --check` |
| 4 | apps 管理 API/审批/上架/下架/退役 | `packages/server/apps` | `scripts/srv-store-test.sh` 扩展 |
| 5 | 客户端 apps API/open/request + 限额 | `packages/server/apps` | `scripts/apps-e2e.sh` |
| 6 | admin-apps 页面 | `packages/admin/admin-apps`, registry | `admin-e2e.sh` + axe |
| 7 | ui-apps 面板/隔离窗口/错误页 | `packages/ui/ui-apps`, `apps/desktop` | Playwright + axe |
| 8 | Go 示例应用 + golden | `examples/wasm`, `scripts/apps-e2e.sh` | frame golden |
| 9 | 限额/安全负向（DB/AI/network/memory/fuel/concurrency） | tests | `cargo test`/e2e |
| 10 | 文档/验收/归档 | docs, acceptance | docs-check/spec-check/review |

## 依赖

- 1→2→3→4→5→6→7→8→9→10。
