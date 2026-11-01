# command-surface（Batch 1）· 交互契约

> 本文件是 `2026-11-01-client-command-surface` 的交互契约（`planned`），不是长期规范。
> 长期规则见 `docs/19` §11、`docs/27` §2.5/§2.6、`docs/11` §8.6；不要在这里复制定义。
> 交付要求：P0/P1 条目必须映射 `@interaction:<ID>` 测试与证据；没有测试/证据的条目不算完成。

- 表面 / 入口：composer `/` 命令菜单 + `/model`/`/permission` popup + `/goal` GoalBar + `/export`/`/feedback` 结果面
- 归属包：`@zeokeel/ui-chat`（菜单/GoalBar/FeedbackDialog）、`@zeokeel/ui-kit`（命令 DTO/Modal/actions）、`apps/desktop/web`（唯一 catalog/actions）
- 权威文档：`docs/19` §2/§11、`docs/27` §2.5–§2.8、`docs/11` §8.6、`target/audits/dsh-reference-inventory.md` §5
- 状态：`planned`（实现开始时与 `intuition.md` 一起改 `active`）
- 证据根：`target/evidence/2026-11-01-client-command-surface/interaction/`
- 测试：`e2e/interaction/command-surface.spec.ts`（`@interaction:<ID>` 标签；实现时新建）

## 1. 元素与语义

| 元素 | role / accessible name | 键盘 | 默认焦点 | 备注 |
| --- | --- | --- | --- | --- |
| composer 输入框 | `textbox` /「消息输入框」 | `/`（词首）、Enter/Shift+Enter、Esc | 是 | 既有 composer；命令菜单不抢焦点 |
| `/` 命令菜单 | `listbox` /「命令」+ `option` | ↑/↓、Enter/Tab、Esc/Shift+Tab | 否 | 分组「添加/指令/技能」；`aria-activedescendant`/`aria-expanded` |
| Batch2 占位 | `option` `aria-disabled=true` | 不可 Enter 执行；↑↓ 可越过 | 否 | 文案「未实现（Batch 2）」；不发送 |
| model/permission popup | `listbox` /「模型」/「权限」 | ↑/↓、Enter/Tab、Esc | 否 | 初始高亮=当前值；单选 |
| GoalBar | `region` /「会话目标」+ 暂停/清除按钮 | Tab/Enter | 否 | composer 上方；无 goal 不渲染 |
| FeedbackDialog | `dialog` /「发送反馈」+ 分类 radio/文本域/提交/取消 | Tab、Esc | 分类 | 复用 ui-kit Modal；失败 inline |
| export 结果 | `status` /「导出」 | — | 否 | 成功显示路径/下载；失败 `role=alert` |

## 2. 状态模型

状态：`empty` / `loading` / `error` / `offline` / `streaming` / `cancelled` + `busy` / `disabled` / `composing` / `menu-open` / `unimplemented`。

| 状态 | 进入条件 | 可见反馈 | aria（live/expanded/selected…） | 退出 |
| --- | --- | --- | --- | --- |
| menu-open | 词首 `/` | 分组菜单 | `aria-expanded=true` + activedescendant | 接受/Esc/点击外 |
| composing | IME composition | 菜单不打开/不触发 | `aria-expanded=false` | compositionend |
| empty | 无匹配 | 「无匹配命令」+ 保留草稿 | 可读 | 继续输入 |
| unimplemented | Batch2 占位 | disabled + 「未实现（Batch 2）」 | `aria-disabled=true` | 移动/Esc |
| busy | 提交 override/export/feedback/goal | 按钮 busy/禁用重复 | `aria-busy=true` | 完成/失败 |
| error | 未知命令/后端失败/超限 | 行内错误/`role=alert`，草稿保留 | `role=alert` | 重试/修改 |
| offline | transport 不可用 | 命令可打开但后端动作显示离线错误 | 可读 | 恢复 |
| streaming | 流式中 | 按 R2 禁用一部分命令 | disabled | turn 完成/取消 |
| cancelled | turn 取消后 | 菜单可重开；无残留 busy | — | — |

## 3. 交互条目（必须 1:1 映射测试）

| ID | 前置 | 人的动作（点击/按键/输入） | 期望（可见 + aria + 焦点 + 持久化） | 优先级 | 测试落点 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| CMD2-MENU-01 | composer 聚焦、非 IME | 词首输入 `/`；输入过滤；↑/↓；Enter/Tab；Esc/Shift+Tab | 菜单分组=添加(file,goal,plan,feedback)/指令(compact,permission,model,export)/技能(skill)；过滤大小写不敏感；接受后移除 `/query`、焦点回输入框；Esc/Shift+Tab 关闭且草稿保留；**无 UI 动作** | P0 | `command-surface.spec.ts` | `CMD2-MENU-01.json` |
| CMD2-MENU-02 | 菜单打开 | 输入 `/nope`；选择 Batch2 `/compact`/`/plan`/`/skill`；尝试 Enter | 未知命令不发送、显示可读错误、草稿保留；Batch2 显示「未实现」disabled、不可执行、不产生消息/IPC；`command/unknown`/`command/unimplemented` | P0 | 同上 | `CMD2-MENU-02.json` |
| CMD2-FILE-01 | 菜单打开 | 选择 `/file`；选择/取消文件；带附件选择其它命令 | 打开文件选择器，成功 chip、取消保留草稿、≤5/≤1 MiB 复用；其它命令带附件拒绝并保留 chip | P0 | 同上 | `CMD2-FILE-01.json` |
| CMD2-MODEL-01 | 有会话、personal/enterprise | `/model`；查看当前值；personal 手动输入/选 recent；enterprise 选 gatewayModels；选 effort；切会话；重启 | personal popup = 当前模型 + 手动输入 + recentModels(≤8)+effort（不声称 provider 列表）；enterprise 只 host 缓存 gatewayModels；当前值高亮；选择只影响当前会话后续轮次；**全局 config-mutex 下 turn 前应用/turn 后恢复，其它会话 turn 排队；turn 中 settings.set 排队到结束后按 revision 恢复/应用；排队 turn 被取消则获锁后跳过不执行**；切会话/重启恢复；失败回滚并提示；不写全局 settings | P0 | 同上 | `CMD2-MODEL-01.json` |
| CMD2-PERM-01 | 有会话 | `/permission`；选择三档；切会话；重启 | 三档显示审批映射与「仅审批、无沙箱、不限制员工」；当前值高亮；会话级生效/恢复；失败回滚；不改全局 | P0 | 同上 | `CMD2-PERM-01.json` |
| CMD2-GOAL-01 | 有会话 | `/goal <objective>`→确认；`/goal`；GoalBar 暂停/恢复；`/goal clear`→确认 | 设置后 GoalBar 显示 objective/status/rounds；查看不发送；暂停/恢复/清除生效；同一 `.zeokeel/goal.json`；空/超限/无 goal 可读错误；不产生工具行 | P0 | 同上 | `CMD2-GOAL-01.json` |
| CMD2-EXPORT-01 | 有会话/空会话 | `/export`；检查文件/路径；模拟超限/写失败 | 成功生成 Markdown（消息/工具摘要/usage，≤8 MiB，文件名 sanitize）；serve 下载；Tauri 展示/复制路径（不承诺自动打开，`system.open_url` 仅 http(s)）；空会话说明；失败可读；仅当前 workspace | P0 | 同上 | `CMD2-EXPORT-01.json` |
| CMD2-FEEDBACK-01 | 任意会话 | `/feedback`→分类+描述→提交；取消；超 4 KiB；企业版 | 弹窗可读；本地保存 JSON 并明示「本地保存」；取消不写；超限 inline；企业版不显示已发送/标 UNVERIFIED；失败可重试；无网络 | P0 | 同上 | `CMD2-FEEDBACK-01.json` |
| CMD2-KEY-01 | composer | `/` 后 ↑↓/Enter/Tab/Esc/Shift+Tab；IME composition 中 Enter/`/` | 键位与 DSH 一致；菜单打开时焦点留输入框；关闭回输入框；IME 不触发/不误发；正文中间 `/` 不开菜单 | P1 | 同上 | `CMD2-KEY-01.json` |
| CMD2-ERR-01 | 菜单/popup/GoalBar/dialog | 触发空/加载/错误/离线/流式中/已取消；未知命令；后端失败 | 分区/组件级六态可见、可重试/返回、草稿/旧值保留、不白屏、不假成功 | P1 | 同上 | `CMD2-ERR-01.json` |
| CMD2-A11Y-01 | 命令面可见 | axe + 键盘遍历 | 0 serious/critical；listbox/dialog/GoalBar aria 正确；焦点可见并回 composer；i18n 无硬编码 | P1 | 同上 | `CMD2-A11Y-01.json` |
| CMD2-VIS-01 | `/` 空查询 light/1280 | 渲染 | 截图基线（禁动画，阈值 0.002；动态最近命令项 mask） | P2（不进门禁） | 同上 | — |

- ID 规则：`<SURFACE>-<AREA>-<NN>`（`CMD2` 前缀；AREA = MENU/FILE/MODEL/PERM/GOAL/EXPORT/FEEDBACK/KEY/ERR/A11Y/VIS）。
- 优先级：P0 = 主路径/安全/数据一致性；P1 = 高频细节/可达性/回归；P2 = 可选增强（不进门禁）。

## 4. 视觉 / ARIA 基线

| ID | 状态 | 主题 | 宽度 | 基线类型 | 基线文件 | 掩码/阈值 |
| --- | --- | --- | --- | --- | --- | --- |
| CMD2-VIS-01 | `/` 空查询 | light | 1280 | `toHaveScreenshot` | `e2e/interaction/__snapshots__/command-surface.spec.ts/command-menu-light-1280.png` | `maxDiffPixelRatio=0.002`；最近命令 mask |
| CMD2-A11Y-01 | 菜单+dialog+GoalBar | — | 1280 | `toMatchAriaSnapshot` + axe | `e2e/interaction/__snapshots__/command-surface.spec.ts/command-surface.aria.yml` | 动态模型名/时间用正则；axe 0 serious/critical |

## 5. 预算与一致性

- 交互延迟：`/` 打开 <100ms；过滤每键 <16ms；popup 切换 <100ms；goal/export/feedback busy 可见。
- 复用：只用 ui-kit Button/Modal/listbox/Toast/Empty/Error；不新增第二套 focus/menu/Dialog/执行器（C17）。
- token：颜色/圆角/间距/动效只用 CSS vars；禁止 raw hex / magic px。
- i18n：所有文案来自字典（zh/en）；禁止硬编码中文/英文；错误码不直出。
- 六态：每个命令/组件必须覆盖；缺一不通过评审。
- 视觉更新：只允许人确认后 `pnpm e2e:ui:update`；CI 不得自动接受。
- 门禁：实现开始时把本文件与 `intuition.md` 改 `active`，接入 `python3 scripts/ui-interaction-gate.py`。
- **工具依赖（沿用前序）**：`pnpm e2e:ui` / `scripts/ui-interaction-gate.py` 由 `2026-11-01-ui-interaction-verification` 落地；当前 **UNVERIFIED**，实现前确认命令存在后再声称门禁绿。
