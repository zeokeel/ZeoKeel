# DSH Web：右栏 / 文件 / 终端 / 预览 / 设置 / 命令 / 快捷键 / 插件 / 主题（只读调研）

> 目的：给 ZeoKeel `ui-inspector`、`ui-commands`、`ui-settings`、`ui-theme` 的模块骨架提供事实依据。
> 证据根 `E=/usr/local/lib/node_modules/@deepseek-ai/dsh/node_modules/@deepseek-ai/`；下文 `<pkg>/README.md:行` 均相对 `E`。`d.ts` 仅用于组件名/状态名，未读源码，不推测实现。
> **C15**：DSH 仅作事实/设计参考，不作为 ZeoKeel 的依赖、构建输入、索引仓或数据源；不复制其代码、locale、视觉资产。本文只写这一个文件，未改任何代码/规范。
> 结论口径：DSH 的“README 级契约”与“声明类型”可验证；“未验证”处已显式标注。

## 0. 先给结论（影响 ZeoKeel 的 8 条）

1. **右栏是 per-session 的 docking surface**：每会话一份布局；无会话则无按钮、无面板；折叠不销毁 tab；关最后一个 docked tab 会连栏一起折叠（`dsh-client-ui-sidebar-right/README.md:12,57,70,142`）。
2. **“设置分页”在 DSH 里并不是 9 个同级页**：本组包贡献的 Settings 顶级页是 Account / General / Models / Built-in plugins；Subagent、Web search、Agent loop、Shell 四页挂在**侧栏 Plugins 页**的 `plugins.item` 下（`dsh-client-ui-settings-general/README.md:30,48`；`dsh-client-ui-settings-subagent/README.md:12`；`dsh-client-ui-plugin-manager/README.md:73`）。
3. **DSH 没有（本组包中可核实的）全局命令面板**：只有 composer 的 `/` 命令菜单/弹窗（`ui-commands`）与 `Mod+/` 快捷键参考（`ui-shortcuts`）；后者只浏览/录制键位，不执行命令（`dsh-client-ui-commands/README.md:12,28`；`dsh-client-ui-shortcuts/README.md:28`）。
4. **设置保存有两种节奏**：General/主题/会话日志是即时写；Models 是卡片内 Apply，Subagent/Agent loop/Shell/Web search 是页面级 Save；全部用 revision 栅栏防并发覆盖（`dsh-client-ui-settings-general/README.md:46`；`dsh-client-ui-settings-models/README.md:38,94`；`dsh-client-ui-settings-subagent/README.md:32`）。
5. **右栏宽度不是 tab 状态**：宽度/轨道由 frame 管，Seat 只收 `width/viewportWidth`；宽度把手只在“已展开 + 普通呈现”出现；窄于 768px 自动全屏（`dsh-client-ui-sidebar-right/lib/types/client/shell/SidebarRight.d.ts:112`；`dsh-client-ui-sidebar-right/README.md:39,48`）。
6. **主题 = Host 设置 + token 样式表 + 首屏 bootstrap**；`light/dark/system`、文本/代码/终端三套字号与字体；第三方主题只能覆盖同名 alias token，且没有完整性校验（`dsh-client-ui-theme/README.md:12,32,42,46,131`）。
7. **插件市场是运行期装包**（pnpm/插件 Loader，安装后重组），与 ZeoKeel C2“构建期组合、运行期不加载插件”直接冲突；Settings 的 Plugin list 只是只读清单（`dsh-client-ui-plugin-manager/README.md:42,55,63`；`dsh-client-ui-settings-plugin-inventory/README.md:12,98`；`docs/16-ui-packages.md:11`；`docs/27-ui-design-system.md:64`）。
8. **DSH 没有“变更/Changes”tab**：右栏只有 guide/files/text/terminal/browser 等类型；ZeoKeel 的“变更”是自有模块，不能从 DSH 找现成交互（`dsh-client-ui-sidebar-files/README.md:29`；`dsh-client-ui-sidebar-documentpreview/README.md:31`；`dsh-client-ui-sidebar-terminal/README.md:46`；`dsh-client-ui-sidebar-browser/README.md:28`）。

## 1. 模块清单与组件树

### 1.1 右栏（docking surface / tab 生命周期）

| 事实 | 证据 |
|---|---|
| 每会话一个 docking surface；两种呈现 normal/fullscreen；切换呈现不重挂 tab | `dsh-client-ui-sidebar-right/README.md:12,39` |
| 没有面板头行；呈现切换与折叠按钮挂在 dockkit chrome 的 tab strip 末端 | `dsh-client-ui-sidebar-right/README.md:50` |
| 面板收起时，入口是会话 header corner 的展开按钮；无会话则没有按钮/面板 | `dsh-client-ui-sidebar-right/README.md:57,142` |
| 根级 `rightbar` 按会话渲染 `rightbar.session` 子树；背景会话保留 `keepMounted` body，但不报告 track | `dsh-client-ui-sidebar-right/README.md:61,106` |
| tab 类型两段注册：类型 `ctx.sidebarRightTabs.register({id,kind,patterns,priority,canOpen,title,guide,keepMounted})` + body `sidebar.right.pane.tab`（keyed by id） | `dsh-client-ui-sidebar-right/README.md:83-84` |
| 资源按 glob/canOpen/priority band 排名；`extension > builtin > fallback`，再按匹配长度/注册顺序 | `dsh-client-ui-sidebar-right/README.md:86` |
| 页面类型按 kind 打开；guide 是内置页面，guide 条目可打开各类型 | `dsh-client-ui-sidebar-right/README.md:83,115,119` |
| 默认页规则：恰好 1 个 guide entry 直接开该页；0/多个则开 guide；guide 只能存在于每 pane 一个 | `dsh-client-ui-sidebar-right/README.md:119,121` |
| 面板初始折叠且为空；首次展开才播种默认页；关闭最后 tab 连栏折叠 | `dsh-client-ui-sidebar-right/README.md:70`；`dsh-client-ui-sidebar-right/lib/types/client/stores.d.ts:74-80` |
| 唯一不可关闭的是“唯一 docked guide”；其他唯一 tab 关闭时连栏折叠 | `dsh-client-ui-sidebar-right/README.md:72,119` |
| 最多两个水平 pane，初始等分，分隔比例限 20%–80%；宽度不足阻止 split | `dsh-client-ui-sidebar-right/README.md:121` |
| 可 float/dock；浮动面板不受折叠影响 | `dsh-client-ui-sidebar-right/README.md:48,72`；`dsh-client-ui-sidebar-right/lib/types/client/stores.d.ts:104-108` |

组件树（名称来自 README/d.ts，非源码结构）：

```text
RightbarRoot                         root-scoped；按 session 拆 rightbar.session
└─ RightbarSeat (per session)        width/viewportWidth/canShow；报告 shown/track/fullscreen
   ├─ dockkit chrome                 tab chips + close + add + split + panel controls（无 header）
   ├─ sidebar.right.pane.tab         keyed body（guide/files/text/terminal/browser…）
   │  ├─ guide                       内置；可用 sidebar.right.tab.guide 链替换
   │  ├─ files                       FilesBody + PathLabel + reload + open-in-app
   │  ├─ text                        documentpreview body + renderer registry
   │  ├─ terminal                    xterm 懒加载 chunk
   │  └─ browser                     BrowserFrame → IframeImpl / ElectronWebViewImpl
   └─ sidebar.right.pane.tab.title   keyed title（可选）
ExpandButton                         面板隐藏时在 conversation.session.header.corner
```

证据：`dsh-client-ui-sidebar-right/lib/types/client/shell/RightbarRoot.d.ts:17`、`dsh-client-ui-sidebar-right/lib/types/client/shell/SidebarRight.d.ts:112`、`dsh-client-ui-sidebar-right/lib/types/client/contract/slots.d.ts:53,67,78`；`dsh-client-ui-sidebar-files/README.md:33`；`dsh-client-ui-sidebar-documentpreview/README.md:32,35`；`dsh-client-ui-sidebar-terminal/README.md:46`；`dsh-client-ui-sidebar-browser/README.md:69,71`。

### 1.2 文件树 / 文档预览 / 终端 / 浏览器

| 模块 | 注册与组成 | 关键事实 |
|---|---|---|
| 文件树 | kind `files`，无 patterns，1 个 guide entry（order 10，黄色文件夹） | 根 = 会话 cwd；每层首次展开才 `workspaceFiles.list`；目录/文件/其他三行型；目录优先、自然大小写不敏感排序；dotfiles 照常显示（`dsh-client-ui-sidebar-files/README.md:29,38,40`） |
| 文件树 body | 顶部 PathLabel + reload + `sidebar.right.tab.files.actions`；chip title 用 FileTypeIcon | 文件行用 `dsh-resource://file/session/<sid>/<path>` 打开预览；其他行灰显不可点（`dsh-client-ui-sidebar-files/README.md:30-31,43-44`） |
| 文档预览 | kind `text`，whole-address 文件 pattern，band `fallback`；仅接受 Session 地址 | 固定 header：Host 绝对路径或请求路径 + PathLabel + renderer 下拉 + wrap + reload；两个 action 子槽（header/unsupported 空态）（`dsh-client-ui-sidebar-documentpreview/README.md:31-32`） |
| 预览 renderer | `ctx.documentPreviews.register({id,extensions,binaryExtensions,priority,title,loading,wrap})` + keyed `sidebar.right.tab.document` | `loading: text-pages / bytes-complete / renderer`；renderer 排名 extension > builtin > 更长后缀 > 注册顺序（`dsh-client-ui-sidebar-documentpreview/README.md:35,37`） |
| 预览内容 | Markdown/code 分页文本；PDF/HTML/图片/表格整字节；Office 本地转 PDF | 文本先读第一页，滚到底/ Load more 续读；字节有 Host `maxFileBytes` 上限；HTML 有静态净化与脚本 Blob 两种策略（`dsh-client-ui-sidebar-documentpreview/README.md:49,56-58,60,62,96`） |
| 终端 | kind `terminal`，guide 卡片 + shell 菜单 | 菜单打开时才探测已装 shell；新建/重命名/接管输入/断线重连/退出码；关闭 tab 请求终止进程；折叠/切换/浮动保留进程（`dsh-client-ui-sidebar-terminal/README.md:28,30,32,34`） |
| 终端恢复 | 内容身份 ↔ Host 关联由 terminal controller 保存；布局由右栏保存 | reload 后按保存的 Host identity 重连；缺失进程显示 unavailable + New terminal；不自动重启（`dsh-client-ui-sidebar-terminal/README.md:36,48,50`） |
| 浏览器 | kind `browser`；Web 用 iframe，Desktop 用 Electron `<webview>` | Web 默认禁用、Desktop 默认启用；HTTP/HTTPS + loopback；默认 sandbox；Desktop tab `keepMounted`（`dsh-client-ui-sidebar-browser/README.md:12,28,63,71`） |

浏览器工具栏：Back/Forward/Reload/Go/Open in system browser；Web 另有 per-tab sandbox 开关（临时、不持久），Desktop 显示页面标题；重启后显示保存的 title/URL，Restore/Reload 才打开（`dsh-client-ui-sidebar-browser/README.md:47,108`）。

### 1.3 设置：实际分页与组件树

**重要纠正**：DSH Settings 的导航来自 `settings.section` ledger，Account 登录后第一；General/Models/Built-in plugins 是 Settings 页。四个“官方配置页”不在此列，而在侧栏 Plugins 页的 `plugins.item` 槽（`dsh-client-ui-settings-general/README.md:30,48,72`；`dsh-client-ui-settings-subagent/README.md:12,42`；`dsh-client-ui-plugin-manager/README.md:73,119`）。

| 页面/入口 | 归属 | 事实 |
|---|---|---|
| Account | `ui-settings-account` | 只在已登录时出现且第一；资料/余额/登录/登出/充值；头像 32px（Settings）/24px（侧栏）（`dsh-client-ui-settings-account/README.md:16,33,69`） |
| General | `ui-settings-general` + 贡献行 | Coding Tools、Current version、Appearance/字体（ui-theme）、Session log、快捷键入口、Open configuration file（仅 loopback）（`dsh-client-ui-settings-general/README.md:42-52`；`dsh-client-ui-settings-session-log/README.md:12`；`dsh-client-ui-shortcuts/README.md:28`） |
| Models | `ui-settings-models` | provider 行/卡片、API key（write-only，走 credentials 域）、模型列表、自定义 pi-ai route、首启 DeepSeek 引导；一次只开一个 editor（`dsh-client-ui-settings-models/README.md:14,34,42,46,56,64,68`） |
| Built-in plugins | `ui-settings-plugins` + `ui-settings-plugin-inventory` | Settings 内只读清单；只有一个 tab 时直接作为页面；tab 首次选中后保持挂载（`dsh-client-ui-settings-plugins/README.md:12,28,30,40`；`dsh-client-ui-settings-plugin-inventory/README.md:12,28,98`） |
| Subagent | `ui-settings-subagent` | 在**侧栏 Plugins** 的 Built-in plugin settings；两个 namespace 一页一 Save（`dsh-client-ui-settings-subagent/README.md:12,28,32`） |
| Web search | `ui-settings-web-search` | 同上；API 切换、key/endpoint/limit；key 走 credentials 域（`dsh-client-ui-settings-web-search/README.md:12,28,30`） |
| Agent loop | `ui-settings-agent-loop` | 同上；只编辑 parallel tool calls 一个字段（`dsh-client-ui-settings-agent-loop/README.md:12,28,67`） |
| Shell | `ui-settings-shell` | 同上；command timeout + output cap（`dsh-client-ui-settings-shell/README.md:12,28,67`） |
| Session log | `ui-settings-session-log` | 实际是 General 里的一行开关，不是独立页（`dsh-client-ui-settings-session-log/README.md:12`） |

```text
SettingsRoot (ui-settings-general)
├─ trigger: 侧栏底部 Settings 控件 / account launcher 菜单；Disconnected/Connected 指示
├─ overlay panel 800×800（viewport 约束；portal 在 #root 旁）
│  ├─ nav = settings.section ledger（可独立滚动）
│  ├─ AccountSection（仅登录后，第一）
│  ├─ GeneralSection
│  │  ├─ DeveloperToolsRow / CurrentVersionRow
│  │  ├─ AppearanceRow + font size stepper + FontSettingsGroup（ui-theme）
│  │  ├─ Session log 行；Shortcuts 入口
│  │  └─ settings.general.item 其他贡献
│  ├─ ModelsSection
│  │  ├─ ProviderRow / ProviderCard / ProviderEditor
│  │  ├─ ApiKeyInput / ModelListEditor / ModelReasoningEfforts
│  │  ├─ CustomProviderCard + 添加模式切换
│  │  └─ Onboarding/Welcome 对话框
│  └─ PluginsSection → settings.plugins.tab → Plugin list (read-only inventory)
├─ HostReleasePrompt（Service version changed）
└─ onboarding ledger（一次只挂一个 step）
```

证据：`dsh-client-ui-settings-general/README.md:25,30,40,48,72`；`dsh-client-ui-settings-models/lib/types/client/ModelsSection.d.ts:1-24`；`dsh-client-ui-settings-models/lib/types/client/ProviderEditor.d.ts:1`；`dsh-client-ui-settings-models/lib/types/client/ApiKeyInput.d.ts:1`；`dsh-client-ui-settings-models/lib/types/client/ModelListEditor.d.ts:1`；`dsh-client-ui-settings-models/lib/types/client/CustomProviderCard.d.ts:1`；`dsh-client-ui-settings-plugins/README.md:40`。

### 1.4 命令面板与快捷键（两条不同入口）

```text
ui-commands（composer 内）
  `/` source → trigger menu
    ├─ Add: File / Goal / Plan / Feedback
    └─ Commands: Compact / Permission / Model / Export
  → CommandDirectory（per-session 目录）
    ├─ host descriptor + input → leadingInput
    ├─ CommandUiSpec popupSelect → PopupSelectView
    ├─ CommandUiSpec action → 客户端回调
    └─ 其他 → execute（host command.execute）

ui-shortcuts（应用层）
  General 行 / Mod+/ → “Keyboard shortcuts” 对话框
    → 搜索 catalog（动作/英文别名/键）
    → 内联 recorder（录制/清除/恢复默认）
    → 持久化（Web localStorage / Desktop file）
```

事实：`/` 命令永远不会静默降级为普通 prompt；Space/Enter 按会话目录解析为 `leadingInput`/`popupSelect`/`action`/`execute`（`dsh-client-ui-commands/README.md:12`）。`+` 与手输 `/` 打开同一个菜单；面板内 ↑/↓/Enter/Tab/Escape/Shift+Tab 有明确归属（`dsh-client-ui-commands/README.md:28`）。命令贡献与装饰不能静默覆盖 host 命令；装饰只改裸调用 UI，保留 catalog 行/参数声明/日志（`dsh-client-ui-commands/README.md:32`）。快捷键对话框 480×600，搜索按有序子序列匹配；录制支持 Windows/macOS Desktop 的一到两个非修饰键（第三键拒绝）（`dsh-client-ui-shortcuts/README.md:28,34,36`）。

### 1.5 主题

```text
ui-theme
├─ ThemeRuntime → ThemeSnapshot（不可变）
├─ ui-layout presenter → 应用到 document
├─ --dsw-* token 样式表（base/corner/design-platform/focus/scrollbar/…）
├─ Settings General：AppearanceRow + text/code/terminal 字号/字体行
└─ 首屏 bootstrap：index head/body 脚本在应用脚本前写入 palette/size/font
```

事实：用户可选 `light/dark/system`；`system` 用 `prefers-color-scheme` 解析；设置存 `ui-theme` namespace，默认由本地 provider 写入 `$DSH_HOME/cordis.patch.yml`（`dsh-client-ui-theme/README.md:12,28,96`）。文本字号 10–22（默认 14），代码 10–16（默认 11），终端 10–20（默认 13）；字体行接受逗号分隔 family，空值恢复内置栈（`dsh-client-ui-theme/README.md:32,38`）。第三方主题通过 `ctx.theme` 注册 alias-token 覆盖，按注册顺序折叠；移除不覆盖最后的持久内置偏好；没有“覆盖是否完整”的校验（`dsh-client-ui-theme/README.md:42,131`）。

### 1.6 插件管理 / 插件市场

| 模块 | 位置 | 事实 |
|---|---|---|
| Plugins 页 | 侧栏全局页（不属于 Session） | 管理 profile 的 bundle 与 row；安装/卸载/开关；官方配置页在 `plugins.item`；详情页有 `plugins.detail.*`、`plugins.tab`、`plugins.row.config` 等槽（`dsh-client-ui-plugin-manager/README.md:14,30,73,87,101,119`） |
| Install | Add plugin → 校验 spec → pnpm → 进度/取消/恢复 → 启用 | Host 读 spec，pnpm 输出可见；一次只跑一个安装；应用后可能提示 restart（`dsh-client-ui-plugin-manager/README.md:42-61`） |
| 开关 bundle/row | 改 profile 的 `cordis.patch.yml` | HMR profile 立即重组；无 HMR/被更高层覆盖时 toast 说明（`dsh-client-ui-plugin-manager/README.md:63-69`） |
| Discover（market） | Plugins 页第二个 tab，Installed 之后 | 第三方 index sources；搜索/分类/label/排序；条目详情与版本列表；确认后复用安装对话框；无 upgrade 动作（`dsh-client-ui-plugin-market/README.md:12,27-37,61`） |

## 2. 交互

### 2.1 右栏打开 / 关闭 / 宽度 / 窄窗

- **打开**：`openResource(address, options?)` 或 `openTab(kind, options?)`；两者都会在同一步展开面板（“用户看不见的内容不算打开”）；资源 tab 默认按 `(kind, contentId)` 去重，可用 `revealIfOpened:false` 允许重复；页面 tab 在每个 pane 内总是去重（`dsh-client-ui-sidebar-right/README.md:93`）。
- **落点**：`replaceTab` 优先 → `paneId` → 当前 active docked pane；`preferNewPane` 先尝试 split，失败回退目标 pane（`dsh-client-ui-sidebar-right/README.md:93,95`）。
- **关闭**：`close(tabId)`；唯一 docked guide 不可关；其他唯一 tab 关闭会连栏折叠；折叠/呈现切换/插件卸载不触发 close handler，只有记录移除/插件卸载才 abort tab signal（`dsh-client-ui-sidebar-right/README.md:72,108`）。
- **宽度**：普通 `push` 模式由 conversation 让位；fullscreen 覆盖 viewport 但保留宽屏 columns；宽度把手只在“展开的 normal 模式”出现；split 分隔比例 20%–80%，最多两个 pane，宽度不足阻止 split（`dsh-client-ui-sidebar-right/README.md:39,43-44,48,121`）。**默认宽度/最小宽度 README 未给出（未验证）**。
- **窄窗让位**：<768px 打开自动 fullscreen；在窄窗退出 fullscreen 会关闭面板；重新变宽不会自动重开已关闭面板；窄窗自动 fullscreen 不占 track（`dsh-client-ui-sidebar-right/README.md:39,44`）。
- **无会话**：没有 header 按钮也没有面板；背景会话的 tab 状态保留但不显示 track（`dsh-client-ui-sidebar-right/README.md:57,61,142`）。
- **快捷键**：右栏 toggle/split/fullscreen 都在快捷键参考里；split/fullscreen 要求焦点在可见 dock pane 内，对话焦点时不可用；不可用时 split 仍可见并给出原因（`dsh-client-ui-sidebar-right/README.md:55`）。

### 2.2 文件树与预览

- **文件树**：根是 session cwd；层级首次展开才拉取，失败按 `workspace-file/not-found`、`outside-workspace`、`not-directory` 或 transport 消息显示；折叠时缓存已显示条目，重开再拉；reload 只刷新根+已展开层，保留当前显示；空目录/截断/无 cwd 各有专门文案（`dsh-client-ui-sidebar-files/README.md:38,46`）。
- **自动刷新**：默认开启，直接监听已打开目录；目录读取中收到的变更留到下一轮；折叠节点关闭 watch 和隐藏后代（`dsh-client-ui-sidebar-files/README.md:50`）。reload 控件在 hover/focus 显示有效快捷键（`dsh-client-ui-sidebar-files/README.md:75`）。
- **文件→预览**：文件行用 `useTabInfo().tab.actions.openResource(fileAddress)` 落到本 tab 所在 pane；文档预览只接受 Session 地址，裸 absolute 不认领；内容身份 = 整个地址，所以同名不同目录/不同会话是不同 tab（`dsh-client-ui-sidebar-files/README.md:43`；`dsh-client-ui-sidebar-documentpreview/README.md:31,42`）。
- **预览**：header 有路径、renderer 下拉（多实现时）、wrap（选中 renderer 声明 `wrap` 时，默认开）、reload；自动刷新默认开但开关隐藏；文本分页，滚到底或 Load more；字节模式整读；换 renderer 会重读但不改变导航地址（`dsh-client-ui-sidebar-documentpreview/README.md:32,35,56-58`）。
- **导航**：`openResource(address,{params:{line}})` 传 1-based 行号；text-pages 会继续读直到该行/EOF；plain-text/code 有行锚点，Markdown 没有；byte renderer 不消费行导航；每次 navigation revision 只答复一次（`dsh-client-ui-sidebar-documentpreview/README.md:126`）。
- **失败/边界**：不支持预览显示文件类型图标 + 说明 + 可选 `sidebar.right.tab.document.unpreviewable`；已知二进制后缀不发读取；只读、不编辑；未知后缀走 plain text 并受 UTF-8/NUL 检查（`dsh-client-ui-sidebar-documentpreview/README.md:32,140`）。

### 2.3 终端与浏览器

- **终端**：New terminal 立即打开“记住的可用 shell”；标题旁箭头打开已装 shell 菜单，探测在菜单打开时进行且不分配终端；失败可 Retry；双击 tab 标题重命名；Take control 在别的页面持有输入时接管；临时断线保留屏幕 + Reconnect；退出后保留 exit code，不自动重启；退出终端也计入会话上限。命令以执行环境的系统用户权限运行，独立于 Agent 权限（`dsh-client-ui-sidebar-terminal/README.md:12,30,32`）。
- **终端生命周期**：关闭/替换 tab 立即移除并后台结束进程；失败无通知/手动重试，保存的未完成关闭请求在 client 插件启动时重试；折叠/切 tab/切 session/float/fullscreen 保留进程；reload 后按保存的 Host identity 在原 tab 重连；缺失进程显示 unavailable + New terminal，点击原位替换（`dsh-client-ui-sidebar-terminal/README.md:34,36`）。
- **浏览器**：地址栏接受 HTTP/HTTPS（含 loopback），拒绝 `file:`/script/data/blob/带凭据/DSH 自身 origin；无 scheme 补 HTTPS；Web iframe 默认 sandbox（脚本/表单/同源/popup），工具栏可临时移除 sandbox；Web 用应用管理的 history，未知 URL 状态会禁用 Back/Forward/external-open，Reload 回最后受控 URL；Desktop 用原生 Chromium history，tab `keepMounted`（`dsh-client-ui-sidebar-browser/README.md:59,63,65,71`）。
- **浏览器恢复**：保存 title/URL；重启后只显示保存值，Restore/Reload 才真正打开；关闭 tab 删除 checkpoint；不恢复页面内存/表单/Chromium history（`dsh-client-ui-sidebar-browser/README.md:47,108`）。

### 2.4 设置：保存 / 校验 / 冲突 / 重启语义

| 类型 | 行为 | 证据 |
|---|---|---|
| 即时写 | General 的 Coding Tools、主题/字号、Session log 开关：接受即写，重复输入禁用，失败就地重试/提示 | `dsh-client-ui-settings-general/README.md:46`；`dsh-client-ui-theme/README.md:32`；`dsh-client-ui-settings-session-log/README.md:28` |
| 卡片 Apply | Models：草稿在卡片内，Apply 后快照 ready 才显示保存状态；失败保留 editor + Host 诊断；并发写用 revision 栅栏，冲突保留草稿 | `dsh-client-ui-settings-models/README.md:38,94` |
| 页面 Save | Agent loop / Shell / Web search / Subagent：只 Save 才写；离开页面丢草稿；非 secret 字段清空=恢复默认（Web search 的 API key 空白则保留已存 key）；非数字/越界就地报错 | `dsh-client-ui-settings-agent-loop/README.md:28`；`dsh-client-ui-settings-shell/README.md:28`；`dsh-client-ui-settings-web-search/README.md:28,30`；`dsh-client-ui-settings-subagent/README.md:28,32` |
| 双 namespace | Subagent 的两个 section 各自 revision 栅栏、独立写入；一个被拒另一个仍可落地；模型草稿被更新 revision 取代时报 conflict | `dsh-client-ui-settings-subagent/README.md:32` |
| 校验 | 浏览器用序列化 Config schema 校验；Host 校验完整配置（含不可序列化检查）；`settingsSchema` 负责 rehydrate/validate/nested edit；wire 非法值不替换已接受值 | `dsh-client-ui-settings/README.md:48,72` |
| 并发 | `settings/document-updated`、`connection/reset` 刷新共享 mirror；写请求带 namespace revision；冲突以 Host 拒绝/重新读取处理 | `dsh-client-ui-settings/README.md:64`；`dsh-client-ui-settings-models/README.md:94` |
| 重启/重载 | 未发现“设置项保存后需重启”的通用提示；有的是 Host 版本变化对话框（Service vs Page，Reload to load the new version）与插件安装/更新后的 restart toast | `dsh-client-ui-settings-general/README.md:34`；`dsh-client-ui-plugin-manager/README.md:36,65,125` |

**给 ZeoKeel 的提醒**：`docs/19-client-ux-flows.md:49` 要求 settings 覆盖“保存中/失败/需重启/需重登”，但 DSH 这套 README 没有给出通用矩阵；ZeoKeel 必须在 spec 里按设置项定义（尤其 `server_url` 已明确“保存后需重登”，`docs/19-client-ux-flows.md:88`）。

### 2.5 命令与快捷键

- **命令来源**：host `commands.list` 的 per-session 目录 + 客户端 `ctx.commandUi` 贡献/装饰；目录缓存按 session，forwarded catalog/connection 事件使缓存失效；未保留的 session 或打开失败的 session 不发 RPC（`dsh-client-ui-commands/README.md:58,60`）。
- **执行**：`popupSelect` 只负责选值，选中后由业务 `onSelect` 决定是否回写/执行；`action` 只消费触发 token 并跑客户端回调，不提交消息；`execute` 走 host `command.execute`；带附件的提交只有声明 `input.attachments` 的 host command 可通过（`dsh-client-ui-commands/README.md:32,46,82`）。
- **快捷键记录**：录制内联在行上；Windows/macOS Desktop 接受 1–2 个非修饰键 + 0–4 修饰键；顺序按 A 再 B 只记 A；第三键拒绝；与可编辑/固定动作重叠即拒绝；保存/取消焦点回到对话框；Remove 清空 override，Restore Default 只移除该命令的 override（`dsh-client-ui-shortcuts/README.md:36`）。
- **冲突/存储**：命令目录/配置变更会使打开的录制草稿失效；Web 读失败禁用编辑并指出 `dsh.keybindings.v1`，Desktop 指 `userData/keybindings.json`；跨 tab 写 last-writer-wins（`dsh-client-ui-shortcuts/README.md:36,38`；`dsh-client-shortcuts/README.md:34`）。
- **打开/关闭**：`Mod+/` 在无 modal 或 Settings 在前时打开参考、参考在前时关闭；参考在前时 Settings 命令不能开关 Settings。Settings 命令为 Desktop `Mod+,`、Web `Mod+Alt+,`；Escape 关最上层并把焦点还给触发控件（`dsh-client-ui-settings-general/README.md:40`；`dsh-client-ui-shortcuts/README.md:30`）。
- **固定动作**：`registerFixed()` 只读、不可录制、参与冲突检查；`stopSequenceMs` 控制双 Escape 间隔（默认 500ms，1–2147483646），由 Host 设置校验后页面加载时采用（`dsh-client-shortcuts/README.md:32`）。

### 2.6 主题切换

- 选择 `light/dark/system` 后由 ThemeRuntime 解析成不可变 snapshot，ui-layout 应用到 document；`system` 跟随 `prefers-color-scheme`；首次绘制由 host 注入的 head/body bootstrap 提前应用 palette/字号/字体，避免先渲染默认再闪（`dsh-client-ui-theme/README.md:12,46,96`）。
- 字号/字体改动即时写 Host 设置；快速连续改动按 gesture 顺序串行、带 namespace revision；最新写被拒则回读持久值（`dsh-client-ui-theme/README.md:32`）。
- 终端主题/字体是消费方：背景、默认文字、光标、选择色跟 DSH theme；主题变化保留运行中的 shell/输出/OSC 覆盖；重置命令回到当前 DSH 默认（`dsh-client-ui-sidebar-terminal/README.md:38`）。
- 第三方主题 = alias-token 覆盖扩展点，不是产品功能；token 表是唯一颜色权威，缺值不补（`dsh-client-ui-theme/README.md:131-132`）。

## 3. 数据 / 状态（README 可推断；未验证处标注）

| 对象 | 形态 | 证据 | 备注 |
|---|---|---|---|
| 右栏 surface | `SurfaceState { layout, history, minted }`；`SidebarRightState { bySession }` | `dsh-client-ui-sidebar-right/lib/types/client/stores.d.ts:33-48` | `minted` 让录制的操作序列可重放；每个 action 一条 history |
| 右栏持久化 | 每 session 的 localStorage key `dsh.sidebar-right.v1.<sessionId>`：layout、tab identities、selection（选中 tab/pane）、split ratios、floating rects、presentation、identity counter | `dsh-client-ui-sidebar-right/README.md:74` | 读取时校验字段类型/节点成员/选择/id 计数/单 pane 或双水平 pane；坏数据只清该 session key |
| 右栏宽度 | 不在 `SurfaceState`；`RightbarSeat` 收 `width/viewportWidth`，frame 管 track 与 resize handle | `dsh-client-ui-sidebar-right/lib/types/client/shell/SidebarRight.d.ts:112`；`dsh-client-ui-sidebar-right/README.md:48` | 默认/最小宽度与宽度持久化位置未验证 |
| 右栏 tab | `TabRecord`（dockkit 定义）；`id/kind/contentId/title` 等 | `dsh-client-ui-sidebar-right/README.md:83,93`；`dsh-client-ui-sidebar-right/lib/types/client/contract/slots.d.ts:113,142,169` | 具体 `TabRecord` 字段未展开（未验证） |
| 右栏导航 | `navigation { address, params, revision }`；`revision` 每次导航 +1 | `dsh-client-ui-sidebar-right/lib/types/client/contract/slots.d.ts:113-126` | `params` 是 JSON-shaped，运行时不校验（`dsh-client-ui-sidebar-right/README.md:93`） |
| tab 生命周期 | `tab.signal` 只在记录移除/插件卸载时 abort；`tab.visible` 区分前台/背景 | `dsh-client-ui-sidebar-right/lib/types/client/contract/slots.d.ts:169-190`；`dsh-client-ui-sidebar-right/README.md:104` | 折叠/切 session 不 abort；`keepMounted` 保留已访问 body |
| 文件树 | `root`、`levels`（每绝对路径 loading/ready/failed）、`expanded`、`autoRefresh`、`scrollTop` | `dsh-client-ui-sidebar-files/README.md:48` | 按 tab id 分桶；store 比 body 活得久 |
| 文件树节点 | `directory/file/other`；entry 带截断 flag；child path = parent + `/` + name | `dsh-client-ui-sidebar-files/README.md:38,40` | 无搜索/过滤/拖拽/重命名/右键/当前文件高亮（README:64） |
| 预览 | 累积 pages/bytes、read/observed versions、pending Resource changes、autoRefresh、loading/failure、renderer、scroll、wrap、answered navigation revision | `dsh-client-ui-sidebar-documentpreview/README.md:33` | 字节不持久化；reload 只重读当前 tab |
| 终端 | 内容身份 ↔ Host identity 由 terminal controller 保存；布局由右栏保存 | `dsh-client-ui-sidebar-terminal/README.md:48` | 进程不在 UI 状态；reload 重连；缺失进程不自动替换 |
| 浏览器 | 每 tab `BrowserController`（地址校验/命令/恢复）+ `BrowserFrame`（导航状态）+ 物理载体 | `dsh-client-ui-sidebar-browser/README.md:69` | 保存 title/URL；sandbox 开关不持久（README:47） |
| 设置 form | `ConfigFormSnapshot { status, value, base, user, revision, writable }`；`set/unset/mutate(ops, expectedRevision?)` | `dsh-client-ui-settings/lib/types/client/config-form-types.d.ts:6-62` | `user` 的“字段存在”才是 overridden；值为空不等于未覆盖 |
| 设置 schema | `ctx.settingsSchema` rehydrate/validate/nested edit；Host 校验完整配置；浏览器校验序列化 Config schema | `dsh-client-ui-settings/README.md:48,72` | 实际 JSON Schema 文件未读取（未验证） |
| 命令项 | `CommandContribution { name, label?, description?, icon?, available(session), ui }`；`ui` = `PopupSelectSpec` 或 `ActionSpec` | `dsh-client-ui-commands/lib/types/client/contract.d.ts:58,75,92` | host descriptor 的完整字段未读取（未验证） |
| 命令弹窗 | `SelectOption { id,label,badge?,detail?,group?,active?,confirmation? }`；`searchMode` substring/fuzzy-label | `dsh-client-ui-commands/lib/types/client/contract.d.ts:26,58` | 分组是 sticky；`active` 决定初始高亮 |
| 快捷键 | 每命令每 profile（desktop/web × macos/windows/linux）默认；override 只存差异，`null` 清空，恢复默认删 override | `dsh-client-shortcuts/README.md:30,34` | Web `dsh.keybindings.v1`；Desktop v1/v2 + `secondCode` |
| 主题 | `ThemeSettings { preference, fontSize, codeFontSize, terminalFontSize, textFontFamily, codeFontFamily, terminalFontFamily }` | `dsh-client-ui-theme/lib/types/theme-settings.d.ts:86-100` | namespace `ui-theme`；preference 字段 `preference` |

## 4. 给 ZeoKeel 的模块拆分建议

### 4.1 建议边界（对齐 docs/27 §3 + docs/28 骨架）

| 建议模块 | 职责 | 从 DSH 可借鉴的事实 | 不建议照搬 |
|---|---|---|---|
| `@zeokeel/ui-shell`（`docs/27-ui-design-system.md:52`；`docs/28-ui-module-map-and-skeleton.md:21,28`） | AppShell + header/status/overlay + 主题 token | 单一不可变 snapshot；Host 设置持久化；首屏 bootstrap；token 消费 | DSH `--dsw-*` 命名/视觉；把主题做成独立运行期包 |
| `@zeokeel/ui-inspector`（`docs/27-ui-design-system.md:53`；`docs/28-ui-module-map-and-skeleton.md:25,40`） | M1 固定宽度单栏 + tabs 文件/变更/终端 + 折叠；后续真实文件树/终端 | 两段 tab 注册、per-session surface、collapse 不销毁、close handler、文件树懒加载/终端恢复 | M1 就做 per-session 多 pane/split/float/fullscreen；dockkit、localStorage key、Electron webview |
| `@zeokeel/ui-commands`（`docs/27-ui-design-system.md:54`；`docs/28-ui-module-map-and-skeleton.md:26,41`） | 全局 overlay 骨架；M1 ⌘K + 命令列表 + 键盘提示 | composer `/` 菜单、`leadingInput/popupSelect/action/execute` 四路、`Mod+/` 快捷键参考 | 声称 DSH 有全局 ⌘K；DSH 事实只支持 `/` + 快捷键参考；⌘K 是 ZeoKeel 产品增强 |
| `@zeokeel/ui-settings`（`docs/27-ui-design-system.md:51`；`docs/28-ui-module-map-and-skeleton.md:27,42`） | M1 单 dialog 内部分节：provider/base_url/model/key、主题、语言、审批只读 | 即时写 vs 卡片 Apply vs 页面 Save；revision 冲突；Host 最终校验 | DSH 的 Settings/Plugins 双层分页；运行期插件安装器 |
| 主题（**不单独建 `ui-theme` 包**）（`docs/27-ui-design-system.md:52`；`docs/28-ui-module-map-and-skeleton.md:28`） | token/`data-theme` 放 app 或 ui-shell；M1 light/dark/system | Host bootstrap + ThemeSnapshot + token 消费；终端跟随 | 把 DSH `ui-theme` 拆成 ZeoKeel 独立包；照搬 `--dsw-*` |
| 插件清单/市场 | 后续只读 built-in 清单；M2 企业商店 | Settings 只读 inventory；Plugins 页可管理是运行期安装 | **不要**移植 runtime plugin install/market（违反 C2） |

### 4.2 M1 顺序（按 docs/28 骨架；先处理文档冲突）

**当前文档状态**：`docs/16-ui-packages.md:24-26,58-69` 已列 `ui-shell/ui-inspector/ui-commands`，并写明生成器支持 `ui.sessions/chat/settings/shell/inspector/commands`；`docs/27-ui-design-system.md:52-54` 已列三包；`docs/28-ui-module-map-and-skeleton.md:37-45,62-68,86-91` 已冻结骨架验收（空壳 + mock + 截图，不做真实功能）。

**冲突（spec 前必须由文档 owner 统一，C19）**：`docs/16-ui-packages.md:12` 仍写 `App.tsx` 只取 `SessionsSlot/ChatSlot/SettingsSlot`，`:13-14` 仍写 M1 不使用 `ui-shell`；`docs/19-client-ux-flows.md:61` 仍冻结“M1 仅实现三个 slot”。这与 `docs/16` §2 / `docs/27` / `docs/28` 的六 slot 骨架冲突。本报告按 docs/28 的骨架给建议，不替 owner 选边。另：`docs/28-ui-module-map-and-skeleton.md:82` 写“设置分 Account/General/Models + Plugins 子页”不够精确——Settings 的 Plugins 是只读 Built-in plugins，四个官方配置页在侧栏 Plugins；建议 spec 前修正。

| 优先级 | 做什么 | 为什么 |
|---|---|---|
| **M1 骨架** | `ui-shell` AppShell 空壳；`ui-inspector` InspectorPanel（3 个固定 tab + `⌘B` 折叠 + 窄窗收起 + mock 文件/变更/终端）；`ui-commands` CommandPalette（`⌘K` + 命令列表 + 键盘提示 + mock）；`ui-settings` 单 dialog 分节；主题 token 进 app/ui-shell | docs/28 §3/§4/§5；M1 只做可点击骨架，不接后端（`docs/28-ui-module-map-and-skeleton.md:44,62-68,86-91`） |
| **M1.x 真实交互** | 文件树懒加载/刷新/watch；文本/Markdown/代码预览；终端 tab（mock→真实 PTY）；设置保存/校验/冲突状态 | DSH 最成熟、依赖最少；先不做 split/float/browser/PDF/Office（`dsh-client-ui-sidebar-files/README.md:38,46,50`；`dsh-client-ui-sidebar-terminal/README.md:30,34,36`） |
| **post-M1** | 变更/diff tab、per-session 多 pane/split/float/fullscreen、浏览器、PDF/Office/Excel 富预览、完整快捷键录制 | “变更”DSH 没有（`docs/28-ui-module-map-and-skeleton.md:77`）；DSH 的 per-session 多 pane 是 post-M1 目标（`docs/28-ui-module-map-and-skeleton.md:80-81`）；浏览器在 ZeoKeel 是 Tauri webview |
| **M2（企业）** | 技能/连接器/应用商店 + 只读插件清单 | C2 禁止运行期插件加载；DSH Discover 是运行期装包，不适用（`docs/16-ui-packages.md:11`；`docs/27-ui-design-system.md:64`） |

**M1 右栏的取舍**：docs/28 明确 M1 只做固定宽度单栏、折叠，多 tab/拖拽/多 pane 后置（`docs/28-ui-module-map-and-skeleton.md:80-81`）。因此 DSH 的 per-session 布局、20–80% split、fullscreen、宽度拖拽都只能作为 **post-M1 事实参考**，不进入 M1 验收；M1 的“tabs 文件/变更/终端”是固定三个 tab，不是任意多 tab。


## 5. 待确认问题（5 个）

1. **右栏绑定粒度**：DSH 是 per-session（每会话独立布局）；ZeoKeel C18 以 workspace 为会话归属。右栏布局应按 session 还是 workspace 持久化？切换同一 workspace 的不同会话时，文件树/终端是否保留？（DSH 事实：`dsh-client-ui-sidebar-right/README.md:74`）
2. **“命令面板”定义**：`docs/28-ui-module-map-and-skeleton.md:26,41,78-79` 暂定 `ui-commands` = `⌘K` 骨架 + `/` 菜单 + 快捷键参考，但 DSH 没有可核实的全局 `⌘K`。请确认 M1 的 `⌘K` 只是空壳入口（不执行真实命令），还是同时做 `/` 命令执行？若要做真实命令，host command 协议是否已就绪？
3. **设置分组**：接受 DSH 的实际分组（Settings 只有 Account/General/Models/Built-in plugins；Subagent/Web search/Agent loop/Shell 在侧栏 Plugins），还是按本任务列的 9 个“设置分页”？另外 `plugins` 是只读清单还是可管理？（DSH 事实：`dsh-client-ui-settings-general/README.md:30`；`dsh-client-ui-plugin-manager/README.md:73`）
4. **插件市场与 C2**：个人版是否明确“不做运行期插件市场”，只保留构建期插件清单？企业版 M2 商店是否只做技能/连接器/应用，不做 DSH 式 bundle 安装/开关？（C2：`docs/16-ui-packages.md:11`；`docs/27-ui-design-system.md:64`）
5. **主题与 OEM accent**：`docs/19-client-ux-flows.md:131` 写“OEM accent”，但 `docs/27-ui-design-system.md:14-16` 规定唯一 live accent 是 blue-500、品牌色只存在于官方 logo；二者冲突。ZeoKeel 是否允许 OEM accent token？若允许，需要改 27 并定义可覆盖 token 白名单；若不允许，需改 19。另：是否采用 DSH 式语义 token 分层（不照搬 `--dsw-*` 名称）？

## 6. 未验证 / 调研边界

- 未读 DSH 源码；组件名来自 README 与 `lib/types/*.d.ts`，不保证内部实现/文件结构完全一致。
- 右栏面板默认宽度、最小宽度、宽度是否持久化：README 未给出；`SurfaceState` 未含宽度，Seat 接收 `width/viewportWidth`，判断为 frame 所有（未验证）。
- Settings 各 section 的精确顺序：nav 是 `settings.section` ledger 投影，Account 登录后第一；其余顺序未在 README 固定（未验证）。
- Host command descriptor 的完整字段、`TabRecord` 的完整字段、Settings JSON Schema 文件形态：未展开（未验证）。
- “DSH 没有全局命令面板”仅限本任务列出的包；未审计整个 DSH 产品。
- “设置保存后需重启”的通用提示：本组 README 未发现；仅发现 Host 版本变化 Reload 与插件安装 restart toast。不能据此断言 DSH 全产品没有。
- 浏览器/终端/预览的 Desktop 实现依赖 Electron/原生 PTY；ZeoKeel 是 Tauri 2，交互事实可参考，载体实现不可照搬。
