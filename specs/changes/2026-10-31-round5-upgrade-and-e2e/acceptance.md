# 2026-10-31-round5-upgrade-and-e2e · 验收

- 状态：`in-progress`
- 验收人：lead；C19 独立评审：待定

## 1. 验收结果

| 命令 | 通过条件 |
| --- | --- |
| cargo fmt/clippy/test --workspace | 0 |
| zk validate/compose personal --check/capability/docs/db lock/lint duplicates | 0 |
| pnpm -r typecheck/lint/test:unit/build | 0 |
| m1/agent/login/admin e2e + m1-serve usage/changes | 全绿 |
| docs/spec/review gates | 0 |
| C19 评审 | APPROVED |

## 2. UNVERIFIED

- 真实签名密钥/证书、Docker/多平台、`zk api` 路由真源、cargo-deny 本地、真实超大规模仓库 changes。
