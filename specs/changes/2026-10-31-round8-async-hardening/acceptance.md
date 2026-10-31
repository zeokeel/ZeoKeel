# 2026-10-31-round8-async-hardening · 验收

- 状态：`in-progress`
- 验收人：lead；C19 独立评审：auditor-build

## 1. 验收结果

| 命令 | 通过条件 |
| --- | --- |
| cargo fmt/clippy/test --workspace | 0 |
| zk/ compose/schema/docs/db lock/lint/matrix | 0 |
| pnpm gates | 0 |
| e2e agent/m1/login/admin/client-upgrade | 0 |
| async 压力/panic/竞态、request-id 测试 | 通过 |
| C19 评审 | APPROVED |

## 1.1 实现记录

- async：存活线程 RAII 计数、catch_unwind、CAS 状态机（interrupt 优先）、children/kit registry 64 有界；压力/panic/竞态测试。
- request-id：最外层 middleware 一次生成/回显，gate/bootstrap/updates/gateway settlement/audit 复用；真实 PG 验证。
- gateway：GatewayLlm 带 client header；但 `/v1/*` 的 426 门禁因 rig-core 原生 Responses 客户端不支持自定义头而**不启用**（docs/04 例外；JWT+发布锁定），列为后续上游依赖。
- matrix writer：docs/24 归属 scripts，CI/check.sh 增加 `--check`。

## 1.2 复评修复（P1 gate）

- 首评 P1：gateway 结算使用客户端 X-Request-Id，导致重复 header 只计费一次。修复：每次准入服务端生成 UUID 作为 wallet_holds/usage_pending/usage 键；客户端 id 仅关联/审计。
- `scripts/srv-auth-test.sh` 修复 round-6 缺失客户端头债务并加入断言：同一 `x-request-id` 两次调用余额 999850、无 usage 行使用 probe id、≥2 个 settlement id；真实 PG 全脚本 0。
- docs/04 §7 明确 request_id 非幂等/计费键；docs/38 matrix 措辞改为对照参考（非 SSOT）。

## 2. UNVERIFIED

- Tauri 真机/Windows、真实模型长时取消、Docker/多平台、cargo-deny 本地、真实私有 OEM。
