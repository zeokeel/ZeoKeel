# DSH Web 导航 / Workspace / Session 侧栏研究

> 范围：`@deepseek-ai/dsh` **0.2.1-rc.2** 的 `ui-sidebar`、`ui-workspace`、`ui-layout`、`ui-commands`、`ui-shortcuts`、`ui-session`。
> 方法：只读 README + 包内公开 `.d.ts`；快捷键默认值用 bundled `lib/client.js` 的注册处核对（README 未写明的字段标“类型声明”）；未读到的运行时细节标“未验证”。
> 约束：DSH 仅作事实参考，不拷贝代码、不作依赖或构建输入（C15）。本文只服务 ZeoKeel UI 模块骨架设计。

## 0. 引用约定

- `P` = `/usr/local/lib/node_modules/@deepseek-ai/dsh/node_modules/@deepseek-ai/`
- `S` = `P/dsh-client-ui-sidebar/README.md`；`W` = `P/dsh-client-ui-workspace/README.md`
- `L` = `P/dsh-client-ui-layout/README.md`；`C` = `P/dsh-client-ui-commands/README.md`
- `K` = `P/dsh-client-ui-shortcuts/README.md`；`U` = `P/dsh-client-ui-session/README.md`
- `Z16/Z19/Z26/Z27/Z28` = `docs/16-ui-packages.md`、`19-client-ux-flows.md`、`26-workspace-and-conversations.md`、`27-ui-design-system.md`、`28-ui-module-map-and-skeleton.md`
- 行号引用统一写作 `S:34`、`W:30`、类型声明写作 `P/<pkg>/lib/types/...:N`。

## 1. 模块清单

| 包 | 负责什么 | 贡献/占用的 slot/seat | 依赖的服务/上下文 |
| --- | --- | --- | --- |
| `ui-sidebar` | 侧栏壳：品牌、New Session、折叠按钮、全局面板列表、滚动区、底部 Settings | **占用** `sidebar`、`shell.leading`（macOS 侧栏完全隐藏时）；**声明** `sidebar.brand.mark`、`sidebar.brand.name`、`sidebar.toggle.badge`、`sidebar.panellist`、`sidebar.workspaces`、`sidebar.settings`、`sidebar.footer.action`（最后一项见类型声明 `P/dsh-client-ui-sidebar/lib/types/client/contract/slots.d.ts:76`） | `slots`、layout owner share、全局 `useSessions`/`useWorkspaces`、`ctx.shortcuts`、`ctx.locale`、导航回调（`startSession`/`toggleSidebar`/`selectPanel`）S:66 |
| `ui-workspace` | Workspace 浏览器 + 新会话 Workspace picker：分组/平铺列表、搜索、置顶/归档/排序、重命名、删除、复制 ID、fork、目录选择 | **占用** `sidebar.workspaces`、`conversation.hero.workspace`（`P/dsh-client-ui-workspace/lib/types/client/contract/slots.d.ts:468`）；**声明** `sidebar.workspaces.directoryFlow`、`conversation.hero.workspace.directoryFlow`、`sidebar.workspaces.session.menu.item`、`sidebar.workspaces.session.row.action`、`sidebar.session.row.leading`、`sidebar.session.row.hover`；**贡献** `shell.overlay`（重命名框、stop-and-archive 框、notice 分发）与 `workspace.toast` keyed 内容 | 注入 `remoteAccess`，提供 `ctx.uiWorkspace`；用 `ctx.slots`、`ctx.layout`、`ctx.shortcuts`、`ctx.locale`、Session/Workspace Controller 投影（W:74、W:84、W:100-104） |
| `ui-layout` | 三栏 `AppFrame`、列宽/收放、右侧栏轨道、主题呈现、面板选择 | **占用** `root`；**声明** `sidebar`、`main`（keyed，`conversation` 为保留 key）、`rightbar`、`shell.bottom`、`shell.overlay`、`shell.leading`；提供 `ctx.layout`（`selectPanel`/`beginNavigation`/`toggleSidebar`/`openRightbar`/`closeRightbar`）L:59、`P/dsh-client-ui-layout/lib/types/client/index.d.ts:44-112` | `slots`、`useSessions`/`usePanelInfo`、theme、shortcuts、locale |
| `ui-commands` | `/` 命令源与命令面板：popupSelect / client action / host leadingInput / execute 四类解析结果；每会话命令目录 | **占用** `conversation.input.overlay`（`PopupSelectView`，`P/dsh-client-ui-commands/lib/types/client/index.d.ts:5`）；向 input-trigger 注册 `/` source | `ctx.commandUi`；input-trigger、conversation、Session scope、commands Remote、locale（C:58） |
| `ui-shortcuts` | 快捷键参考/录制/清除/恢复默认；Settings 入口 | **贡献** `settings.general.item`（`ShortcutsRow`）与 `shell.overlay`（`ShortcutReference`），`P/dsh-client-ui-shortcuts/lib/types/client/Reference.d.ts:25,31`；注册 `Mod+/` 命令 | `ctx.shortcuts`（catalog/config/fixedCatalog）、locale、settings |
| `ui-session` | Session Controller 的 React/Hook 适配：列表、保留计数、统一 UI 状态、`SessionProvider` 作用域 | **不占用业务 slot**；提供 root hooks `useSessions`/`useSessionStatus`/`useSessionRetainInfo`/`useSessionAccess`，以及 `session`/`session-maybe` scope 的 `useSession`/`sessionId`/`useProjection`/`useAccess`（U:13、U:27；`P/dsh-client-ui-session/lib/types/client/index.d.ts:74-109`） | 注入 Session Controller、`remoteAccess`、renderer；提供 `ctx.uiSession` |

关键所有权规则：`sidebar` 是 single slot，注册者会**替换整个导航列**，并连带其内部 seat 消失；要加东西应注册到内部 seat，而不是再占 `sidebar`（`P/dsh-client-ui-layout/lib/types/client/index.d.ts:34-48`）。`ui-workspace` 用 `slots.inject()` 等待 `sidebar.workspaces` 声明，声明消失时自动撤下（W:92）。

### 1.1 组件树（侧栏 + 三栏）

```text
root (single; ui-layout AppFrame)
├─ shell.leading (single; 仅 macOS 侧栏完全隐藏时)
│  └─ HeaderLeadingControls (ui-sidebar：展开侧栏 + New Session)
├─ sidebar (single; ui-sidebar SidebarRoot)
│  ├─ brand row: sidebar.brand.mark / sidebar.brand.name
│  ├─ New Session + 折叠按钮 (+ sidebar.toggle.badge)
│  ├─ sidebar.panellist (list; 全局面板图标, id ↔ main key)
│  ├─ sidebar.workspaces (single; ui-workspace WorkspaceBrowser)
│  │  ├─ header: section 标题 / 搜索 / 视图选项 / Add workspace
│  │  ├─ sidebar.workspaces.directoryFlow (single; 目录选择器占用)
│  │  └─ 分组区: Workspace 行 → Session 行
│  │     └─ Session row
│  │        ├─ sidebar.session.row.leading (list; 仅 idle 时的 16px 前导位)
│  │        ├─ title / time / 状态点
│  │        ├─ sidebar.workspaces.session.row.action (list; hover 按钮)
│  │        ├─ "..." → sidebar.workspaces.session.menu.item (list)
│  │        └─ hover card → sidebar.session.row.hover (list; 仅悬停卡打开时)
│  ├─ sidebar.footer.action (list)
│  └─ sidebar.settings (single; ui-settings 触发行 + 面板)
├─ main (keyed; "conversation" 保留; ui-conversation)
│  ├─ conversation.hero.workspace (single; WorkspacePicker)
│  │  └─ conversation.hero.workspace.directoryFlow (single)
│  └─ conversation.input.overlay (ui-commands popup)
├─ rightbar (single; ui-sidebar-right; 经 SessionProvider 渲染 rightbar.session)
├─ shell.bottom (single; 默认空)
└─ shell.overlay (list; 重命名、stop-and-archive、notice、快捷键参考)
```

`sidebar.workspaces.session.menu.item` 的 shipped 顺序：`pin`(100)、`rename`(200)、`fork`(300)、`archive`(400)、`session-id`(450)；`sidebar.workspaces.session.row.action` 为 `archive`(100)、`pin`(200)（W:104）。这两个 list 的 entry 只拿 `sessionId`/`displayTitle`，其余状态由 entry 自己注入读取（W:106）。

## 2. 交互

### 2.1 新会话目标 Workspace 选择规则

优先级固定为：**显式选中的 Workspace**（scoped action，或 hero picker / Workspace 行的新建）→ **当前 Session 的 Workspace** → **最近活跃的 Workspace**；三者都没有时，进入空白 New Session 页面（不建 Session）S:14、S:34、W:74。

- 点击 expanded brand 在 Web/Windows 上等同 New Session；macOS Desktop 上品牌不作为快捷入口，保留专用按钮（S:34、S:52）。
- New Session 按钮在 hover/键盘聚焦时用尾部灰字显示“实际绑定的 Workspace”；空间不足时中心图标/文字先于快捷键淡出，CSS 预留快捷键宽度（S:34）。
- `openWorkspace` 只有“没有更晚导航”时才提交；`startSession` 的 `prompt` 是纯文本，`clearPreviousDraft` 只清目标会话的文本与 inline reference，保留附件（W:74、W:96）。
- 启动恢复使用保存的 blank 与其记录的 preset；preset 失效时回退 Host 默认；显式 New Session 失败弹 transient notice，superseded 请求不弹（W:78）。
- 空安装会 `workspaces.initializeDefault` 并创建/复用 blank Session（W:80）。**ZeoKeel 不同**：首用只建默认 Workspace，不自动建 Session（Z26:30）；无 Workspace 时禁止建会话（Z26:23）。

### 2.2 Workspace 折叠/展开与侧栏收放

- Workspace 分组可折叠；折叠会**重置该 Workspace 及嵌套 Workspace 的 watermark**，但恢复后仍覆盖“符合条件”的 Session（W:30）。
- `Group by` 默认 `WorkSpace`（兄弟 section）；`Workspace Tree` 按最近已登记祖先嵌套，子 Workspace 排在父 Workspace 自己的 Session 之前；祖先默认展开，除非有保存的折叠状态；保存的折叠会隐藏当前 Session，祖先文件夹图标仍高亮（W:44）。
- 分组/展开状态保存在当前浏览器；切换模式保留各 Workspace 的展开偏好；单列表视图保持扁平（W:44）。
- 侧栏列：264–420px，默认 280px，收起为 **56px rail**；<1024px 自动收起；右栏打开会让“手动展开”的侧栏收起（L:30）。
- 窄窗下 `narrowExpanded` 是独立 override：再点 toggle 会在被挤压的 center 上展开，而不是改持久宽度（`P/dsh-client-ui-layout/lib/types/client/stores.d.ts:18-28`）。
- 右栏 concession 顺序：首次打开 45% viewport，之后保留像素偏好、上限 70%；为保住 center 的 400px，先把右栏压到 300px，再报告空间不足让 occupant 关闭，最后才继续压 center；关闭/fullscreen 时没有外侧 resize handle，拖拽无过渡（L:30）。
- macOS Desktop 收起时**不留 rail**，整列隐藏并由 `shell.leading` 补回展开/新建按钮（S:52、L:39）。Windows Electron 的 caption 行为另有一套（S:48、L:41）。
- 折叠动画：expanded 内容在当前宽度淡出，上部控件一起淡出并向左平移到 56px rail，列滑动收尾；初始即收起则静态渲染；`prefers-reduced-motion` 关闭过渡（S:46）。
- 滚动条是 pointer affordance：指针不在列内时透明，离开后 thumb 再保留 2s；滚动区预留宽度由 `ui-workspace` 负责，避免行位移（S:56）。

### 2.3 Session watermark 与 Show more

- 每个打开的 Workspace 初始显示前 **5** 个非空 Session；watermark 会上升到“前 50 个中最后一个符合条件”的 Session：running、有 running 子会话、等待用户、有未读完成、或在 main view 打开；从搜索打开会把 watermark 提到该 Session（任意位置）（W:30）。
- watermark 是**行数且不会自行下降**：run/通知结束后行仍可见；reorder 把 Session 移到 watermark 之后则隐藏（W:30）。
- **Show more** 每次 +5，不再受 50 行限制；剩余 ≤5 时显示其余并在最后一批后提供 **Show less** 重置（W:30）。
- 选中的 blank **New Session** 是 watermark 之外的额外行，直到第一次 prompt；watermark 不持久化，grouped tree unmount 即重置（W:30）。
- 当前 UI 的会话标题超过行宽时省略号截断；hover 行会把标题滚到远端（如 fork 的递增标题），离开回到起点（W:60）。

### 2.4 pin / archive / reorder / search / rename / delete 的行为

| 动作 | 入口 | 行为要点 |
| --- | --- | --- |
| Pin | menu 100 / hover 200 | 移到**完整保存序列**的最前；pinned 分区在两种视图都领先；不改变当前排序模式；Manual 中领先 pinned 分区，Last updated 中不必领先；unpin 不改保存位置（W:36、W:38） |
| Archive | menu 400 / hover 100 | 安静 Session 直接归档；有 running 工作时 Host 拒绝，侧栏弹 **stop-and-archive**，按 family 列出 turn/subagents/jobs/reminders，确认后先停再归档，取消则继续运行；归档保留账号位置；成功 notice 带 undo，隐藏归档时可切到 All conversations（W:56、W:58） |
| 归档过滤 | View options | 三态：**Hide archived**（默认）/ **All conversations** / **Archived only**；Archived only 会丢掉没有归档 Session 的 Workspace；可见归档行灰显、不可打开，但 Rename/Fork/Unarchive 可用（W:58） |
| Reorder | 拖拽行/Workspace | Session 拖拽进入 **Manual** 并编辑完整序列（含隐藏 archive）；pinned/ordinary 共用同一序列；折叠组拖拽用目标 Session 身份；Session 显示顺序 browser-local，**Workspace 组顺序 Host-durable**（W:38、W:40） |
| Search | 头部按钮展开输入 | 非空 query 把两种浏览模式替换为单一扁平结果：立即做 title/Workspace 大小写不敏感子串匹配；250ms debounce 后合并 Host 内容匹配与 snippet；新 query abort 前一请求；内容搜索失败仍保留 metadata 结果；上限 **20**；选中未归档结果会清空并收起搜索、打开 Session、滚动到行并展开祖先；归档结果只提供 Unarchive（W:50） |
| Rename | menu 200 / 双击标题 | 对话框预填显示标题；确认未修改也允许，用于**钉住自动标题**；Workspace 重命名只改 title，重复标题检查按 Workspace 身份排除自身（W:56、W:82） |
| Delete | 仅 Workspace 删除 | 确认框说明保留边界；删除只移除注册，Session 不删；DSH 把 Session 归入 **Ungrouped**，父 Workspace 删除后子 Workspace 归最近祖先或 root（W:56、W:46、W:222）。**ZeoKeel 不允许 ungrouped**，必须 `reassign_to`（Z26:49、Z26:34） |
| Copy Session ID | menu 450 | 关闭菜单并直接复制完整 ID，不打开对话框、不加载历史、不调 Host；成功/失败进全局 toast（W:54、W:56） |
| Fork | menu 300 / 快捷键 | 用源 Session **最后一个完成 turn** 创建子会话，递增继承标题；不 retain、不加载子历史、不改变选择；源需要迁移时只导航、不重试（W:56） |

补充：**DSH 没有 Session 删除**，只能归档；归档行通过归档视图或搜索结果恢复（W:222）。

### 2.5 状态点

| 状态 | 行内呈现 | 事实来源 |
| --- | --- | --- |
| running | 共享 ongoing loader；父行可由已加载子目录中的 running direct children 触发，但**子会话行本身不建立 running** | U:13、W:181 |
| unread completion | “done” 点；`completionUnread` 是**本地查看状态**，`completion-time > last-seen` 从不回传 Host | U:13、S:107、`P/dsh-client-ui-session/lib/types/client/index.d.ts:30-36` |
| pending interaction | 共享 warning 点；尾部更新时间替换为 **Approval** / **Plan review** / **Answer**；优先级高于 loader | W:66 |
| scheduled | 时钟 mark，位于 `sidebar.session.row.leading`；仅 idle 行渲染；不是按钮、无 tab stop，按压不打开行；搜索结果无 leading seat | W:70 |
| archived | 状态点与 leading seat 都留空；仅在 hover card 展示 live status | W:66 |
| idle | 行内无点；hover details 用共享 idle 点 | W:66 |

leading seat 只在“主状态 idle”（无 pending、无自身/后代 activity、无 unviewed completion）时挂载，所以 occupant 永远不会和状态点并排（W:66）。

### 2.6 空态

- 列表为空：居中 glyph-over-text 占位；Archived-only 视图自己命名空态 **“No archived sessions yet”**，并给 **View other sessions** 文本动作切回 Hide archived（W:58）。
- 没有 `sidebar.panellist` 注册时，列表与其间距都不渲染（S:38）。
- 目录选择 hole 为空时 **Add workspace** 不出现；补目录选择交互是唯一新增入口（W:100）。
- `shell.bottom` 空内容不预留空间；有内容时三列高度相应缩减（L:34）。

### 2.7 窄窗 / 收起 rail / 键盘快捷键

- 窄窗不是抽屉：DSH 保持 56px rail；<1024 自动收起，手动 toggle 可覆盖展开；右栏先让位（L:30、L:95）。ZeoKeel 规范是 ≤900px 抽屉（Z27:39），两者不同，不能直接照搬。
- macOS 完全隐藏侧栏；Windows caption 有自己的折叠/菜单布局（S:48、S:52、L:41）。
- 键盘（Desktop / Web macOS+Windows；Linux Web 默认不绑定，见 W:62）：

| 命令 | Desktop | Web (mac/win) |
| --- | --- | --- |
| 切换左侧栏 | `Mod+B` | `Mod+Alt+B`（L:28；`P/dsh-client-ui-layout/lib/client.js:655`） |
| New Session | `Mod+N` | `Mod+Alt+N` |
| Search sessions | `Mod+K` | `Mod+Alt+K` |
| Add workspace | `Mod+O` | `Mod+Alt+O` |
| Rename session | `Mod+Alt+G` | `Mod+Alt+G` |
| Fork session | `Mod+Alt+F` | `Mod+Shift+F` |
| Archive session | `Mod+Shift+A` | `Mod+Alt+A` |
| 快捷键参考 | `Mod+/` | `Mod+/` |

以上六条 workspace 快捷键的默认值在 `P/dsh-client-ui-workspace/lib/client.js:123-183`；`Mod+/` 见 K:30。Rename 需要 main 有非空 Conversation、无 modal 遮挡且没有全局面板处于激活；Search/Rename 属于本包，输入草稿留在浏览器（W:62）。`Mod+B` 在 modal 后被禁用，并让位给 terminal 输入（L:28）。上表中除 `Mod+/` 外的七条（layout toggle + 六条 workspace）Linux Web 默认不绑定（W:62）；`Mod+/` 是例外，Linux Web 接受 `Mod+Slash`（K:77）。

### 2.8 命令面板（与侧栏/Composer 的边界）

- 输入 `/` 或点 Composer 的 `+` 打开同一菜单：Add（File/Goal/Plan/Feedback）与 Commands（Compact/Permission/Model/Export），每行带 glyph、本地化标题/描述，标题与命令名不同时把命令名作 alias（C:28）。
- 命令行**不会被静默降级为普通 prompt**：Space/Enter 按会话目录解析为 `leadingInput`、`popupSelect`、`action` 或 `execute`（C:12）。
- 带图片/附件的提交只允许声明 `input.attachments` 的 host command；其他命令抛 `attachmentsUnsupported`，draft 与附件卡保持原位（C:46）。
- 命令目录按会话缓存，需要已 retain 的 Client Session 且首次 history open 成功后才发 `commands.list`；未 retain/打开失败直接拒绝，不偷偷重开会话（C:60）。

## 3. 关键 props / 状态（仅 README + 公开类型可证）

| 对象 | 关键字段 | 来源/可信度 |
| --- | --- | --- |
| `SidebarRoot` props | `collapsed`、`width`、`startSession`、`toggleSidebar`、`selectPanel`、`usePanels`、`useShortcuts`、`usePanelInfo`、`t`、`renderSlot` | `P/dsh-client-ui-sidebar/lib/types/client/SidebarRoot.d.ts:7`；类型声明 |
| `SidebarSection` owner share | `wide`（宽列/rail）、`expandSidebar()`（rail 图标请求展开） | `P/dsh-client-ui-sidebar/lib/types/client/contract/slots.d.ts:113` |
| Browser view state | `groupBy: 'workspace'\|'workspace-tree'\|'flat'`、`orderBy: 'manual'\|'updated'`、`groupExpansion: Record<string,boolean>`、`sessionOrderByAccount`、`archivedFilter?: 'default'\|'show'\|'only'` | `P/dsh-client-ui-workspace/lib/types/client/stores.d.ts:19-27`；类型声明 |
| `SessionNode` | `id`、`title`、`blank`、`pendingInteraction?`、`running`、`runningSubagentCount`、`completed`、`pinned`、`archived`、`updatedAt` | `P/dsh-client-ui-workspace/lib/types/client/tree.d.ts:23` |
| `GroupNode` | `key`、`workspaceId?`、`cwd?`、`createdAt?`、`label`、`sessionCount`、`expanded`、`containsCurrent`、`sessions` | `P/dsh-client-ui-workspace/lib/types/client/tree.d.ts:45` |
| `SearchResultNode` | `id`、`title`、`workspace`、`pendingInteraction?`、`running`、`runningSubagentCount`、`completed`、`archived`、`snippet?` | `P/dsh-client-ui-workspace/lib/types/client/tree.d.ts:63` |
| Watermark 字段 | **未验证**：没有找到公开的 watermark 数值字段；README 只描述行为（W:30），实现可能在组件局部 state | 类型声明未暴露 |
| `LayoutState` | `panelInfo.activePanelId`、`sidebar`、`viewportWidth`、`narrowExpanded`、`rightbar`、`rightbarShown`、`rightbarTrack`、`rightbarFullscreen`、`rightbarInstant` | `P/dsh-client-ui-layout/lib/types/client/stores.d.ts:11-38` |
| `PanelInfo` | `activePanelId: MainPanelId \| null`（null = Conversation） | `P/dsh-client-ui-layout/lib/types/client/service.d.ts:17` |
| `SessionStatus` | `running: boolean \| undefined`、`pendingInteraction?`、`completionUnread` | `P/dsh-client-ui-session/lib/types/client/index.d.ts:30` |
| Commands | `CommandUiSpec` 为 `popupSelect` 或 `action`；`CommandDirectory` 持有 per-session wire cache；`PopupSelectController` 持 popup 状态 | C:12、C:58 |
| Shortcuts UI store | `open`、`query`、`focusRequest` | `P/dsh-client-ui-shortcuts/lib/types/client/store.d.ts:4-6` |

未验证项（仅 README 行为，未在公开类型中确认）：watermark 的具体存储位置、AnimatedRows 的动画参数、hover card 800ms 首卡等待的计时器实现（行为本身见 W:187）、workspace tree 的持久化 key。

## 4. 给 ZeoKeel 的模块化建议

### 4.1 M1 先做什么

1. **保持 `@zeokeel/ui-sessions` 单包单 slot `ui.sessions`，作为侧栏 owner**。现状注册 6 个 slot，但 M1 active 仍只有 `sessions/chat/settings`；`ui.shell/inspector/commands` 是骨架、post-M1 接入（Z28:37-47、Z28:96-97；Z16:62-67）；`ui.shell` 尚未接入 app（Z16:65）。新增/拆分 slot 必须同步改 `tools/zk` 的 `slot_binding` 与 Z16（Z16:69），M1 不值得为侧栏再付一次生成器成本。
2. **包内拆文件，不拆包**：把当前 `web/src/index.tsx`（763 行）拆成 `SidebarShell`、`WorkspaceSection`、`SessionRow`、`WorkspaceDialogs`、`useSidebarView` 五块；对外仍只导出 `components["sessions.sidebar"]`。这是低风险的结构准备，不改变 registry。
3. **先定义纯 view model**：借鉴 `GroupNode`/`SessionNode` 的字段形状，但只保留 M1 已有事实：`Workspace{id,path,title,is_default}`、`Conversation{id,workspace_id,title,updated_at_ms}`、`activeWorkspaceId`、`activeConversationId`、`streaming`、`workspaceError`。不要把 DSH 的 `pinned/archived/completed/pendingInteraction` 填成假值。
4. **M1 交互优先级**：新会话目标规则（显式→当前→最近→无 Workspace 的空态）、Workspace 展开/折叠、搜索过滤、Workspace 重命名/移除（`reassign_to` 二次确认）、running 状态点、空/加载/错误/离线/流式中/已取消六态（Z27:37）、`ui.sessions` 的无工作区/目录不可用/切换取消流式（Z19:44）、窄窗抽屉 + Esc、焦点可见。Z26:37 已把 pin/archive/order 排除出 M1，不要提前做。
5. **复用 `@zeokeel/ui-kit`**：当前 `ui-sessions` 内部自带的 `Modal`（含 focus trap/portal）应上移为 `ui-kit` 的 Dialog 封装，与 settings/未来确认框共用（M1 保持无额外依赖，T21 再评估 Radix）；`StatusDot` 目前只有 `live` 布尔，后续需要 `running/done/pending/scheduled/idle` 五值时再扩展，不要在包内复制第二套。C17 禁止近似实现。
6. **不要复制 DSH 的 blank Session 概念**：DSH 的“空白 New Session”是真实 Session 行；ZeoKeel C18 规定无 Workspace 不建会话、首用不自动建会话（Z26:23、Z26:30）。ZeoKeel 的空态应是“选择/新建工作区”，不是一个可发送的 blank 行。

### 4.2 后置（M1.5 / T21）再拆的包与槽位

| 演进项 | 职责 | 触发/前置 |
| --- | --- | --- |
| `ui.sessions` 内子槽：`brand` / `panels` / `footer` / `settings` | 侧栏壳局部可替换（品牌、全局面板、footer、设置 seat） | 需要嵌套 slot + registry 生成器支持；M1 不做 |
| `ui.sessions.row.leading` / `.row.menu` / `.row.action` | 行前导装饰、菜单项、hover 按钮的 list seat | 出现 approval/schedule/plugin 等第二个消费者时 |
| `ui.shell.overlay` | 重命名、stop-and-archive、toast 等共享浮层 | `ui.shell` 接入 app、且浮层需要共享层级时 |
| `ui.commands`（已有 ⌘K 骨架） | 补 `/` 菜单 + 快捷键参考 + 可选全局面板；注意 `/` popup 与 `commands.palette` 是不同 surface | Z28 §4.5 暂定合并到本包；实现时分开组件/状态，不要混成一套 |
| `ui.inspector`（已有骨架） | 右栏文件/变更/终端内容 | M1 后实现右栏 |
| `ui.sidebar` 独立包（可选，最后手段） | 只拿走侧栏壳，不拿走 Workspace 数据与行行为 | 必须替换整个侧栏壳而不动浏览器时；否则会多一套实现 |

docs 28 §2 已把“命令面板/快捷键”合并为 `ui-commands`，因此不再建议单独 `ui.shortcuts` 包；快捷键参考作为 `ui-commands` 的一个 overlay surface，Settings 入口仍走 `ui.settings`。

拆分时必须同步：`zk.toml` 的 `[[ui.contributions]]`、`tools/zk` 的 slot binding、Z16 §2.2 槽位表、Z19 页面/槽位表。不要只改代码。**注意**：DSH `ui-commands` 的 popupSelect 挂在 `conversation.input.overlay`（Composer 上方），ZeoKeel 的 `commands.palette` 是 ⌘K 面板；docs 28 §4.5 暂定把 `/` 菜单、快捷键参考、可选全局面板都归 `ui-commands`，实现时必须按 surface 分开组件与状态。

### 4.3 与现有 `@zeokeel/ui-sessions` 的关系

- **M1 它是唯一 owner**：Workspace 列表、会话列表、新建、重命名、移除、设置入口都在这里；不要为了“像 DSH”再建 `@zeokeel/ui-workspace`，否则违反 Z26:146 与 C17。
- **新骨架不是替代**：`ui.shell` 是 frame（`navigation`/`inspector`/`overlay` 的容器），`ui.inspector` 是右栏，`ui.commands` 是 ⌘K 面板；它们与 `ui.sessions` 并列，不是父子。不要把 skeleton 扩成第二份 sidebar 实现。
- **M1 之后它仍应是 Workspace/Session 浏览器的 owner**；若将来拆出 `ui.sidebar`，它只拿走“壳”，不拿走 Workspace 数据与行行为。
- **状态/权限适配不要塞进 UI 包**：DSH 的 `ui-session` 是非 slot 的适配层（U:13）。ZeoKeel 若需要同样的 `useSessionStatus`/`useSessionAccess`，M1 应放在 app 的 controller/reducer 层经 props 注入；T21 若出现第二个消费者，再引入非 slot 的 session adapter 包。`ui-kit` 只放无业务的表现原语；不要让 `ui-sessions` 直接依赖 transport 或 Rust 实现（C4）。
- **替换语义不同**：DSH 是运行期 slot 声明/`slots.inject()`；ZeoKeel 是构建期 registry（Z16:10-14）。因此 DSH 的“替换 sidebar 会带走内部 seat”在 ZeoKeel 应表达为：替换 `ui.sessions` 就是替换整个侧栏；若未来要只换行，必须先把行拆成独立 slot 并进生成器。

### 4.4 事实校核（现状 vs 规范）

- Z27:35 写“侧栏 288px”，当前 `apps/desktop/web/src/app.css:477-478` 是 **248px**；Z27:39 写 ≤900px 抽屉，当前实现也是 ≤900px 抽屉（`app.css:494`）。骨架 spec 需要先定 248/288 与是否引入 56px rail，否则截图/axe 基线会漂。
- `ui.shell` 骨架尚未接入 app（Z16:65）；当前 248px/抽屉仍由 `App.tsx` + `app.css` 直排。接入 `ui.shell` 后应以 `AppShellProps`（`navigation`/`inspector`/`overlay`）重新冻结布局基线，而不是把 app 直排当成最终形态。
- 注册/骨架 vs M1 active：Z16:62-67 列 6 个注册 slot，Z19:43 写 M1 active 3 个；Z28:44-47、Z28:96-97 已裁决 M1 active 仍为 `sessions/chat/settings`，`ui.shell/inspector/commands` 只是 post-M1 骨架。当前 app 只 import `SessionsSlot/ChatSlot/SettingsSlot`（`apps/desktop/web/src/App.tsx:18`），与裁决一致。
- 当前 `ui-sessions` 的展开状态是 `useState` 本地集合（`packages/ui/ui-sessions/web/src/index.tsx:140`），没有持久化；DSH 的 group expansion 是 browser-persisted（W:44）。M1 可不持久化，但要在 spec 里写明，避免后续误判为回归。

## 5. 待确认问题

1. **post-M1 侧栏拆分**：Z28:22、Z28:44-47 已定 M1 将导航内聚到 `ui.sessions`（3 个 active slot 之一），因此 M1 不拆。post-M1 若要支持局部替换，选 B 在 `ui.sessions` 内加 `brand/panels/footer/settings` 子槽，还是 C 新增 `ui.sidebar` 包？B/C 都要改 `tools/zk` slot binding 与 Z16/Z19，建议 T21 再做，请确认。
2. **Session 级动作范围**：Z26:37 只排除了 pin/archive/order；**rename/fork/copy ID** 是否属于 M1？DSH 把它们都放在行菜单（W:56），ZeoKeel 当前只有 Workspace rename/remove。需要明确 M1 的会话行菜单最小集。
3. **状态契约**：M1 只有 `streaming`；DSH 的 `pendingInteraction`/`completionUnread`/scheduled mark 依赖 approval/schedule/job 域。是否现在就在 UI view model 预留 `status?: {running, pending?, completed?}`（不填值），还是等这些域落地再加？另外 “unread” 的定义是否等同 DSH 的“完成时未被 main view 查看”？
4. **扁平 vs 树/ungrouped**：Z26:25 明确平铺、Z26:49 排除 ungrouped，DSH 却支持 Workspace Tree 与 Ungrouped（W:44、W:56）。请确认 ZeoKeel **永久不做**树/ungrouped，还是 post-M1 以显式开关引入；这决定 Workspace 行是否需要 `parentId`/路径祖先计算。
5. **会话删除**：DSH 永不删除 Session，只能 archive（W:222）；ZeoKeel 的 `workspace.remove` 只迁移会话（Z26:34）。请确认 ZeoKeel 也采用 **archive-only、无 `session.delete`**，以及 M2 是否仍不提供物理删除/导出后删除。

---

**C15 声明**：本文只记录 DSH 的公开行为与类型事实，未复制其源码，未把 DSH 包加入 ZeoKeel 依赖、构建输入或索引；ZeoKeel 的实现必须走自己的契约/包/生成器路径。
