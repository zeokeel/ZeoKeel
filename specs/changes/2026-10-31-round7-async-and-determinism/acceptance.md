# 2026-10-31-round7-async-and-determinism · 验收

- 状态：`done`（C19 独立复评 APPROVED at HEAD 25d1712）
- 验收人：lead；C19 独立评审：auditor-build（非实现者）

## 1. 验收结果

| 命令 | 通过条件 |
| --- | --- |
| cargo fmt/clippy/test --workspace | 0 |
| zk validate/compose personal --check/capability/docs/db lock/lint duplicates | 0 |
| pnpm -r typecheck/lint/test:unit/build | 0 |
| m1/agent/login/admin/client-upgrade e2e | 全绿 |
| 异步子代理/确定性/426/open_url 测试 | 通过 |
| docs/spec/review gates | 0 |
| 异步子代理 | agent-rig 24 lib + tools tests：start→running→done、interrupt→cancelled、并发上限 8、深度限制 |
| 生成物确定性 | stable_json 单测；多包构建后的 target/debug/zk compose/db lock --check 0 |
| 426/审计/open_url | bootstrap/updates nested 426 + request id echo；audit_log.request_id 实 PG 验证；stdout spawn/reap 测试 |
| C19 评审 | APPROVED（auditor-build 独立；P0/P1=0；P2 见 review §4） | docs/reviews/2026-10-31-round7-verification.md |

## 1.1 实现记录

- 异步子代理：kit `start/status/result/interrupt` + `AsyncAgentStatus`；agent-rig 后台线程、共享 Arc 取消、并发 8、状态/结果；工具 `subagent_start/subagent_status/subagent_interrupt`、list_agents 同步+异步；`extra_tools` 生成约定；同步 subagent/fork 兼容。
- P2-N1：`stable_json` 递归排序生成物 key；packages/roles/zk.lock 重生成；多包构建后 target/debug/zk compose/db lock 检查 0。
- 426：kit 共享 helper（request id、retryable、current/minimum/portal）用于业务 gate/bootstrap/updates manifest；audit_log.request_id 落库（实 PG 验证）；open_url 后台 reap。
- web：UpgradeGate await system.open_url + 失败 role=status；426 flat/nested 兼容与 request_id 展示。

## 1.2 复评与残留

- 独立复跑：workspace tests 403 passed/418 blocks；P2-N1 多包构建后 target/debug/zk compose/db lock/capability 全 0；e2e updates/agent/m1/admin/login/client-upgrade 全 0；真实 PG 三处 426 nested + audit request_id 落库验证。
- 非阻塞 P2（review §4，下一轮优先 P2-1/3）：并发上限可被 interrupt 绕过（只数 running）；线程 panic 无防护/children 只增不减；interrupt 与 completion 竞态；extra_tools 隐式生成约定未在 docs/24 writer 表声明；审计 hash 不含 request_id/actor；gateway 自生成 request_id 与 HTTP 不一致；gate 生成 A 而 bootstrap 生成 B 的 request-id 不一致；matrix writer SSOT 冲突且 CI 无 matrix --check；async 子代理无真实 turn e2e。

## 2. UNVERIFIED

- Tauri 真机、Windows 运行时、真实媒体 provider、Docker/多平台、cargo-deny 本地、真实私有 OEM 索引。
