# 28 · UI 模块地图与 UX 骨架

> 目标：先摸清 DSH 的 UI 功能，再按**功能内聚**决定合并/拆分（不追求 1:1 包），
> 给每个模块一个**空壳包 + 功能说明 + props/slot 契约**，后续按功能实现。
> 视觉/包组合权威仍是 `27`；槽位机制 `16`；工作区规则 `26`。

## 1. 合并/拆分原则

- **按交互面拆**：导航、对话、输入、右栏、全局命令、设置是不同交互面；可独立变化/替换的拆包。
- **按耦合合并**：DSH 的 chat/tool/approval/plan/goal 在 M1 共享“消息/turn”状态与渲染管线，**先合并为 `ui-chat`**；
  当 tool/approval/plan 需要独立贡献/替换时再拆（预留 slot）。
- **不拆不能替换的**：M1 的 composer 与 conversation 共享 runtime/焦点/键盘，**并入 `ui-chat`**，不单独建包。
- **全局单例**：shell、commands、settings 各自单包。
- **后置模块**：jobs/subagent/deliverables/plugins/market/automation/ browser/terminal 高级交互，先不建包；
  对应入口可灰显/占位，避免空包膨胀。

## 2. 模块地图（DSH → ZeoKeel）

| 模块 | DSH 包（事实参考） | ZeoKeel 包/角色 | 合并/拆分 | M1 范围 |
| --- | --- | --- | --- | --- |
| 应用框架/三栏/顶栏/状态栏 | `dsh-client-ui-layout` | `ui-shell` / `ui.shell` | 拆（全局单例） | 框架、header、status、响应式 |
| 导航（workspace/session/搜索/新建） | `dsh-client-ui-sidebar` + `-workspace` | `ui-sessions` / `ui.slot.sessions` | 合并（导航内聚） | 平铺 workspace + 缩进 sessions |
| 对话/消息/流式/工具/计划/审批 | `-chat`+`-tool`+`-approval`+`-plan`+`-goal` | `ui-chat` / `ui.slot.chat` | **M1 合并**；工具/计划/审批预留子组件/slot | 消息、流式、工具行、计划卡、审批卡 |
| 输入/composer | `-conversation` composer + `-input-trigger`+`-attachment` | 并入 `ui-chat` | **合并**（共享 runtime/焦点） | Enter/Shift+Enter/IME、/、@、模型、停止 |
| 右栏（文件/变更/终端） | `-sidebar-right`+`-files`+`-terminal` | `ui-inspector` / `ui.slot.inspector` | 拆（独立面板） | 文件/变更/终端 tabs、折叠 |
| 命令面板/快捷键 | `-commands`+`-shortcuts` | `ui-commands` / `ui.slot.commands` | 合并（全局 overlay） | ⌘K、命令列表、键盘提示 |
| 设置 | `-settings*` | `ui-settings` / `ui.slot.settings` | 合并（单 dialog，分节内部） | provider/主题/语言/审批预设 |
| 主题 | `-theme` | 并入 `ui-shell`（token/app.css） | 合并 | light/dark/system |
| Jobs/子代理/deliverables/插件市场 | `-jobs`/`-subagent`/`-deliverables`/`-plugin-*` | 暂不建包（灰显入口） | 后置 | 不实现 |
| 文件浏览器/终端高级交互 | `-sidebar-browser`/`-documentpreview` | 后续 `ui-inspector` 内面板 | 后置 | 只读占位 |
| 自动化任务/计划页 | `-schedule`/`-plan` | 后置 | 后置 | 灰显 |

## 3. 槽位与包骨架

| slotId | 角色 | 包 | skeleton component | props（M1 约定） |
| --- | --- | --- | --- | --- |
| `ui.shell`（M1 已接入） | `ui.shell` | `@zeokeel/ui-shell` | `ShellFrame`（`shell.frame`） | `boot/bootError/workspaceCount/workspaceError/sidebarOpen/offline/streaming/mock/title/packageLine/workspacePath/toast/navigation/conversation/settings/onRetryBoot/onToggleSidebar/onDismissToast/onCreateWorkspace/t` |
| `ui.sessions` | `ui.slot.sessions` | `@zeokeel/ui-sessions` | `Sidebar` | 现有 |
| `ui.chat` | `ui.slot.chat` | `@zeokeel/ui-chat` | `ChatView` | 现有 |
| `ui.inspector`（M1 已接入） | `ui.slot.inspector` | `@zeokeel/ui-inspector` | `InspectorPanel`（`inspector.panel`） | `open/tab/files/changes/changesState/changesError/changesWarning/onRetryChanges/onTab/onClose`；⌘B；变更 tab 读取 `workspace.changes`（`{items:[{path,add,del}],warning?}`） |
| `ui.commands`（M1 已接入） | `ui.slot.commands` | `@zeokeel/ui-commands` | `CommandPalette`（`commands.palette`） | `open/commands/onRun/onClose`（内部搜索/方向键/Enter/Esc） |
| `ui.settings` | `ui.slot.settings` | `@zeokeel/ui-settings` | `SettingsDialog` | 现有 |

- 新包先只提供**空壳组件 + props 类型 + README 功能说明**；`zk compose` 生成 registry。`ui.shell`/`ui.commands`/`ui.inspector` 均已在 M1 接入。
- `ui-shell` 已承接 AppShell 布局/header/status/overlay；app 薄壳保留 transport/controller/reducer/i18n/theme/first-run。
- 工具/计划/审批的独立 slot 预留（`ui.toolcall`/`ui.approval`/`ui.plan`），M1 不建独立包。
- **冲突裁决**：`docs/16/19` 的 M1 active slot 以 `ui.shell` + `ui.sessions/chat/settings` + `ui.commands` + `ui.inspector` 为准；`ui.inspector` 本轮已接入（文件树只读）。

## 4. 交互行为（UX 骨架目标）

### Navigation
- 平铺 Workspace；每行缩进其会话；默认 `~` 第一行；行可折叠；hover `⋯`（重命名/设默认/移除）。
- 点击会话同步 active workspace；新建会话用 active workspace；无 workspace → 首用。
- 搜索（会话/文件）；底部设置；窄窗抽屉 + Esc 关闭。

### Conversation + Composer
- 消息：用户右侧 low-contrast pill；助手无气泡；流式光标；取消保留内容 + “已取消”；错误内联。
- 工具行：单行折叠（状态点/name/关键参数/耗时），点击展开 args/result；Shift 点击展开全部。
- 工具状态**以 `docs/19` 为准**：`pending/running/success/failed/cancelled`（DSH 的 `preparing` 对应 `pending`）；审批决策后续映射 DSH `allowed-once/rejected`（M1 API 仍 `approved`）。
- **Plan/Todo（M2 占位）**：DSH 的 plan 是 Markdown 审阅 + Todo dock，不是步骤数组；M1 不实现，UX 骨架里的步骤卡标 “Todo · M2”。
- **审批（M1 inline 占位）**：允许一次/拒绝；DSH 的 composer takeover/持久策略/超时 post-M1；不要宣称 M1 有完整审批。
- Composer：Enter 发送、Shift+Enter 换行、IME 安全；`/` 命令 popup、`@` 文件引用、附件、模型下拉、上下文占用、发送/停止。
- Composer 的 `deliveryMode/attempt/queue/attachments` 为 post-M1 预留，不在 M1 承诺；M1 不注册 `ui.composer` slot，避免与 `docs/19` 冲突。
- 空态/加载/错误/离线/流式中/已取消六态。

### Inspector
- Tabs 文件/变更/终端；`⌘B` 折叠；窄窗自动收起；文件树懒加载（后续）、变更 diff 计数、终端 mock。
- 状态：当前 tab/宽度/选中文件；只读占位，不调用后端。

### Commands / Settings
- ⌘K 打开/关闭；↑↓/Enter/Esc；命令项类型（navigation/action/settings）。
- 设置 dialog（`2026-11-01-client-settings-ia` 落地）：左侧分区导航（窄窗 <720px 顶部横向 tab）+ 右侧 pane；全部 pane 由
  `ui.settings.pane` 列表 slot 贡献（order 10 通用 / 20 模型 / 21 本地 Provider / 30 Agent / 40 工具与审批 / 50 网络与检索 /
  60 嵌入 / 65 媒体 / 80 企业 / 90 关于），personal/enterprise 差异由构建期 `editions` 过滤，pane 内无 edition 分支。
- 单一底部保存：合并全部 pane 草稿为整份 `AppSettings` 提交，`expected_revision` 栅栏冲突时保留草稿并提供「重新加载」；
  切换 pane 不丢草稿；Esc 关闭、焦点陷阱、关闭后焦点回到入口；provider/api key 仍掩码、留空保持。

## 4.5 DSH 事实校正（来自三份模块摸排）

- DSH 把 **conversation 装配层**（target-neutral event/view registry、session binding、composer shell）与 **chat 目标层**（消息节点/渲染）分开；
  ZeoKeel M1 合并进 `ui-chat`，但内部按 `conversation/`、`composer/`、`tool/`、`approval/` 目录组织，post-M1 可拆包。
- DSH **Composer 是 shell + takeover 链**：审批、plan chip、GoalBar、queue、附件各自 slot；ZeoKeel M1 的 composer **渲染在 `ui-chat` 内**，`ui.composer` slot 仍为 post-M1 保留，不注册、不验收。
- DSH **审批**是 allow-once/reject 的临时请求，持久策略/Host waterfall 不在 UI；ZeoKeel M1 同（bridge 占位）。
- DSH **plan 是 plan mode + Markdown review**，步骤清单属于 Todo dock；ZeoKeel M1 的“计划卡”是简化版，后续拆 `ui-plan`/`ui-todo`。
- DSH **Jobs/Subagent/Deliverables** 都在 header/turnTail/右栏，M1 不建包；右栏“变更”是 ZeoKeel 自有（DSH 无 changes tab）。
- DSH **命令**：composer `/` 菜单 + `Mod+/` 快捷键参考；没有本组包可核实的全局 ⌘K 命令面板。
  ZeoKeel 的 `ui-commands` 暂定“`/` 菜单 + 快捷键参考 + 可选全局面板”；全局 ⌘K 是产品增强，不是 DSH 复刻。
- DSH **右栏是 per-session docking surface**（normal/fullscreen、tab 不销毁、宽度由 frame 管）；ZeoKeel M1 只做固定宽度单栏、折叠；
  多 tab/拖拽/多 pane 后置。
- DSH **设置**顶级页是 Account / General / Models / Built-in plugins；Subagent/Web search/Agent loop/Shell 挂在侧栏 Plugins 页下，Session log 是 General 一行；
  ZeoKeel 已按 settings-ia 变更为**分页 dialog**（10 个 pane 贡献，见 §4「Commands / Settings」）；设置内搜索/插件清单仍后置。
- DSH **主题**是 Host bootstrap + token；ZeoKeel 沿用 app token/`data-theme`，不引入运行期主题包。
- DSH **插件市场运行期装包**与 C2 冲突，ZeoKeel 不采用；Settings 插件仅只读清单（后续）。

## 4.6 待裁决项的当前决定

1. **Plan**：M1 不做 plan mode；步骤卡降级为 “Todo · M2” 占位（DSH 的 plan 是文档审阅，不是步骤数组）。
2. **Approval**：M1 inline 占位（允许一次/拒绝），post-M1 再谈 composer takeover/持久策略/超时。
3. **工具/审批 DTO**：以 `docs/19` 为准 `pending/running/success/failed/cancelled`；审批决策后续 `allowed-once/rejected`，M1 API 保持 `approved` 并映射。
4. **Jobs/Subagent/Deliverables**：骨架入口灰显/隐藏，不实现；不建包。
5. **Composer**：M1 不做 deliveryMode/queue/attachments 完整状态，类型只预留。
6. **Slot/M1 范围**：M1 仍只有 `ui.sessions/ui.chat/ui.settings` 三个 active slot；`ui.shell/ui.inspector/ui.commands` 是骨架，不接入 M1。

## 4.7 导航后置项裁决（回应 navigation 报告 §5）

1. **post-M1 侧栏拆分**：M1 保持 `ui-sessions` 单包单 slot；不建 `ui.sidebar`/子 slot。只有出现第二消费者或明确替换需求时才拆。
2. **M1 会话操作**：仅新建/选择/workspace 重命名/workspace 移除；**session rename/fork/copy ID 后置**（与 pin/archive/reorder 一起）。
3. **状态契约**：M1 `running | idle`；`unread | pending | scheduled` 后置。不提前定义 unread 语义。
4. **平铺/ungrouped**：永久平铺 Workspace（用户要求），**不做 ungrouped**；移除有会话必须 reassign（C18 唯一例外）。
5. **归档/删除**：M1 无 session delete/archive；post-M1 先做 archive-only，**不提供硬删除**；是否删除由后续 spec 决定。
6. **/ 菜单与 ⌘K**：同一 `ui-commands` 包内**按 surface 分开组件/状态**（composer `/` popup vs shell ⌘K palette），不共享一个组件状态。
7. **侧栏宽度**：以实际 248px 为准；`docs/27` 已同步为 248。

## 5. 骨架验收（本轮）

- 每个新包有四件套（`zk.toml`/契约/README/空壳组件+props），`zk validate`/`compose --check` 绿。
- `ux.html` 可用 mock 数据完整操作：切 workspace/session、折叠、tabs、右栏折叠、⌘K、设置、工具展开、审批、composer 发送/停止。
- 截图 light/dark/narrow/empty/loading/error/cancelled；不接后端；**不实现真实功能**。
- 后续按功能逐个把空壳替换为实现，并保持 slot/包边界。

## 6. 模块摸排报告

- `research/dsh-ui-modules/navigation.md`：sidebar/workspace/layout/commands/shortcuts/session。
- `research/dsh-ui-modules/conversation.md`：conversation/chat/tool/approval/plan/goal/jobs/subagent/deliverables/input-trigger/attachment/feedback。
- `research/dsh-ui-modules/inspector-settings.md`：sidebar-right/files/terminal/preview/browser/settings/commands/shortcuts/theme/plugins。
这些报告只作事实参考（C15），不进入依赖/构建输入。

> 本文只覆盖 **M1 personal UI**（含骨架）；M2 企业登录/首配/管理台/网关 UI 不在本文范围（见 `04`/`08`/`19`）。
