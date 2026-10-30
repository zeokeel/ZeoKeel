# 16 · 前端也是包：M1 实现与目标模型

> 状态：**M1 已实现（2026-10-30；证据补跑见 §5）**；目标模型（T21）见 §6，尚未实现，不得按目标语气描述现状。
> 视觉/品牌/评审权威见 `27`；工作区上下文见 `26`；本文件只讲 UI 包的声明/registry/边界。

## 1. M1 采用形态（现状）

- **pnpm workspace 成员包**：UI facet 在包内 `web/`；根 `pnpm-workspace.yaml` 覆盖 `apps/*/web`、`packages/*/*/web`。
- **声明真源 = `zk.toml`**：每个 UI 包在 manifest 里用 `[[ui.contributions]]` 声明 `slot/component/order`；
  `zk compose` 读取后生成 `apps/desktop/web/src/generated/registry.ts` 与 `shared/contracts/ui-contributions.json`。
- **build-time 组合**：换 UI 包 = 改 edition/profile → `zk compose` → 重新 `pnpm build`；运行期无加载器/热插拔（C2）。
- **shell 只认 slot/component**：`App.tsx` 从 `registry.ts` 取 `ShellSlot/SessionsSlot/ChatSlot/SettingsSlot`，不直接 import 业务 UI 包。
- **M1 app 不接入**（已注册骨架）`package.json#zeokeel.client`、`props_schema` 闭包校验、`virtual:@zeokeel/registry`、`scripts/lint-boundaries.mjs`、
  `ui-shell-kit`/transport 包——这些是 §6 的目标模型（T21），不是现状；`ui-shell` 已在 M1 接入。

## 2. M1 包结构（active 6）

| 包 | 角色 | 内容 | 依赖 |
| --- | --- | --- | --- |
| `@zeokeel/ui-kit` | `ui.kit` | 类型/DTO、`UiProvider/useUi`、BrandLogo、icons、Button/Switch/StatusDot/Chip | react |
| `@zeokeel/ui-sessions` | `ui.slot.sessions` | Sidebar（**平铺 Workspace 列表**/会话列表/新建/选中/设置入口/窄窗；规则见 `26` §6） | ui-kit |
| `@zeokeel/ui-chat` | `ui.slot.chat` | ChatView + assistant-ui primitives + 工具卡/审批/空态 + ExternalStoreRuntime adapter | ui-kit + `@assistant-ui/*` |
| `@zeokeel/ui-settings` | `ui.slot.settings` | SettingsDialog + 校验 | ui-kit + Radix dialog |
| `@zeokeel/ui-shell` | `ui.shell` | AppShell + FirstRun（布局/header/status/overlay/首用页；app 已接入） | ui-kit + Radix tooltip |
| `@zeokeel/ui-inspector` | `ui.slot.inspector` | 右栏文件树/变更/终端（`workspace.files`，⌘B） | ui-kit |
| `@zeokeel/ui-commands` | `ui.slot.commands` | ⌘K 命令面板（搜索/键盘导航/动作由 app 注入） | ui-kit |
| `apps/desktop/web` | 薄壳 | transport/controller/reducer/i18n/theme/UiProvider/registry/first-run | registry 生成的包 |

依赖方向：`ui-chat/ui-sessions/ui-settings → ui-kit`；app → `generated/registry` → UI 包；**包不得 import app，业务包不得互相 import**。
`@assistant-ui/*` 只允许出现在 `ui-chat`；React 为 peer dependency。共享视觉/交互必须进 `ui-kit`（C17）。

### 2.1 目录与 manifest（实际）

```text
packages/ui/ui-kit/{zk.toml,Cargo.toml,src,tests,web/{package.json,src}}
packages/ui/ui-sessions/...
packages/ui/ui-chat/web/src/{index.tsx,runtime.ts,AssistantThread.tsx,...}
apps/desktop/web/src/generated/{brand.ts,registry.ts}   # @generated，勿手改
```

```toml
# packages/ui/ui-chat/zk.toml（节选）
kind = "ui"
[role]
name = "ui.slot.chat"
contract = "zk-contract-ui"
cardinality = "singleton"
editions = ["personal", "enterprise"]
[facets]
rust = { crate = "zk-ui-chat" }
ui = { package = "@zeokeel/ui-chat" }
[[ui.contributions]]
slot = "ui.chat"
component = "chat.thread"
order = 10
```

### 2.2 槽位（M1 实际）

| slotId | 角色 | 组件 | 说明 |
| --- | --- | --- | --- |
| `ui.sessions` | `ui.slot.sessions` | `sessions.sidebar` | 侧栏（**平铺 Workspace 列表**/会话列表） |
| `ui.chat` | `ui.slot.chat` | `chat.thread` | 对话区（含 composer/工具卡/审批/空态） |
| `ui.settings` | `ui.slot.settings` | `settings.dialog` | 设置弹窗 |
| `ui.shell` | `ui.shell` | `shell.frame` | 应用框架（布局/header/status/overlay；app 已接入） |
| `ui.inspector` | `ui.slot.inspector` | `inspector.panel` | 右侧检查器（已接入） |
| `ui.commands` | `ui.slot.commands` | `commands.palette` | ⌘K 命令面板（已接入） |

M1 **active** slot 为 `ui.shell` + 内容三槽 `ui.sessions/chat/settings` + `ui.commands` + `ui.inspector`；新增 slot 必须同时改 `tools/zk` 的 `slot_binding` 与本文，否则 `registry.ts` 不生成。

### 2.3 生成器行为（M1 实际）

1. 扫描 `packages/ui/*/zk.toml` 的 `[[ui.contributions]]`；
2. 生成 `registry.ts`：`import { components as cN } from "<ui.package>"`、`COMPONENTS`、`XxxSlot = cN["component"]`；
3. 生成 `shared/contracts/ui-contributions.json`（package/role/slot/component/order）；
4. `--check` 逐字节比较；生成物禁止手改（C11）。
5. **不校验** `props_schema`/slot props 兼容/组件闭包（T21）；M1 由 TypeScript 编译 + 运行时 guard 保证。

## 3. 边界与共享（M1）

- `apps/desktop/web/src` 不得出现业务 UI 的第二份实现；已删除旧 `components/**`、`assistant/**`、`settings-validation.ts`。
- app 可直接使用 `@zeokeel/ui-kit` 基础件（BrandLogo/icons 等）；**业务 slot 必须经 registry**。
- Tailwind 4：`src/app.css` 必须为每个 UI 包源码加 `@source`；新增 UI 包必须同步加，否则包内 utility 丢失。
- 类型：M1 的 DTO 是 `ui-kit/types.ts` 手写镜像（注释标注 Mirrors）+ `guards.ts` 运行时 guard；这是 **C10 的 M1 过渡例外（ADR-059）**，ts-rs/schemars/Ajv 是 T22（M2 前硬退出条件），不是现状。
- i18n/theme/transport/controller/reducer 仍在 app（应用基础设施，不是展示包）；`t` 经 `UiProvider` 注入。

## 4. 换包主通道（M1）

1. 改 `editions/<edition>.toml`/`profiles/<profile>.toml` 的 `ui.slot.*` 映射到另一个包 id；
2. `zk validate` → `zk compose` → `pnpm install`（候选包并集）→ `pnpm build`；
3. e2e/截图/axe 重新跑；运行期不切换。

## 5. 测试与证据（M1）

- `pnpm --dir apps/desktop/web build` + `pnpm test`（vitest）。
- Playwright 真实 serve 载体：建会话→发送→流式→工具卡→设置→刷新恢复；`?mock=1` 只作演示。
- 截图 light/dark/narrow/六态；axe 合法 JSON、0 serious/critical；详见 `27` §5/`11` §8–§9。

## 6. 目标模型（post-M1 / T21；尚未实现）

以下为外部/独立发布 UI 包与更强边界的目标，**未实现**；实施前需先评审并升 T21 spec：

- `package.json#zeokeel.client`（platform/role/baseline/components/inject）清单；
- `props_schema` + slot props 兼容/组件闭包校验；`registry.gen.ts` / `virtual:@zeokeel/registry`；
- `scripts/lint-boundaries.mjs` 读 rollup metafile 拒绝越界 import；
- `ui-shell-kit`/`ui-shell`/transport-tauri/http/mock 包拆分；
- ts-rs 生成 `shared/ts/*`、schemars JSON Schema + 前端 Ajv（C10）；
- 完整 SLOT_DEFS（login/onboarding/first-run/gate/skills/connectors/apps/update/diagnostics/admin.*）与对应包。
这些目标不得写进 M1 验收；M1 只按 §2–§5 验收。
