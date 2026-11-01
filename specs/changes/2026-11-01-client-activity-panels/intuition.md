# activity-panels（Phase 2b 子 spec）· 直觉判定（INT）

> 本文件与 `interaction.md` 同目录（第七件，`docs/19` §11.1、`docs/27` §2.7/§2.8、`docs/11` §8.6）。
> 只写本 surface 的直觉判据与映射；原则表/走查方法不在本文件复制。
> 状态：`planned`（实现开始时**与 `interaction.md` 同时**改 `active`；planned 只 WARN，不参与门禁强制）。
> 证据根：`target/evidence/2026-11-01-client-activity-panels/intuition/`；走查见 `intuition-walkthrough.md`。
> 说明：`e2e proxy` 只证明可观察行为，不证明用户觉得直观；最终需 fresh-context 评审 + ≥5 人用户测试（当前 **UNVERIFIED**）。

| ID | 原则 | 场景 / top task | 用户直觉期望（可观察） | 反例 | 验证方式 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| INT-01 | 可发现性 | T1 查看活动/停止任务（P1） | 右栏 tab 中「活动」名称直白、在 files/changes/terminal 之后可见；刷新按钮常驻；turnTail「在右栏查看」卡片明显 | 活动入口藏在菜单；刷新 hover-only；present 只在原文里 | e2e proxy + review | ACT2-SNAPSHOT-01（target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-SNAPSHOT-01.json；e2e/interaction/activity.spec.ts）、ACT2-TURNTAIL-PRESENT-01（target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-TURNTAIL-PRESENT-01.json；e2e/interaction/activity.spec.ts）、review: walkthrough W1/W6/W10 |
| INT-02 | 预期一致 | T2 交互（P2） | tab 用 ←/→；schedule 开关用 Space；删除/停止/中断确认用 Esc 取消、Enter 确认；与既有右栏 tab/菜单一致 | 删除直接生效无确认；Esc 关整个右栏；开关需要双击 | e2e proxy | ACT2-SCHEDULE-DELETE-01（target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-SCHEDULE-DELETE-01.json；e2e/interaction/activity.spec.ts）、ACT2-POLL-01（target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-POLL-01.json；e2e/interaction/activity.spec.ts）、review: walkthrough W3/W4/W9 |
| INT-03 | 状态可见 | T1/T2 刷新/动作（P1/P3） | 打开/刷新显示 loading；job 停止、subagent 中断、schedule 保存 100ms 内 busy；完成后列表/状态可见更新 | 点击无反馈、重复点、状态不动 | e2e proxy + review | ACT2-SNAPSHOT-01（target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-SNAPSHOT-01.json；e2e/interaction/activity.spec.ts）、ACT2-JOB-STOP-01（target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-JOB-STOP-01.json；e2e/interaction/activity.spec.ts）、ACT2-SCHEDULE-EDIT-01（target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-SCHEDULE-EDIT-01.json；e2e/interaction/activity.spec.ts） |
| INT-04 | 就近反馈 | T2 schedule 编辑/删除（P1/P3） | interval/prompt 非法在行内提示；未知 id/`deleted 0` 显示「任务已不存在/已结束」；不弹无上下文 toast | 只显示「失败」；错误后列表不变也不说明 | e2e proxy + review | ACT2-SCHEDULE-EDIT-01（target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-SCHEDULE-EDIT-01.json；e2e/interaction/activity.spec.ts）、ACT2-SCHEDULE-DELETE-01（target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-SCHEDULE-DELETE-01.json；e2e/interaction/activity.spec.ts） |
| INT-05 | 默认安全 | T1/T2 破坏性动作（P1/P2） | 停止 job/中断 subagent/删除 schedule 都有内联二次确认（显示对象名/影响，默认焦点「取消」，`role=alertdialog`）；仅 running 可操作；不产生工具行/审批 | 一点就停/删；确认框默认焦点在破坏按钮；对已结束对象反复操作 | e2e proxy + review | ACT2-JOB-STOP-01（target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-JOB-STOP-01.json；e2e/interaction/activity.spec.ts；job-stop-confirm case）、ACT2-SUBAGENT-INTERRUPT-01（target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-SUBAGENT-INTERRUPT-01.json；e2e/interaction/activity.spec.ts；subagent-interrupt-confirm case）、ACT2-SCHEDULE-DELETE-01（target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-SCHEDULE-DELETE-01.json；e2e/interaction/activity.spec.ts）、review: walkthrough W2/W3/W4 |
| INT-06 | 可恢复 | T2/T3 失败/切走（P2/P3） | 轮询失败保留旧数据 + stale；动作失败保留旧快照可重试；切会话/关闭立即停轮询；turnTail 预览失败可返回消息 | 一次失败清空面板；切走后旧响应覆盖新会话；预览失败卡死 | e2e proxy | ACT2-STALE-01（target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-STALE-01.json；e2e/interaction/activity.spec.ts）、ACT2-ERROR-01（target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-ERROR-01.json；e2e/interaction/activity.spec.ts）、ACT2-TURNTAIL-PRESENT-01（target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-TURNTAIL-PRESENT-01.json；e2e/interaction/activity.spec.ts） |
| INT-07 | 认知负担 | T1 总览（P1/P2） | 分区（jobs/subagents/team/goal/todo/schedule/present）有标题与计数；长输出折叠/截断；不把 7 区平铺成长日志 | 所有工具行混在一起；一次展示完整 stdout；无分组 | review（Hick/Miller） | ACT2-SNAPSHOT-01（target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-SNAPSHOT-01.json；e2e/interaction/activity.spec.ts）、review: walkthrough W1/W10 |
| INT-08 | 文案即引导 | T1/T2 空/错误/降级（P1/P3） | 空分区给「暂无运行中任务」；错误给原因+重试；不直出 `activity/*` 错误码；无 plan 显示无计划卡 | 空白区；`activity/schedule-invalid` 直出 | review + i18n | review: walkthrough W5/W6/W8、i18n-check |
| INT-09 | 键盘/焦点 | T2/T3 键盘/读屏（P2/P3） | tab/刷新/停止/中断/开关/删除确认键盘可达；删除取消后焦点回删除按钮；aria-live 播报动作结果 | 只能鼠标；Esc 后焦点丢失；确认框无 role | e2e proxy | ACT2-SCHEDULE-DELETE-01（target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-SCHEDULE-DELETE-01.json；e2e/interaction/activity.spec.ts）、ACT2-A11Y-01（target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-A11Y-01.json；e2e/interaction/activity.spec.ts） |
| INT-10 | 空态/错误态 | T1/T3 六态（P3） | 空/加载/错误/离线/流式中/已取消在各分区有对应提示，可重试/返回，不白屏、不假成功 | 白屏；错误静默；流式中分区消失 | e2e proxy | ACT2-ERROR-01（target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-ERROR-01.json；e2e/interaction/activity.spec.ts）、ACT2-STALE-01（target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-STALE-01.json；e2e/interaction/activity.spec.ts）、ACT2-SNAPSHOT-01（target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-SNAPSHOT-01.json；e2e/interaction/activity.spec.ts） |
| INT-11 | 感知性能 | T1 切 tab/刷新（P2） | 切到 activity ≤100ms 显示骨架；2s 轮询不打断阅读；动作 busy 不闪烁 | 切 tab 卡 1s；轮询清空列表；操作后整页闪白 | e2e/perf + review | ACT2-POLL-01（target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-POLL-01.json；e2e/interaction/activity.spec.ts）、review: walkthrough W7 |
| INT-12 | 单一模式 | T1/T2 所有动作（P1/P2） | 只经 `activity.snapshot`/`activity.action` 一条数据/动作路径；复用 ui-kit tab/list/switch/dialog，无第二套轮询/确认/焦点 | 前端直接读文件/工具；两套删除确认；两套轮询 | review + C17 | review: walkthrough W6/W9 |

## 覆盖说明

- P0 交互 ID → INT 映射（每个 P0 至少 1 条）：
  - `ACT2-SNAPSHOT-01` → INT-01/INT-03/INT-07/INT-10/INT-12；
  - `ACT2-POLL-01` → INT-02/INT-06/INT-11；
  - `ACT2-JOB-STOP-01` → INT-03/INT-05/INT-06；
  - `ACT2-SUBAGENT-INTERRUPT-01` → INT-03/INT-05/INT-06；
  - `ACT2-SCHEDULE-EDIT-01` → INT-03/INT-04/INT-05；
  - `ACT2-SCHEDULE-DELETE-01` → INT-02/INT-04/INT-05/INT-09；
  - `ACT2-TURNTAIL-PRESENT-01` → INT-01/INT-06/INT-10。
- 用户测试（≥5 人 think-aloud）：**UNVERIFIED**（未执行；发布前按 `intuition-walkthrough.md` §4 执行；未执行前**任何 acceptance/发布文案不得声称“符合直觉/直观”**）。
- 未解决 P0 摩擦：**无 P0 摩擦**（见 `intuition-walkthrough.md` §3；F1/F2/F3 已按 design/tasks 最小修法关闭）。
- 状态：`planned`；实现开始时**与 `interaction.md` 同时**改 `active` 并补对应 e2e 证据/走查复核。
