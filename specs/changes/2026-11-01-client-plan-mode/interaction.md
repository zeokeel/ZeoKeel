# command-plan-mode · 交互契约

- 入口：composer `/plan`/`/plan off`；右栏 plan tab；turnTail 计划卡
- 归属：`@zeokeel/ui-chat`、`@zeokeel/ui-inspector`、ui-kit、desktop-web
- 权威：docs/19 §2.1b/§11、docs/27、docs/11 §8.6
- 状态 planned；证据根 target/evidence/2026-11-01-client-plan-mode/interaction/
- 测试 e2e/interaction/command-plan.spec.ts

- 状态：`planned`（实现开始时与 `intuition.md` 一起改 `active`）
## 1. 元素
| 元素 | role/name | 键盘 | 默认焦点 |
| --- | --- | --- | --- |
| `/plan` 命令 | option /「计划」 | ↑↓/Enter/Tab/Esc | 否 |
| plan tab | tab /「计划」 | ←/→/Enter | 否 |
| 计划卡 | region /「计划」+ 批准/拒绝/继续 | Tab/Enter | 否 |
| 计划 Markdown | text | 可读 | 否 |

## 2. 状态
empty/loading/error/offline/streaming/cancelled + plan-mode/draft/pending_review/approved/rejected/busy/disabled。plan mode 可见；pending 有动作；approved/rejected 只读；流式禁用提交/审核。

## 3. 条目
| ID | 前置 | 动作 | 期望 | 优先级 | 测试 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| CMD3P-PLAN-01 | 非流式 | `/plan`；描述任务；检查 mutating/只读/delegation（subagent/subagent_start/subagent_fork/agent_team/workflow/ralph）；`/plan off` | 进入可见状态；只读可用；mutating 0 执行；**delegation 各工具被阻断且 child mutating 0**；off 退出；刷新恢复 | P0 | command-plan.spec.ts | CMD3P-PLAN-01.json |
| CMD3P-PLAN-02 | plan mode | exit_plan_mode 提交；plan tab/turnTail 批准/拒绝/继续；刷新 | pending_review 显示计划；审核生效/持久化；批准不自动执行；拒绝保留反馈；超长/未知可读 | P0 | 同上 | CMD3P-PLAN-02.json |
| CMD3P-KEY-01 | composer/plan tab | 菜单/plan tab/审核按钮键盘；Esc | 与 Batch1 键位一致；焦点回 composer/卡 | P1 | 同上 | CMD3P-KEY-01.json |
| CMD3P-ERR-01 | 各态 | 空/错/离线/流式/超长/未知 | 可读/可重试/不白屏/不假成功；状态保留 | P1 | 同上 | CMD3P-ERR-01.json |
| CMD3P-VIS-01 | plan pending | light/1280 | 截图基线（计划 mask/0.002） | P2 | 同上 | — |

## 4. 视觉/ARIA
`e2e/interaction/__snapshots__/command-plan.spec.ts/plan-review-light-1280.png`；axe 0 serious。

## 5. 门禁
实现开始 active；`pnpm e2e:ui --grep command-plan` + gate。
