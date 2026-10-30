# 2026-10-31-round4-data-and-ui · 验收

- 状态：`in-progress`
- 验收人：lead；C19：auditor-rig（或独立非实现者）

## 1. 待填

| 命令 | 通过条件 |
| --- | --- |
| cargo fmt/clippy/test --workspace | 0 |
| export_schema/schema-check/generated clean | 0 |
| zk validate/compose personal --check/capability/docs/db lock/lint duplicates | 0 |
| pnpm -r typecheck/lint/test:unit/build | 0 |
| m1/agent/login/admin e2e + workspace.changes 场景 | 全绿 |
| updates/oem/portal e2e | 0 或明确 UNVERIFIED |
| usage/settings/changes 单测 | 通过 |

## 2. UNVERIFIED

- Docker/多平台/SBOM；真实媒体 provider；cargo-deny 本地。
