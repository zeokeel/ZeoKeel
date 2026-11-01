# 基线交互契约（已实现 surface）

> 本文件是 `2026-11-01-ui-interaction-verification` 的 `interaction.md`，为**当前已实现**的 5 个 surface 提供 `active` 基线契约。
> 长期规则见 `docs/19` §11、`docs/27` §2.5/§2.6、`docs/11` §8.6；不要在这里复制定义。
> 目标：让「交互 ID → 测试 → 证据」可执行、可回归、可评审；Phase 2a/2b 新 surface 各自 spec 另写 `interaction.md`（`planned` → `active`）。

- 状态：`active`
- 证据根：`target/evidence/2026-11-01-ui-interaction-verification/interaction/`
- 测试目录：`e2e/interaction/`
- 优先级口径：P0 = 主路径/焦点/数据一致性；P1 = 可达性/结构/视觉回归；P2 = 可选增强（不进门禁，本文件不列）。

## 0. 通用规则

- 定位：role / accessible name / label 优先；禁止 CSS 类选择器作为主定位。
- 键盘：用真实按键序列；IME 用 `compositionstart` + `isComposing` keydown 模拟（真实输入法为 UNVERIFIED）。
- 证据：测试末尾写 `<证据根>/<ID>.json`（`{id,title,at,passed}`）；只有通过的测试才会写。
- 标签：每个测试带 `{ tag: ["@interaction:<ID>"] }`；多个 ID 可写多个标签。
- 视觉：只对 composer 空态做最小截图（light/1280，禁动画，`maxDiffPixelRatio=0.002`）；其余以 aria snapshot + axe 为主。
- 禁止：`waitForTimeout`、`force: true`、无 `@known-issue` 的 `test.skip`。

## 1. composer（`@zeokeel/ui-chat`）

- 入口：会话页底部输入区（`AssistantComposer`）；输入框 `role=textbox`、aria-label「消息输入框」；发送「发送」/停止「停止」。
- 状态：empty / typing / sending / streaming / offline / error / cancelled（当前实现无 `/`、`@`、附件；Phase 2a 由 composer spec 补）。

| ID | 前置 | 人的动作 | 期望（可见 + aria + 焦点 + 持久化） | 优先级 | 测试落点 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| CMP-FOCUS-01 | 已有会话、输入框聚焦 | 输入文本后按 Enter 发送 | 消息发出；输入框清空且仍聚焦；发送按钮恢复可用 | P0 | `composer.spec.ts` | `CMP-FOCUS-01.json` |
| CMP-KEY-01 | 已有会话 | 输入「a」+Shift+Enter+「b」；再 Enter | 换行不发送（值为两行）；Enter 发送且不追加空行 | P0 | `composer.spec.ts` | `CMP-KEY-01.json` |
| CMP-IME-01 | 已有会话、输入框聚焦 | 输入中文后置 composing，按 Enter（`isComposing=true`） | 不发送；草稿保留；焦点不回退/不丢 | P0 | `composer.spec.ts` | `CMP-IME-01.json` |
| CMP-PERSIST-01 | 已发送一条带工具/usage 的消息 | 刷新页面 | 用户消息、助手回复、usage（输入/输出/合计）仍在；无重复消息 | P0 | `composer.spec.ts` | `CMP-PERSIST-01.json` |
| CMP-A11Y-01 | 已有会话 | 打开聊天页 | axe 0 serious/critical；输入框有可访问名称；发送/停止有名称 | P1 | `composer.spec.ts` | `CMP-A11Y-01.json` |
| CMP-VIS-01 | 新建空会话 | 渲染 composer | 空态截图与基线一致（light/1280；禁动画；阈值 0.002） | P1 | `composer.spec.ts` | `CMP-VIS-01.json` |

## 2. settings dialog（`@zeokeel/ui-settings`）

- 入口：顶栏「设置」按钮；`role=dialog`、可访问名称「设置」；分页导航按钮；底部唯一「保存」。
- 状态：closed / open / saving / saved / conflict / error。

| ID | 前置 | 人的动作 | 期望（可见 + aria + 焦点 + 持久化） | 优先级 | 测试落点 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| SET-OPEN-01 | 任意页面 | 点击「设置」 | dialog 可见且有可访问名称；焦点进入 dialog 内首个可聚焦控件；分页导航可见 | P0 | `settings.spec.ts` | `SET-OPEN-01.json` |
| SET-ESC-01 | 设置已打开 | 按 Esc | dialog 关闭；焦点回到「设置」按钮（精确 opener） | P0 | `settings.spec.ts` | `SET-ESC-01.json` |
| SET-ARIA-01 | 设置已打开 | 渲染 dialog 结构 | aria snapshot 含 dialog 名称、设置分区导航（`nav` + `aria-current` 唯一）、保存按钮；不假设 tablist/aria-selected | P1 | `settings.spec.ts` | `SET-ARIA-01.json` |
| SET-A11Y-01 | 设置已打开 | axe 扫描 | 0 serious/critical | P1 | `settings.spec.ts` | `SET-A11Y-01.json` |

## 3. inspector（`@zeokeel/ui-inspector`）

- 入口：`Ctrl+B` 或顶栏切换；`role=complementary`、可访问名称「右侧检查器」；tabs：文件/变更/终端；关闭按钮有名称。
- 状态：closed / open(files) / open(changes loading/error/empty) / open(terminal skeleton)。

| ID | 前置 | 人的动作 | 期望（可见 + aria + 焦点 + 持久化） | 优先级 | 测试落点 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| INS-TOGGLE-01 | 任意页面 | 按 Ctrl+B；再按 Ctrl+B | 第一次打开（complementary 可见），第二次关闭；不被输入框吞键 | P0 | `inspector.spec.ts` | `INS-TOGGLE-01.json` |
| INS-ARIA-01 | inspector 已打开 | 渲染 tablist | aria snapshot 含 tablist + 3 个 tab + 选中态唯一；面板 `role=tabpanel` | P1 | `inspector.spec.ts` | `INS-ARIA-01.json` |
| INS-A11Y-01 | inspector 已打开 | axe 扫描 | 0 serious/critical | P1 | `inspector.spec.ts` | `INS-A11Y-01.json` |

## 4. command palette（`@zeokeel/ui-commands`）

- 入口：`Ctrl+K`；`role=dialog`、可访问名称「命令面板」；搜索框 `role=textbox`、名称「搜索命令」；`role=listbox` + `role=option`。
- 状态：closed / open(empty query) / open(filtered) / no-match / disabled command。

| ID | 前置 | 人的动作 | 期望（可见 + aria + 焦点 + 持久化） | 优先级 | 测试落点 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| CMD-OPEN-01 | 输入框聚焦 | 按 Ctrl+K | dialog 可见；焦点在搜索框；列表可见 | P0 | `commands.spec.ts` | `CMD-OPEN-01.json` |
| CMD-NAV-01 | 命令面板已打开 | 按 ArrowDown | `aria-activedescendant` 指向当前 option；`[aria-selected=true]` 恰好 1 个；↑ 回退 | P0 | `commands.spec.ts` | `CMD-NAV-01.json` |
| CMD-ESC-01 | 命令面板已打开（从输入框打开） | 按 Esc | dialog 关闭；焦点回到打开前的输入框 | P0 | `commands.spec.ts` | `CMD-ESC-01.json` |
| CMD-A11Y-01 | 命令面板已打开 | axe 扫描 | 0 serious/critical；dialog/listbox/option 语义正确 | P1 | `commands.spec.ts` | `CMD-A11Y-01.json` |

## 5. sessions sidebar（`@zeokeel/ui-sessions`）

- 入口：左侧导航；「新建会话」按钮；会话行；搜索框。
- 状态：empty / list / selected / search-no-match / sidebar-collapsed。

| ID | 前置 | 人的动作 | 期望（可见 + aria + 焦点 + 持久化） | 优先级 | 测试落点 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| SES-NEW-01 | 任意页面 | 点击「新建会话」 | 新会话被选中；输入框可见且可聚焦；侧栏出现对应会话 | P0 | `sessions.spec.ts` | `SES-NEW-01.json` |
| SES-A11Y-01 | 侧栏可见 | axe 扫描 | 0 serious/critical；导航/按钮有可访问名称 | P1 | `sessions.spec.ts` | `SES-A11Y-01.json` |

## 6. 已知边界（本基线不做 e2e）

- 流式中/停止/取消、离线、错误注入：现有 `apps/desktop/web/tests/composer-keyboard.test.tsx` 等 L1 测试 + reducer golden traces 覆盖；本基线只固化可稳定执行的真人旅程。
- 企业版登录/网关/商店/管理台 surface：后续按同一模板补契约。
- 真实输入法、像素级跨平台视觉：见 `KNOWN-LIMITATIONS.md` §3.4。
