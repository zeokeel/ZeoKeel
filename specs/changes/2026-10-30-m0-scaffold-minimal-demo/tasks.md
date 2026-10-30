# 2026-10-30-m0-scaffold-minimal-demo · 任务清单

> 每个任务可独立提交/验证；没有 verify 的不算任务。

## 1. 任务表

| # | 任务 | 写作用域 | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | spec 五件套 + 状态 approved | `specs/changes/2026-10-30-m0-scaffold-minimal-demo/` | — | `python3 scripts/spec-check.py` | ☑ |
| 2 | 根 workspace + toolchain + zk.toml + editions/profiles/brands + 契约 crates | `Cargo.toml`, `rust-toolchain.toml`, `zk.toml`, `editions/`, `profiles/`, `brands/`, `packages/contracts/` | 1 | `cargo metadata --offline` | ☐ |
| 3 | client 包：tool-fs / llm-mock / agent-loop + 单测 + manifests | `packages/client/` | 2 | `cargo test -p zk-tool-fs -p zk-llm-mock -p zk-agent-loop` | ☐ |
| 4 | `zk` v0：validate/compose/plan/capability/find；模板与生成 | `tools/zk/` | 2,3 | `cargo run -p zk -- validate`；`compose --check` | ☐ |
| 5 | 生成 glue + app Cargo + 两端 app（Tauri/axum）+ web demo | `crates/zk-composed/`, `apps/` | 4 | `cargo check -p zk-desktop -p zk-server` | ☐ |
| 6 | 脚本/CI/README + 全量验证 + acceptance | `scripts/`, `.github/`, `README.md`, spec acceptance | 5 | `cargo test --workspace`; `docs-check`; `spec-check` | ☐ |

## 2. 写作用域

- 本变更只写：本目录、根配置、packages/contracts|client、tools/zk、crates/zk-composed、apps/、scripts/、.github/、README.md。
- 不写：docs/ 规范正文（除必要更正）、experiments/、research/、specs/archive/。
- 并行：package 实现（A）与 app 实现（B）写作用域不重叠；根配置/生成器由 Lead 写。

## 3. 提交顺序

1. 任务 1：spec
2. 任务 2：workspace + 契约
3. 任务 3：client 包
4. 任务 4：zk 生成器
5. 任务 5：两端 app + UI
6. 任务 6：测试/CI/证据

## 4. 验收前检查

- [ ] `python3 scripts/spec-check.py`
- [ ] `python3 scripts/docs-check.py`
- [ ] `zk validate` / `compose --check` / `capability --check`
- [ ] `cargo test --workspace --offline`
- [ ] 无手改 `@generated`
- [ ] PR 只包含本 spec 范围
- [ ] `acceptance.md` 已填证据路径
