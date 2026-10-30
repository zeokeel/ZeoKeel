# Design notes — assistant-ui design system, tint 250

M1 桌面 UI 视觉与实现说明。范围：`apps/desktop/web/**`，不改 Rust/bridge/契约。
本次采用 **assistant-ui 官方 design system**（“Every surface is a printed page”），只把中性色
hue 调到 `--tint: 250`（冷灰蓝）以显清新；不再使用暖米色、衬线与自绘 logo。

参考：<https://www.assistant-ui.com/design>、<https://www.assistant-ui.com/design.md>。

## 1. 视觉决策

- **一个中性色组**：所有灰阶都在 oklch `--tint: 250` 上，低 chroma；浅色为纸面、深色为深蓝灰。
- **唯一 live accent = blue-500**：流式光标、运行中/已选中/连接中状态才出现蓝色；没有 live 状态时
  页面无蓝。mock 徽标/普通按钮/选中行填充均用中性色。
- **destructive 是状态不是强调色**：错误/停止/危险操作用 destructive，不做第二强调色。
- **无背景渐变**：品牌渐变只存在于官方 logo 图片内；页面/卡片/按钮无渐变、无 glow、无纹理。
- **字重/字号建层级**：一个 sans 面（Public Sans）负责标题/正文/标签/数字；mono 只用于代码、
  命令、标识符（JetBrains Mono，连字关闭）。
- **线预算**：section 边界用 hairline（header/sidebar/footer）；列表行靠间距 + hover 填充，不用分隔线；
  不在同一条边叠 border + ring + shadow；阴影全局 0，只有 dialog/toast 等浮层 lift。

## 2. 字体（本地打包，无 CDN）

| 用途 | 字体 | 来源 |
| --- | --- | --- |
| UI / 标题 / 正文 / 标签 | Public Sans 400 / 500 / 600 | `@fontsource/public-sans` |
| 代码、命令、标识符 | JetBrains Mono 400 / 500，`font-variant-ligatures: none` | `@fontsource/jetbrains-mono` |
| 中文回退 | `PingFang SC` / `Noto Sans SC` / `system-ui` | 系统字体 |

字体 CSS 在 `src/main.tsx` 引入，woff/woff2 由 Vite 打进 `dist/assets/`。无外部字体请求。

## 3. 官方 logo（构建期注入，禁止手绘）

`zk compose` 从 `brands/official/` 生成：

- `apps/desktop/web/src/generated/brand.ts`（`@generated`）：`BRAND = {name, poweredBy, channel, logoLight, logoDark}`。
- `apps/desktop/web/public/brand/logo.svg`、`logo-dark.svg`。

UI 只用 `packages/ui/ui-kit/web/src/BrandLogo.tsx`：按当前解析后的主题选择 `logoLight`/`logoDark`，`<img>` 渲染。
使用位置：Sidebar、EmptyThread、启动 loading、启动 error、设置弹窗品牌位。仓库中不存在其它 logo
产物（`BrandMark.tsx` 已删除，无自绘 SVG、无“ZK”方块）。

## 4. 依赖版本（pnpm 解析）

| 包 | 版本 |
| --- | --- |
| @assistant-ui/react | 0.15.32 |
| @assistant-ui/react-markdown | 0.14.24 |
| tailwindcss / @tailwindcss/vite | 4.3.3 / 4.3.3 |
| @fontsource/public-sans / jetbrains-mono | 5.3.0 / 5.3.0 |
| react / react-dom | 19.3.1 |
| @radix-ui/react-dialog / tooltip | 1.3.1 / 1.4.1 |
| vite / @vitejs/plugin-react | 8.3.5 / 6.1.2 |
| typescript | 5.9.3 |
| vitest / jsdom | 5.0.5 / 30.1.3 |
| @testing-library/react / user-event | 16.3.3 / 14.6.8 |

Tailwind 4 通过 `@tailwindcss/vite` 接入；`src/app.css` 里 `@import "tailwindcss";` + `@theme inline`。

## 5. 包拆分、registry 与 UiProvider

UI 按“一切皆包”拆到 `packages/ui/*/web`；app 只保留基础设施与薄壳：

| 包 | npm | 角色 / 贡献 | 内容 | 依赖 |
| --- | --- | --- | --- | --- |
| ui-kit | `@zeokeel/ui-kit` | `ui.kit`（无 slot） | DTO/类型、`UiProvider/useUi`、`BrandLogo`、icons、Button/Switch/StatusDot/Chip | react |
| ui-sessions | `@zeokeel/ui-sessions` | `ui.slot.sessions` → `sessions.sidebar` | Sidebar（列表/新建/选中/设置入口/窄窗） | ui-kit |
| ui-chat | `@zeokeel/ui-chat` | `ui.slot.chat` → `chat.thread` | ChatView + assistant-ui Thread/Composer/ActionBar + 工具卡/审批卡/空态 + ExternalStoreRuntime adapter | ui-kit、`@assistant-ui/*` |
| ui-settings | `@zeokeel/ui-settings` | `ui.slot.settings` → `settings.dialog` | SettingsDialog + settings 表单校验 | ui-kit、`@radix-ui/react-dialog` |

依赖图（只向下）：

```text
apps/desktop/web (shell/infra)
  └─ generated/registry.ts (zk compose)
       ├─ @zeokeel/ui-sessions ─ @zeokeel/ui-kit
       ├─ @zeokeel/ui-chat ───── @zeokeel/ui-kit + @assistant-ui/*
       └─ @zeokeel/ui-settings ─ @zeokeel/ui-kit + @radix-ui/react-dialog
```

- **registry 使用**：`src/generated/registry.ts` 导出 `SessionsSlot` / `ChatSlot` / `SettingsSlot`（以及 `COMPONENTS`）。
  App shell **只**从这里取组件：`<SessionsSlot />`、`<ChatSlot />`、`<SettingsSlot />`。换包 = 改 edition/profile →
  `zk compose` → 重新 build；app 不 import 具体 UI 包（只依赖 registry）。
- **UiProvider（ui-kit）**：app 把 controller 的只读状态与动作、解析后的 theme、`BRAND`、translator 注入 `UiValue`；
  包只通过 `useUi()` 读取，**不 import app**。`UiValue` 含 t/locale/theme/brand/boot/meta/conversations/activeId/
  messages/messagesState/messagesError/streaming/offline/settings/settingsOpen/sidebarOpen/mock + actions
  （send/cancel/retry/create/select/openSettings/closeSettings/saveSettings/toggleSidebar/retryBoot/retryMessages/decideApproval）。
- **样式扫描**：Tailwind 4 的 `@source` 需要在 `src/app.css` 显式列出四个包目录（`packages/ui/*/web/src` 的 glob 在 Vite 下不生效）；包内只写 utility class，`zk-*` 结构样式仍由 app 的 `app.css` 提供。
- **已删除 app 内展示副本**：`apps/desktop/web/src/components/**`、`src/assistant/**`、`state/settings-validation.ts` 已迁出，app 不再保留第二份实现。
- **留在 app 的基础设施**：transport、protocol、controller、reducer、i18n 字典、theme resolve、App shell
  （providers、boot loading/error、header、offline banner、toast）；类型 DTO 在 ui-kit，app 通过 `src/types.ts` re-export。
- **边界**：包间不互相 import（仅允许 `@zeokeel/ui-kit`）；`@assistant-ui/*` 只出现在 ui-chat；`react`/`react-dom` 为
  peerDependency；品牌 logo 只在 ui-kit `BrandLogo` 里读 `UiValue.brand`。

### 5.1 Workspace 侧栏与高密度（M1）

- **结构（DSH 版）**：brand + 搜索/新建 → 全宽“新会话” → 命令面板/自动化/插件入口 → “工作区” section（搜索/筛选/添加）
  → **平铺 Workspace 行**（文件夹、默认标记、可折叠）+ **缩进会话行**（状态点/标题/时间，当前会话 live 蓝条）
  → 底部设置。没有独立“会话”section，也没有日期分组；Workspace 不嵌套。
- **数据**：boot 对每个 workspace 调用 `session.list`，`UiValue.conversationsByWorkspace` 保存所有工作区的会话；
  `conversations` 仍保留为当前工作区列表（兼容）。点击某会话会自动把 `activeWorkspace` 切到它所属 workspace。
- **命令**：`workspace.list/create/rename/set_default/remove`；`session.create/list/messages` 与 `chat.send` 全部携带
  `workspace_id`；`chat.send` 不再自动建会话（空工作区显示“新建会话”入口，发送前必须有 conversation_id）。
- **首用**：无 workspace 时整页首用（默认 `~` 或选择已存在目录）；创建后进入空工作区，不自动建会话。
- **密度 v2**：基础 13px/1.55；侧栏 248px；顶栏 44px；状态栏 24px（workspace 路径/连接/包数/版本）；消息列 760px；
  composer 2 行；工具调用默认单行折叠（展开看 args/result）；审批保持紧凑卡。
- **证据**：vitest 34 tests（含 workspace protocol/reducer/首用）；Playwright 9 tests（默认 `~`、workspace 隔离/切换、
  无 workspace 不建会话、移除二次确认+reassign、serve 真实 IPC）；axe 0 serious/critical；截图 01–09。

## 6. Design tokens（`src/app.css`）

### 5.1 中性色 / live / destructive

| token | Light | Dark | 说明 |
| --- | --- | --- | --- |
| `--zk-background` | `oklch(0.992 0.003 250)` | `oklch(0.17 0.008 250)` | 页面纸面 |
| `--zk-foreground` | `oklch(0.145 0.008 250)` | `oklch(0.96 0.004 250)` | 墨色文字 |
| `--zk-muted` | `oklch(0.968 0.005 250)` | `oklch(0.22 0.008 250)` | 静默填充/选中行/代码底 |
| `--zk-muted-foreground` | `oklch(0.5 0.01 250)` | `oklch(0.72 0.01 250)` | 次级文字（≥4.5:1） |
| `--zk-border` | `oklch(0.922 0.006 250)` | `oklch(0.28 0.008 250)` | 唯一 hairline |
| `--zk-card` / `--zk-popover` | `0.985/0.995` | `0.195/0.21` | 卡片/浮层表面 |
| `--zk-sidebar` | `oklch(0.978 0.004 250)` | `oklch(0.186 0.009 250)` | 侧栏纸面 |
| `--zk-blue-500` | `oklch(0.623 0.214 259.815)` | 同值 | **唯一 live accent** |
| `--zk-destructive` | `oklch(0.505 0.213 27.518)` | `oklch(0.704 0.191 22.216)` | 错误/停止状态 |
| `--zk-overlay` | fg 32% | fg 55% | 浮层遮罩 |
| `--zk-shadow-lift` | 10px 32px fg/10% | 12px 40px black/45% | 仅浮层 |

`--tint: 250` 记录在 `:root`；主题由 `<html data-theme="light|dark|auto">` 切换，`auto` 走
`prefers-color-scheme`。Tailwind utility 由 `@theme inline` 暴露：`bg-background`、`text-foreground`、
`bg-muted`、`text-muted-foreground`、`border-border`、`bg-live`、`text-destructive`、`bg-popover` 等。

### 5.2 圆角 / 阴影 / 动效

- 圆角按官方 scale：page 0、document 6、sm 6、control/md 8、surface 10、xl 12、thread 16、capsule 9999。
  实现为 `--radius-*`，使用 `rounded-document/control/surface/xl/thread/capsule`。
- 阴影：普通表面 `shadow: none`；只有 dialog/toast 使用 `--zk-shadow-lift`。
- 动效：hover/transition 150ms；侧栏 180ms；流式光标/typing dots 是唯一循环动画；全部在
  `prefers-reduced-motion` 下关闭。

## 7. assistant-ui 集成映射

`packages/ui/ui-chat/web/src/runtime.ts`：`useExternalStoreRuntime<ChatMessage>`，现有 `transport`/reducer 仍是唯一数据源。

| adapter | 映射 |
| --- | --- |
| `messages` | `state.messages`（`ChatMessage[]`） |
| `convertMessage` | text part；`toolCalls → tool-call`；`approvals → data-approval`；assistant status（running/complete/incomplete） |
| `onNew` | `AppendMessage` 取 text → `controller.send(text)` |
| `onCancel` | `controller.cancel()`（`chat.cancel`） |
| `onReload` | `controller.retrySend()`（重置最后 assistant 后重发最后 user） |
| `isRunning` / `isDisabled` / `isSendDisabled` | `state.streaming` / `state.offline` / `state.offline` |

使用的 primitives：`AssistantRuntimeProvider`、`ThreadPrimitive.Root/Viewport/ViewportFooter/Messages/Suggestion`、
`MessagePrimitive.Root/Content`、`ComposerPrimitive.Root/Input/Send/Cancel`、`ActionBarPrimitive.Root/Copy/Reload`、
`AuiIf`、`MarkdownTextPrimitive`（`@assistant-ui/react-markdown`，安全渲染，`smooth={false}`）。
工具卡 = `MessagePrimitive.Content` 的 `tools.Fallback`；审批 = `data.by_name.approval`；
侧栏会话列表自研（复用 `session.*`），未用 `ThreadList`/`BranchPicker`。

## 8. 组件规则

- **侧栏**（ui-sessions）：288px，`--zk-sidebar` 纸面，右侧一条 hairline；header = 官方 logo + `BRAND.name`（无 tagline）；
  New conversation 8px 圆角 hairline 按钮；会话行无分隔线，hover `foreground/3%` 填充；选中行
  `bg-muted` + 左侧 2px blue-500 指示（“已选中 = live”）；footer 12px 元信息；≤900px 抽屉（180ms）。
- **Header**：56px，仅标题 + 状态（idle 中性点 / streaming 蓝点）+ mock 中性徽标 + 设置；无大字/无渐变。
- **消息**：760px 居中；用户消息右对齐 `bg-muted` + `rounded-thread`(16)；助手无头像/无气泡/无左线，
  仅 12px 名称 + 内容；操作条（复制/重发）默认透明，hover/focus 填充出现；流式光标为 blue-500 闪烁。
- **工具卡**：hairline + `rounded-document`(6)，状态点（running blue / done 中性 / error destructive），
  mono args/result、可折叠、240 字符截断；无左侧色条。
- **Composer**：`rounded-thread`(16)、1px hairline、focus 时 blue ring；发送按钮 ink 面（非蓝），
  streaming 时换 destructive 描边 Stop；底部一行 12px 键盘提示 + 字数。
- **设置**（ui-settings）：Radix Dialog（`rounded-xl` 12 + lift），header 带官方 logo；分组为 `bg-muted` field panel（无边框），
  两栏栅格；标签 12px 句首大小写、正常字距；switch 为 capsule，开启时 blue-500；字段错误用 destructive。
- **空态**（ui-chat）：官方 logo + sans 32px/500 问句 + 3 个 `ThreadPrimitive.Suggestion` 建议（`bg-muted` 卡片，
  hover 填充，无阴影/无上浮）；无 tagline/口号（遵循官方 copy 规则）。

## 9. 六态映射

| 状态 | 呈现 |
| --- | --- |
| 空 | `thread.isEmpty` → logo + 32px 问句 + 建议卡；侧栏会话空态 |
| 加载 | boot spinner / 消息 skeleton（`bg-muted`）/ 设置保存中 |
| 错误 | boot 错误页 + 重试；消息加载错误 + 重试；assistant `incomplete/error` → destructive alert + ActionBar 重发 |
| 离线 | 顶部 destructive 文案横幅（中性底）；composer disabled |
| 流式中 | `thread.isRunning`：Stop、蓝点状态、blue-500 光标、typing dots |
| 已取消 | `incomplete/cancelled` → “已取消” 12px 文案，保留内容 |

## 10. 可访问性与验证

- 键盘：Enter 发送 / Shift+Enter 换行 / IME 不误发（assistant-ui Composer + 测试）；Esc 关设置；Tab 正常；
  focus-visible 2px blue-500。
- aria：输入/发送/停止/设置/侧栏切换/会话列表/错误 alert；`aria-current` 当前会话；`aria-busy` 加载；
  `aria-invalid` 字段；消息区 `role="log"`。
- 对比度：正文/次级/标签 ≥4.5:1；blue-500 仅用于非文字指示（点/光标/选中线）或大控件焦点环；
  `axe-core` 在空态/流式中/完成后/设置弹窗四态 **0 serious/critical**（`screenshots/axe.json`）。
- 命令：`pnpm build`（`tsc --noEmit` + vite build）、`pnpm test`（vitest + testing-library，31 例）。

## 11. 证据与截图

`apps/desktop/web/screenshots/`（`?mock=1`，vite preview + Playwright）：

| 文件 | 内容 |
| --- | --- |
| `01-empty.png` | 空态（light） |
| `02-streaming.png` | 流式中（blue 光标/Stop） |
| `03-tool-card.png` | 工具卡（完成态） |
| `04-settings.png` | 设置弹窗（light） |
| `05-light.png` | 显式 Light 主题 |
| `06-dark.png` | Abyss 深色主题 |
| `07-narrow.png` | 720px 窄窗（抽屉/汉堡） |
| `axe.json` | axe 四态结果（0 serious/critical） |

构建产物：`dist/index.html` + `assets/*`（相对路径）+ `brand/*` + 本地字体 woff/woff2；无外部 CDN。

## 12. 已知取舍 / 后续

- assistant-ui 使 bundle 增大（约 842 kB raw / 255 kB gzip，构建有 >500 kB 提示）；M1 未做路由级 code-split。
- 字体只打包 latin 子集 + 中文回退系统字体（中文无本地字体文件，符合“不引入外部字体/CDN”）。
- 未接 ESLint/jsx-a11y 与 Playwright e2e/CI（docs/18 门禁与 e2e 属后续任务）；axe 为本次手工审计。
- 审批仍是 `data-approval` 占位（后端尚未发 tool-approval 协议）；未使用 assistant-ui approval 机制。
- UI 已按包拆分（ui-kit/sessions/chat/settings），app 不再包含展示组件；registry 与 `zk.toml`/Cargo 未手改。
- 品牌资产为 `zk compose` 生成的 `src/generated/brand.ts` + `public/brand/*`，不手改。
