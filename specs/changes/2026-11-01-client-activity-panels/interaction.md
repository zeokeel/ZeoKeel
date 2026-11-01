# activity panels（Phase 2b 子 spec）· 交互契约

> 本文件是 `2026-11-01-client-activity-panels` 的交互契约（`planned`），不是长期规范。
> 长期规则见 `docs/19` §11、`docs/27` §2.5、`docs/11` §8.6；不要在这里复制定义。
> 交付要求：P0/P1 条目必须映射 `@interaction:<ID>` 测试与证据；没有测试/证据的条目不算完成。

- 表面 / 入口：右栏 activity tab（order 40）与 chat turnTail（Phase 2b 子 spec）
- 归属包：`@zeokeel/ui-inspector`（activity tab）+ `@zeokeel/ui-chat`（turnTail）+ `@zeokeel/ui-kit`（类型/actions）
- 权威文档：`docs/16` §2.2（tab order）、`docs/19` §2/§11、`docs/28` §4、`docs/11` §8.6
- 状态：`planned`（实现开始时改 `active`；planned 只 WARN，不参与门禁强制）
- 证据根：`target/evidence/2026-11-01-client-activity-panels/interaction/`
- 测试：`e2e/interaction/activity.spec.ts`（带 `@interaction:<ID>` 标签；实现时新建）

## 1. 元素与语义

| 元素 | role / accessible name | 键盘 | 默认焦点 | 备注 |
| --- | --- | --- | --- | --- |
| activity tab | `tab` /「活动」 | ←/→ 切 tab、Enter/Space | 否 | tablist 中 order 40；`aria-selected` |
| 刷新按钮 | `button` /「刷新活动」 | Enter/Space | 否 | 手动刷新 snapshot |
| jobs 行 / 停止 | `listitem` + `button` /「停止任务」 | Tab/Enter | 否 | 运行中才可停止；输出摘要可展开 |
| subagent 行 / 中断 | `listitem` + `button` /「中断子代理」 | Tab/Enter | 否 | 仅 running 可中断 |
| schedule 行/开关/编辑/删除 | `listitem` + `switch`/`button`（编辑/删除） | Tab/Enter/Space | 否 | 删除需内联二次确认；非法值行内错误 |
| todo 列表 | `list`/`listitem`（只读） | — | 否 | 来自工具行派生；不提供写操作 |
| turnTail 交付卡 | `button` /「在右栏查看」 | Enter/Space | 否 | present 路径去重；点击打开 files 预览 |
| turnTail 计划折叠 | `button` /「计划」 | Enter/Space | 否 | `<details>/<summary>` 语义；无匹配不渲染 |

## 2. 状态模型

状态：`empty` / `loading` / `error` / `offline` / `streaming` / `cancelled` + `busy` / `disabled` / `menu-open`（按适用）。

| 状态 | 进入条件 | 可见反馈 | aria（live/expanded/selected…） | 退出 |
| --- | --- | --- | --- | --- |
| loading | 首次打开/手动刷新 | 分区 loading | `aria-busy=true` | 请求完成 |
| error | snapshot/动作失败 | 分区错误 + 重试 | `role=alert` | 重试成功 |
| stale | 轮询失败但已有旧数据 | 旧数据 + stale 标记 | `role=status` | 下次成功 |
| empty | 无 jobs/subagent/goal/todo/schedule/present | 分区空态 | — | 有数据 |
| busy | 动作执行中（kill/interrupt/delete） | 按钮 disabled + spinner | `aria-busy=true` | 动作完成 |
| offline | 断网/本地 IPC 不可用 | 分区错误态 | 可读提示 | 恢复 |
| streaming | 会话流式中 | 轮询继续；动作按可用性 | — | turn 完成/取消 |
| menu-open | schedule 删除确认 | 内联确认可见 | 按钮 `aria-expanded` | 确认/取消 |

## 3. 交互条目（必须 1:1 映射测试）

| ID | 前置 | 人的动作（点击/按键/输入） | 期望（可见 + aria + 焦点 + 持久化） | 优先级 | 测试落点 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| ACT2-SNAPSHOT-01 | 右栏已打开、有 activity | 点击「活动」tab；读取各分区 | tab order 40（files→changes→terminal→activity）；snapshot 分区渲染 jobs/subagents/team/goal/todo/schedules/presentations；空分区显示空态 | P0 | `activity.spec.ts` | `ACT2-SNAPSHOT-01.json` |
| ACT2-POLL-01 | activity tab 打开 | 等待 2s；点手动刷新；切走/关闭 tab | 期间轮询更新；手动刷新拉新；切走/关闭停止轮询且不更新其它会话 | P0 | `activity.spec.ts` | `ACT2-POLL-01.json` |
| ACT2-JOB-STOP-01 | 有 running job | 展开输出；点击「停止任务」 | 输出摘要/展开可见；停止后状态刷新（completed/failed 或列表移除）；未知/已结束错误可读；不产生工具行/审批 | P0 | `activity.spec.ts` | `ACT2-JOB-STOP-01.json` |
| ACT2-SUBAGENT-INTERRUPT-01 | 当前会话有 running subagent | 点击「中断子代理」 | 中断后状态刷新为 cancelled/done；未知/已结束错误可读；roster 按 parent 过滤 | P0 | `activity.spec.ts` | `ACT2-SUBAGENT-INTERRUPT-01.json` |
| ACT2-SCHEDULE-EDIT-01 | 当前 workspace 有 schedule | 切换启用开关；编辑 interval/prompt 并保存 | 文件/列表同步；非法 interval(<60)/prompt>32 KiB 行内错误且不写盘；只操作当前 workspace | P0 | `activity.spec.ts` | `ACT2-SCHEDULE-EDIT-01.json` |
| ACT2-SCHEDULE-DELETE-01 | 有 schedule 行 | 点击删除 → 内联确认/取消 | 确认后删除并刷新；取消不变；未知 id/`deleted 0` 映射 `activity/schedule-invalid`；二次确认键盘可达 | P0 | `activity.spec.ts` | `ACT2-SCHEDULE-DELETE-01.json` |
| ACT2-TURNTAIL-PRESENT-01 | 本轮 assistant 有 present 工具结果 | 查看消息尾部；点击「在右栏查看」 | 交付卡显示文件名/描述、path 去重；点击打开右栏 files 预览；文件不存在可读错误；无空卡 | P0 | `activity.spec.ts` | `ACT2-TURNTAIL-PRESENT-01.json` |
| ACT2-TURNTAIL-PLAN-01 | assistant 文本含 `## 计划`/`## Plan` | 展开/折叠计划 | 只识别标题段落（中文/英文、`#`~`######`、冒号/行尾）；Markdown 渲染；无匹配不显示；不实现 plan mode | P1 | `activity.spec.ts` | `ACT2-TURNTAIL-PLAN-01.json` |
| ACT2-STALE-01 | 轮询失败但已有数据 | 等待下一次轮询 | 保留旧数据 + stale 提示；不反复弹错/不白屏 | P1 | `activity.spec.ts` | `ACT2-STALE-01.json` |
| ACT2-ERROR-01 | 各分区 | 触发空/加载/错误/离线/流式中/已取消 | 分区级状态可见、可重试/返回；不丢其它分区状态 | P1 | `activity.spec.ts` | `ACT2-ERROR-01.json` |
| ACT2-A11Y-01 | activity tab/turnTail 可见 | axe 扫描 + 键盘遍历 | 0 serious/critical；tab/动作/删除确认键盘可达；aria 正确；i18n 无硬编码 | P1 | `activity.spec.ts` | `ACT2-A11Y-01.json` |
| ACT2-VIS-01 | activity tab 默认态 | 渲染 | 截图基线（light/1280，禁动画，阈值 0.002；动态 job/subagent 项 mask） | P2（不进门禁） | `activity.spec.ts` | — |

- ID 规则：`<SURFACE>-<AREA>-<NN>`（本 surface 前缀 `ACT2-`；AREA 如 `SNAPSHOT`/`POLL`/`JOB`/`SUBAGENT`/`SCHEDULE`/`TURNTAIL`/`STALE`/`ERROR`/`A11Y`/`VIS`）。
- 优先级：P0 = 主路径/安全（session/workspace 归属、不产生工具行/审批）；P1 = 可达性/降级/回归基线；P2 = 可选增强（不进门禁）。

## 4. 视觉 / ARIA 基线

| ID | 状态 | 主题 | 宽度 | 基线类型 | 基线文件 | 掩码/阈值 |
| --- | --- | --- | --- | --- | --- | --- |
| ACT2-VIS-01 | activity 默认态 | light | 1280 | `toHaveScreenshot` | `e2e/interaction/__snapshots__/activity.spec.ts/activity-light-1280.png` | `maxDiffPixelRatio=0.002`；动态项 mask |
| ACT2-A11Y-01 | activity + turnTail | — | 1280 | `toMatchAriaSnapshot` + axe | `e2e/interaction/__snapshots__/activity.spec.ts/activity.aria.yml` | 动态 id/命令用正则；axe 0 serious/critical |

## 5. 预算与一致性

- 交互延迟：tab 切换 <100ms；轮询 2s；动作 busy 时禁用并显示状态；不得用 `waitForTimeout` 等反模式。
- 复用：只用 ui-kit 的 Button/Dialog/Listbox/Switch/Tooltip/Toast/Empty/Error；禁止第二套 focus/toast/dialog/menu（C17）。
- token：颜色/圆角/间距/动效只用 CSS vars；禁止 raw hex / magic px。
- i18n：所有文案来自字典（zh/en）；禁止硬编码中文/英文。
- 六态：每个分区必须覆盖；缺一即不通过评审。
- 视觉更新：只允许人确认后 `pnpm e2e:ui:update`；CI 不得自动接受。
- 门禁：实现开始时把本文件 `状态` 改 `active`，并把 P0/P1 标签/证据接入 `python3 scripts/ui-interaction-gate.py`。
- **工具依赖（P1-5）**：`pnpm e2e:ui` 与 `scripts/ui-interaction-gate.py` 由 `2026-11-01-ui-interaction-verification` 落地；本文件写作时两者尚不存在，当前 **UNVERIFIED**，实现前必须确认可用后再声称门禁绿。
