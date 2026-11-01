# workspace-new-session · 交互契约

> 本文件是 `2026-11-01-client-workspace-new-session` 的交互契约（`planned`），不是长期规范。
> 长期规则见 `docs/19` §11、`docs/27` §2.5/§2.6、`docs/11` §8.6；不要在这里复制定义。
> 交付要求：P0/P1 条目必须映射 `@interaction:<ID>` 测试与证据；没有测试/证据的条目不算完成。

- 表面 / 入口：会话侧栏 Workspace 行 scoped「新建会话」
- 归属包：`@zeokeel/ui-sessions`（+ `@zeokeel/ui-kit` action / `apps/desktop/web` controller/i18n）
- 权威文档：`docs/19` §2/§11、`docs/26`（C18）、`docs/27` §2.5–§2.8、`research/dsh-ui-modules/navigation.md` §1.1/§2.1
- 状态：`planned`（实现开始时与 `intuition.md` 一起改 `active`）
- 证据根：`target/evidence/2026-11-01-client-workspace-new-session/interaction/`
- 测试：`e2e/interaction/sessions-workspace-new.spec.ts`（`@interaction:<ID>` 标签；实现时新建）

## 1. 元素与语义

| 元素 | role / accessible name | 键盘 | 默认焦点 | 备注 |
| --- | --- | --- | --- | --- |
| Workspace 行 | `group` / workspace 标题 | Tab 到折叠/标题/新建/`⋯` | 否 | 现有行；点击标题选中 |
| scoped 新建按钮 | `button` /「在 <workspace> 新建会话」 | Enter/Space | 否 | hover + focus-visible 可见；active 行常显 |
| 全局新建 | `button` /「新建会话」 | Enter/Space | 否 | 非流式走 active workspace；streaming 禁用 + 同一提示 |

## 2. 状态模型

状态：`empty` / `loading` / `error` / `offline` / `streaming` / `cancelled` + `busy` / `disabled` / `hover` / `focus`。

| 状态 | 进入条件 | 可见反馈 | aria（live/expanded/selected…） | 退出 |
| --- | --- | --- | --- | --- |
| hover | 指针在 Workspace 行 | scoped 按钮出现 | — | 指针离开 |
| focus | 键盘 focus 行/按钮 | 按钮出现 + focus ring | `focus-visible` | Tab 离开 |
| active row | 当前 active workspace | 按钮常显 + 行高亮 | `aria-current`（既有） | 切 workspace |
| busy | 点击后请求中 | 按钮 disabled/`aria-busy` | `aria-busy=true` | 成功/失败 |
| streaming | 当前会话流式中 | scoped/global/⌘K 三入口禁用 + 同一 tooltip/aria/disabledReason「当前正在生成，停止后可新建」 | `aria-disabled=true` | turn 完成/取消 |
| empty | 无 Workspace | 无行/按钮；空态提示先建 workspace | 可读 | 有 workspace |
| error | 创建失败/不可用 | toast/行内可读错误；选择不变 | `role=alert`/toast | 重试/恢复 |
| offline | transport 不可用 | 创建失败可读 | 可读 | 恢复 |
| disabled | 无 workspace/不可用/streaming/busy | 按钮 disabled + title/aria 可读 | `aria-disabled` | 条件消失/停止 |
| cancelled | 当前流停止/失败 | 按钮恢复可用；无状态变化 | — | — |

## 3. 交互条目（必须 1:1 映射测试）

| ID | 前置 | 人的动作（点击/按键/输入） | 期望（可见 + aria + 焦点 + 持久化） | 优先级 | 测试落点 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| SES2-WSNEW-01 | 侧栏有 ≥2 个 Workspace；非流式 | hover 非 active 行 → 点 scoped `+`；再在另一行重复 | 按钮在 hover/focus 可见；点击在**该行 workspace** `session.create` 并选中新会话；active workspace/高亮切到目标；其它 workspace 会话不变；不经过 picker；不误建到原 active | P0 | `sessions-workspace-new.spec.ts` | `SES2-WSNEW-01.json` |
| SES2-WSNEW-02 | 侧栏有 Workspace | Tab 到行/按钮；Enter/Space；检查 active 行常显、非 active hover/focus、`aria-label` 含 workspace 名、行点击不触发 | 键盘可达；focus-visible 显示；active 行常显；按钮独立 focus；点击不触发整行选中/折叠；按钮名含 workspace；Enter/Space 创建且选中；无 hover 环境下 focus 可见 | P0 | 同上 | `SES2-WSNEW-02.json` |
| SES2-WSNEW-03 | 当前会话流式中；有多个 Workspace；全局按钮/⌘K 可见 | 在任意 workspace 行点/Enter `+`；点侧栏顶部/header `+`；打开 ⌘K 选 new-conversation；停止/取消流后再试 | 流式期间 **scoped/global/⌘K 三入口全部禁用**（按钮 `aria-disabled`/tooltip、⌘K disabledReason，同一文案「当前正在生成，停止后可新建」）；不建/不切/不串消息；停止后三入口恢复；非流式全局/⌘K 走 active workspace；无 workspace 时禁用/提示 | P1 | 同上 | `SES2-WSNEW-03.json` |
| SES2-WSNEW-VIS-01 | files? 侧栏 Workspace 行 | 渲染 light/1280 | 截图基线（hover 态可单独人工确认；阈值 0.002；会话行动态数据 mask） | P2（不进门禁） | 同上 | — |

- ID 规则：`<SURFACE>-<AREA>-<NN>`（本 surface 前缀 `SES2-WSNEW`；AREA 如 `ACT`/`KEY`/`VIS`）。
- 优先级：P0 = 主路径/数据一致性（建到正确 workspace/可访问）；P1 = 流式/全局一致性；P2 = 视觉增强。
- 一个 ID 至少一个测试；一个测试可覆盖多个 ID（标签可多个）；不得引用未定义 ID。

## 4. 视觉 / ARIA 基线

| ID | 状态 | 主题 | 宽度 | 基线类型 | 基线文件 | 掩码/阈值 |
| --- | --- | --- | --- | --- | --- | --- |
| SES2-WSNEW-VIS-01 | 侧栏 Workspace 行默认态 | light | 1280 | `toHaveScreenshot` | `e2e/interaction/__snapshots__/sessions-workspace-new.spec.ts/workspace-row-light-1280.png` | `maxDiffPixelRatio=0.002`；会话行动态数据 mask |
| SES2-WSNEW-VIS-01 | 行 hover/focus 态 | light | 1280 | `toHaveScreenshot` | 同目录 `workspace-row-hover-light-1280.png` | 人工确认；动态数据 mask；不自动接受 |
| SES2-WSNEW-02 | 侧栏 Workspace 行 | — | 1280 | `toMatchAriaSnapshot` + axe | `e2e/interaction/__snapshots__/sessions-workspace-new.spec.ts/workspace-row.aria.yml` | workspace 名用正则；axe 0 serious/critical |

## 5. 预算与一致性

- 交互延迟：hover 显示 ≤100ms；focus 立即；点击后 ≤100ms busy；创建/选中 <1s（本地 SQLite）。
- 复用：复用 ui-sessions 行样式/图标按钮；无新增 Dialog/确认流程（C17）。
- token：颜色/圆角/间距/动效只用 CSS vars；禁止 raw hex / magic px。
- i18n：所有文案来自字典（zh/en）；按钮名含 workspace 名；禁止硬编码。
- 六态：覆盖 empty/loading/error/offline/streaming/cancelled + busy/disabled/hover/focus。
- 视觉更新：只允许人确认后 `pnpm e2e:ui:update`；CI 不得自动接受。
- 门禁：实现开始时把本文件与 `intuition.md` 改 `active`，接入 `python3 scripts/ui-interaction-gate.py`。
- **门禁（工具已落地，`d0b7741`）**：实现/评审时运行 `pnpm e2e:ui --grep "workspace-new"` 与 `pnpm e2e:ui:gate`；未运行前本地证据标 UNVERIFIED。
