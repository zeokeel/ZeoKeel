# 2026-11-01-client-plan-mode · 验收

- 状态 draft（D3/D4/D6 已定 Lead 2026-11-01；待独立评审）
- 证据根 target/evidence/2026-11-01-client-plan-mode/
- 交互/直觉 planned→active；用户测试 UNVERIFIED。

## 1. 门禁
| 命令 | 结果 | 证据 |
| --- | --- | --- |
| spec-check/docs-check | 待跑 | logs |
| cargo test -p zk-contract-ipc -p zk-tool-plan -p zk-agent-rig -p zk-desktop | 待跑 | logs/cargo |
| export_schema 幂等/schema-check | 待跑 | logs/schema |
| zk validate/compose --check/capability | 待跑 | logs/zk |
| pnpm typecheck/lint/test:unit/build + i18n | 待跑 | logs/pnpm |
| pnpm e2e；e2e:ui --grep command-plan；gate | 待跑 | interaction/CMD3P-PLAN-01/02.json |

## 2. P0
| ID | 优先级 | 测试 | 证据 |
| --- | --- | --- | --- |
| CMD3P-PLAN-01 | P0 | e2e/interaction/command-plan.spec.ts | CMD3P-PLAN-01.json |
| CMD3P-PLAN-02 | P0 | 同上 | CMD3P-PLAN-02.json |
| CMD3P-KEY-01 | P1 | 同上 | CMD3P-KEY-01.json |
| CMD3P-ERR-01 | P1 | 同上 | CMD3P-ERR-01.json |
| CMD3P-VIS-01 | P2 | 同上 | — |

## 3. 手工
- [ ] `/plan`/`off` 真实命令/状态恢复；Batch1 不回归。
- [ ] plan mode mutating 0 次/只读可用；**delegation 六项（subagent/subagent_start/subagent_fork/agent_team/workflow/ralph）全部阻断，child mutating 0**；`subagent_fork` 专项测试；派生 `subagent*`/role + 静态兜底；exit_plan_mode 提交；非 plan 拒绝。
- [ ] 右栏 plan tab order50/turnTail 审核批准/拒绝/继续；批准不自动执行；刷新/切会话恢复。
- [ ] 64 KiB/错误/六态/i18n/axe/键盘；错误码登记；新 tool role/capability；docs/16 plan tab order 50；Lead 16 文件例外已记录。
- [ ] 无 sandbox/企业策略；用户测试 UNVERIFIED。

## 4. 数据/回滚
- [ ] `.zeokeel/plans.json` 损坏降级 off；无 DB migration；回滚删文件；计划仅本地。

## 5. 结论
draft；实现后填证据；未执行标 UNVERIFIED。
