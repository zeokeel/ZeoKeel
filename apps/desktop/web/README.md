# ZeoKeel 桌面 UI（M1）

React 19 + TypeScript + Vite + Tailwind 4 + **assistant-ui** 的桌面聊天界面，运行在 Tauri webview
（生产）或本地 HTTP `serve` 载体（e2e/浏览器调试）里。

- 设计：**assistant-ui 官方 design system**（“Every surface is a printed page”），中性色
  `--tint: 250`（冷灰蓝），**blue-500 = 唯一 live accent**（流式/运行中/已选中/连接中），无暖米色、无衬线、
  无 UI 渐变。token / 组件规则见 [`design-notes.md`](design-notes.md)。
- 字体：`@fontsource/public-sans`（400/500/600）+ `@fontsource/jetbrains-mono`（400/500）本地打包进
  `dist/`，中文回退 `PingFang SC`/`Noto Sans SC`/system-ui；无外部 CDN。
- 品牌 logo：`zk compose` 从 `brands/<channel>/` 生成 `src/generated/brand.ts` 与
  `public/brand/logo.svg`/`logo-dark.svg`；UI 统一用 `BrandLogo`（按主题切换），禁止手绘 logo。
- 消息/流式渲染：assistant-ui `ThreadPrimitive` + `ExternalStoreRuntime`；现有 `transport`/reducer 是唯一
  数据源（适配见 `src/assistant/runtime.ts`）。
- Markdown：`@assistant-ui/react-markdown`（react-markdown 安全渲染，未启用 raw HTML）。


## 包结构（一切皆包）

| 包 | 内容 |
| --- | --- |
| `packages/ui/ui-kit`（`@zeokeel/ui-kit`） | 共享 DTO/类型、`UiProvider/useUi`、`BrandLogo`、icons、Button/Switch/StatusDot/Chip |
| `packages/ui/ui-sessions`（`@zeokeel/ui-sessions`） | Sidebar：平铺 Workspace + 嵌套会话、workspace CRUD/首用/搜索、设置入口/窄窗（`sessions.sidebar`） |
| `packages/ui/ui-chat`（`@zeokeel/ui-chat`） | ChatView + assistant-ui Thread/Composer/ActionBar/工具卡/审批/空态（`chat.thread`） |
| `packages/ui/ui-settings`（`@zeokeel/ui-settings`） | SettingsDialog + 表单校验（`settings.dialog`） |
| `apps/desktop/web`（本目录） | 薄壳：transport/controller/reducer/i18n/theme resolve + `UiProvider` + registry 组装 |

`src/generated/registry.ts` 由 `zk compose` 生成，导出 `SessionsSlot/ChatSlot/SettingsSlot`；App 只从这里取组件，
换包 = 改 profile → `zk compose` → build。包间只允许依赖 `@zeokeel/ui-kit`（`@assistant-ui/*` 只在 ui-chat）。

## 开发与验证

```bash
cd apps/desktop/web
pnpm install          # workspace 安装（根 pnpm-lock.yaml 会更新）
pnpm build            # tsc --noEmit + vite build → dist/
pnpm test             # vitest + testing-library
pnpm dev              # 浏览器调试：http://localhost:5173/?mock=1
```

## 传输（transport）

| 场景 | 选择方式 |
| --- | --- |
| Tauri 桌面 | 自动检测 `window.__TAURI__` → `ipc_call` + `zk://event` |
| HTTP `serve` 载体 | 无 Tauri 时 `POST /ipc` + SSE `/events` |
| 本地/截图/组件测试 | 地址加 `?mock=1` → 内存 mock 后端（300ms 流式；输入含“读取”时发工具事件） |

IPC 形状见 `packages/contracts/ipc`：`CommandRequest`、`CommandResponse`、`UiEvent`；命令名
`meta.get`/`session.create`/`session.list`/`session.messages`/`chat.send`/`chat.cancel`/
`settings.get`/`settings.set`/`approval.decide`（M1 占位）。

> 前端只依赖契约形状；所有外部输入在 `src/protocol.ts` 做运行时校验，非法数据忽略并告警。

## 设计与证据

- 视觉/组件/映射说明：[`design-notes.md`](design-notes.md)
- mock 截图与 axe 报告：`screenshots/`
  （`01-empty`/`02-streaming`/`03-tool-card`/`04-settings`/`05-light`/`06-dark`/`07-narrow` + `axe.json`）
