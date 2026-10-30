# 2026-10-31-audit-hardening · 任务

| # | 任务 | 验证 | 状态 |
| --- | --- | --- | --- |
| 1 | 修复 personal 生成物基线 + admin 空 registry + compose --check 限制 | `zk compose personal --check`=0；git 基线 personal | done |
| 2 | capability 索引重建 + @capability 覆盖 contract/kit | `zk capability --check`=0（39） | done |
| 3 | rustfmt 全仓 + 生成 glue rustfmt 稳定 | `cargo fmt --all -- --check`=0 | done |
| 4 | zk-bootstrap + 两端内核 boot/shutdown | `cargo test -p zk-bootstrap -p zk-desktop -p zk-server` | done |
| 5 | agent-rig 默认 + 企业 gateway rig 适配 + 占位模型 | `zk plan enterprise` agent.loop=agent-rig；compose+check=0 | done |
| 6 | rig 配置/usage/memory/hooks/cap-vector/能力诚实化 | 8 crate tests=0；docs/38 矩阵 | done |
| 7 | T22 schemars DTO + schema/TS/SCHEMAS + schema-check | `cargo test export_schema`=0；`schema-check.sh`=0 | done |
| 8 | 服务端 usage/幂等/审计/迁移/路由/auth/meta | real-PG 脚本=0；scoped clippy/tests=0 | done |
| 9 | web 真实 lint/typecheck/unit + C14 + a11y/i18n | 4 个 pnpm -r 门禁=0 | done |
| 10 | zk docs/channel/protected/roles/capability 结构校验 | `zk docs --check`=0；负向测试 exit 1 | done |
| 11 | release/deploy 去 false-green + channel-aware Dockerfile | PyYAML/bash -n；build-dist/upgrade-drill 退出码矩阵 | done |
| 12 | e2e 确定性（login/admin/agent/M1） | 4 套 e2e 全绿（login 4、admin 11、agent 9×2、M1 14/15 skip） | done |
| 13 | 文档/ADR（064/065/066）与实现状态回写 | `docs-check`/`spec-check`/`review-check`=0 | done |
| 14 | 全量门禁复核 + 提交 | fmt/clippy/test/zk/docs/web/schema 全绿 | done |
