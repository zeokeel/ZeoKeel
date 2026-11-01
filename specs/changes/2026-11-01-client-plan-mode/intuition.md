# command-plan-mode · 直觉判定（INT）

> planned；证据根 target/evidence/2026-11-01-client-plan-mode/intuition/；用户测试 **UNVERIFIED**。

| ID | 原则 | 场景 | 期望 | 反例 | 验证 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| INT-01 | 可发现性 | `/` 菜单 | `/plan` 在「添加」可见/真实 | 藏/仍占位 | e2e+review | CMD3P-PLAN-01（interaction/CMD3P-PLAN-01.json）；W1 |
| INT-02 | 预期一致 | `/plan`/off | 进入/退出状态可见；不是普通消息 | 输入变消息/无状态 | e2e | CMD3P-PLAN-01 |
| INT-03 | 状态可见 | 计划提交/审核 | draft/pending/approved/rejected 可见；busy/错误可读 | 无状态/假成功 | e2e | CMD3P-PLAN-02 |
| INT-04 | 就近反馈 | 超长/未知 | 计划 tab/卡 inline 错误；不直出码 | 全局 toast/错误码 | e2e | CMD3P-ERR-01 |
| INT-05 | 默认安全 | plan mode | mutating 0 执行；**delegation 工具全部阻断（child 也 0 写）**；批准不自动执行；本地审核 | 经 subagent/subagent_start/subagent_fork/agent_team/workflow/ralph 写操作逃逸 | e2e+review | CMD3P-PLAN-01/02；W3 |
| INT-06 | 可恢复 | 拒绝/刷新 | 保留计划+反馈；刷新/切会话恢复 | 拒绝丢计划 | e2e | CMD3P-PLAN-02 |
| INT-07 | 认知负担 | plan tab | 一屏计划/状态/动作；不混 activity | 计划与活动混排 | review | W2 |
| INT-08 | 文案引导 | 空/待审 | 说明只读/批准不执行/拒绝反馈 | 误以为批准自动执行 | review+i18n | W4 |
| INT-09 | 键盘/焦点 | 菜单/plan tab/审核 | 键盘可达/焦点回 | 只能鼠标 | e2e | CMD3P-KEY-01 |
| INT-10 | 空/错误 | 无计划/离线 | 空态/错误可读不白屏 | 空白 | e2e | CMD3P-ERR-01 |
| INT-11 | 感知性能 | tab/提交 | 切换<100ms；提交 busy | 卡顿 | e2e/perf | W1 |
| INT-12 | 单一模式 | 全流程 | 复用 Batch1 菜单/side-panels tab/单一计划状态；无第二套 Dialog | 两套 plan 卡/tab | review+C17 | W4 |

## 覆盖
- CMD3P-PLAN-01→INT-01/02/05/11；CMD3P-PLAN-02→INT-03/05/06/07/08/12。
- P1 KEY→02/09；ERR→04/08/10；A11Y→09/10/12。
- 用户测试 UNVERIFIED；无 OPEN P0（walkthrough W1/W3 关闭）；planned。
