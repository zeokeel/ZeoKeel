# 2026-11-01-client-composer-surface · 技术设计

> 规范细节链接 `docs/`，不在这里复制第二份规范。组合/manifest 以 `24` 为准，API 以 `04` 为准。

## 0. 已定决策（用户 2026-11-01 批准推荐项）

| # | 决策 | 落地口径 |
| --- | --- | --- |
| D1 | 命令集边界 | 只复用既有 client 命令目录（`App.tsx` 唯一 catalog + `runCommand`）；`/` 与 ⌘K 同一来源；不接 Host/skill 命令 |
| D2 | 附件位置/上限 | `<workspace>/.zeokeel/attachments/<conversation_id>/`；≤1 MiB/文件、≤5 个/条、8 类 allowlist；`workspace.attachment_save`；**serve body limit 2 MiB 保持不变**（1 MiB base64≈1.37 MiB 在限内，§2.2） |
| D3 | 模型范围 | 只读标签 + 点击打开设置并定位模型相关 pane（personal `provider` / enterprise `enterprise`；`openSettings(paneId)` 深链，`SettingsDialog.initialPane`）；不做内联切换/发现（§2.4） |
| D4 | context 口径 | 只显示既有 usage（本轮+会话累计），不显示窗口百分比（§2.4） |
| D5 | `@`/附件 | 仅 workspace 文件/文件夹引用；附件不直传多模态（§2.3/§2.4） |
| D6 | 发送语义 | 客户端合并引用块为文本，`chat.send` 契约不变（§2.3/§2.6） |

## 1. 影响面

| 类型 | 变更 |
| --- | --- |
| 包 | 修改 `zeokeel/ui-chat`（composer 输入面）、`@zeokeel/ui-kit`（共享 DTO/UiActions）、`zeokeel/ui-settings`（可选 `initialPane` 深链）；**不新增包/slot**（`ui.composer` 仍为 post-M1 保留，composer 留在 `ui-chat`，与 `docs/28` §4 一致） |
| 契约 | `zk-contract-ipc`：新增命令 `workspace.attachment_save` + `AttachmentSavePayload`/`AttachmentSaveResult`；`ChatSendPayload` **不变**（引用块为文本）；`zk-contract-session` 不变 |
| 路由 | 无 HTTP 路由；本地 IPC 新增 1 命令；`serve /ipc` body limit **不变**（附件单文件上限 1 MiB，base64+JSON 仍在 2 MiB 内） |
| 迁移 | 无数据库迁移；新增 workspace 内附件文件目录 |
| 生成物 | `shared/contracts/schema/*.json`、`shared/contracts/ts/contracts.ts`（生成器写）；`crates/zk-composed/src/lib.rs`/`Cargo.toml` 由 `zk compose` 生成（源：`tools/zk/src/main.rs`），本 spec 只改生成器源，**不手改 `@generated` 模板/产物** |
| edition/profile/channel | 无 edition 分支；personal/enterprise 均可用；模型标签数据由 app 按 edition 计算后注入 UiValue |
| 文档 | `docs/19` §2（输入行/附件状态）、`docs/26` §5（`workspace.attachment_save` 命令 + `attachment/*` 错误码）、`KNOWN-LIMITATIONS.md`（附件边界） |

## 1.5 复用检查（C17，必填）

| 要实现的点 | 现有能力（capability id / 路径） | 结论 | 归属 |
| --- | --- | --- | --- |
| 命令目录/执行 | app 现有 `commands`/`runCommand`（`apps/desktop/web/src/App.tsx:591-607`） | 扩展（提升为唯一 catalog，⌘K 与 `/` 共用） | desktop-web |
| 命令面板交互模式（listbox/↑↓/Enter/Esc） | `@zeokeel/ui-commands`（`packages/ui/ui-commands/web/src/index.tsx`） | 复用模式，不复制状态/组件 | ui-chat 内菜单 |
| 文件候选/引用 | `workspace.files`（`contract.session` FileEntry；host `ipc.rs:914`；前端 `parseFileEntries` `protocol.ts:164`） | 复用（不新增文件搜索 IPC） | 契约/主进程 |
| 用量 | `AgentUsage` + `usage` 事件 + `UsagePart`/`readUsage` | 复用 | ui-chat |
| 发送/停止/离线 | `UiActions.send/cancel`、`UiValue.streaming/offline` | 复用 | ui-kit |
| 模型持久化 | `settings.get/set` + revision（settings-ia） | 复用（只读展示 + 打开设置） | ui-kit/desktop-web |
| 附件校验/落盘 | 无现成能力；host 仅复用 workspace/conversation 归属校验（`chat_send` 的 `list_conversations_in`） | 新建（单一 IPC） | `contract.ipc` + host |
| 附件路径安全 | `zk-kit-tools` 现有 workspace/path 语义（`crates/zk-kit-tools/src/lib.rs`）；`zk-composed` 是 app 生成 facade（C4） | **扩展**：新增 `zk_kit_tools::safe_join_under(root, parts)` 单一 helper（逐组件 no-follow、canonicalize 父目录、`starts_with(root)`、拒绝绝对/`..`），host 经 `zk_composed` facade 调用；**禁止自行拼路径**（C17） | `crates/zk-kit-tools`（kit） |

- 搜索证据：`rg -n "AssistantComposer|composerPlaceholder|commands =|workspace.files|parseFileEntries|AgentUsage|usage" packages apps docs`；
  `./target/debug/zk find kit.tools` → `crates/zk-kit-tools/src/lib.rs`；capability 索引 `contract.ipc`（`shared/contracts/capability-index.json:43`）。
- 生成器 facade 边界（C4/C2）：`apps/desktop` 只依赖 `zk-composed`（组合根生成 glue）；`tools/zk/src/main.rs` 生成 `pub use zk_kit_tools;`（以及 side/activity 所需 kit/tool 的 facade），
  因此 app 不新增直接 kit/tool 依赖、不手改 `@generated`。`zk compose --check` 验证生成物一致。
- 为什么不能复用/扩展：`ui-commands` 是 app 持有的全局 overlay（open/commands/onRun props），不能内嵌到 composer；
  复用其**命令目录与交互模式**，composer 菜单作为 `ui-chat` 内部组件（不新增 slot，`docs/28` §4 明确 composer 留在 ui-chat）。
- 新增可复用能力：`zk_kit_tools::safe_join_under`（`kit.tools` 内扩展，不新增 capability id）；IPC 命令属 `contract.ipc` 扩展；不登记新 INDEX 条目。

## 2. 方案

composer 从「纯 textarea + Send/Stop」升级为「受控输入 + 三个内联 popup + chip 草稿 + 模型/用量 footer」，
所有触发/选择在 `ui-chat` 内完成，数据/动作经 `UiValue`/`UiActions` 由 app 注入（不 import app，不新增 slot）。发送仍走
`chat.send`：引用/附件在客户端合并为**规范化引用块**追加到用户正文，host/agent/契约不感知新结构。

### 2.1 关键流程

```text
用户输入 / 或 @
  → composer 检测词首触发（非 IME composing）
  → /：候选 = App 注入的 commands（同一 catalog）→ ↑↓/Enter → actions.runCommand(id) → 清空触发文本
  → @：actions.searchWorkspaceFiles(query)（内部复用 workspace.files 缓存）→ Enter → 加 chip、移除 @query
拖放/选择附件
  → 客户端校验类型/大小/数量 → actions.saveAttachment(file) → IPC workspace.attachment_save
    → 成功：chip（名称/大小/类型/图片缩略图）；失败：chip 错误态 + 重试（不发送）
发送（Enter 或 Send）
  → composeOutgoingText(正文, chips) = 正文 + "\n\n--- referenced files ---\n@path..."（去重、稳定顺序）
  → 现有 runtime onNew → actions.send(finalText)
  → UI 渲染用户消息时 strip 引用块为 chip 列表
```

### 2.2 数据结构（新增/修改）

```text
# 契约（packages/contracts/ipc/src/lib.rs，additive）
commands::WORKSPACE_ATTACHMENT_SAVE = "workspace.attachment_save"
payloads::AttachmentSavePayload {
  workspace_id: String,
  conversation_id: String,
  name: String,          # basename（host sanitize + 校验）
  mime: String,          # 客户端提示；host 以扩展名/magic 重新判定并以结果为准
  data_base64: String,   # 编码后的文件字节
}
results::AttachmentSaveResult {
  path: String,          # workspace 相对路径（正斜杠），如 ".zeokeel/attachments/<conv>/<uuid>-<name>"
  name: String,          # sanitize 后的显示名
  bytes: u64,            # 解码后字节数
  mime: String,          # host 判定的 canonical MIME
}

# 限制（host 常量，spec R3/R2 的边界）
MAX_ATTACHMENT_BYTES = 1 * 1024 * 1024
MAX_ATTACHMENTS_PER_MESSAGE = 5
ATTACHMENT_EXT_ALLOWLIST = ["png","jpg","jpeg","webp","gif","pdf","txt","md","json","csv","log"]
# 说明：serve 载体 axum Json 默认 2 MiB body limit；1 MiB 原始字节 base64 后约 1.37 MiB，
# 加 JSON 包装仍在限内。更大上限需同改 serve body limit（本 spec 不做，记 KNOWN-LIMITATIONS）。

# kit 单一 helper（crates/zk-kit-tools；C17）
pub fn safe_join_under(root: &Path, parts: &[&str]) -> Result<PathBuf, ToolError>
# 规则：parts 不得为空/绝对/`..`/控制字符；逐组件 symlink_metadata 拒绝 symlink；
#       逐级 create_dir 后 canonicalize 父目录，starts_with(canonical root) 必须成立；
#       失败返回 ToolError::Denied/InvalidArgs，不写盘。

# 引用块（纯文本，稳定格式；客户端写入/剥离的唯一实现）
"{body}\n\n--- referenced files ---\n@path1\n@path2\n"
# 编码：path 必须是 workspace 相对路径；拒绝控制字符/换行；块只允许在消息尾部；
# path 以 @ 开头时写成 @@path（parser 去一层 @）；空 body 时仅块。
```

### 2.2.1 生成器 facade（C4/C11，P2-11/P2-12）

- `apps/desktop`（Rust 组合根）**不新增**对 `zk-kit-tools`/tool 包的直接依赖；`tools/zk/src/main.rs` 生成 `zk-composed` 时把所需 kit 加入其
  generated Cargo 依赖并 `pub use`（如 `pub use zk_kit_tools;`），desktop 通过 `zk_composed::zk_kit_tools::safe_join_under` 调用。
- **生成物归属澄清（P2-11）**：`tools/zk/templates/app_desktop.Cargo.toml` 是**生成器输入源**（其首行 `@generated` 为历史遗留注释，不代表它是本 spec 的生成物）；本 spec **不改该模板**，只改生成器源 `tools/zk/src/main.rs` 并让 `zk compose` 生成 `zk-composed`/`apps/desktop/Cargo.toml`；二者仍是 @generated，禁止手改，`--check` 必须绿。
- 只改生成器源；`crates/zk-composed/{Cargo.toml,src/lib.rs}` 与 `apps/desktop/Cargo.toml` 由 `zk compose` 写（@generated），
  **不手改模板/产物**；`zk compose --check` 必须绿（模板 `tools/zk/templates/app_desktop.Cargo.toml` 保持不变）。

### 2.3 引用块与 UI chip

- `composeOutgoingText(body, refs)`：`refs` 按添加顺序去重（path）；body 为空时只发块；块内每行一个 `@<相对路径>`；
  尾部换行固定，便于 `stripReferenceBlock` 精确匹配。
- `parseUserContent(text)`：返回 `{ body, refs }`；`refs` 用于 chip 渲染与刷新后恢复；块标记**固定大小写**且只在尾部精确匹配；
  旧消息不含标记时视为普通正文；含控制字符/空路径/非法 `..` 的行跳过不计入 refs（不产生伪 chip）；`@@x` 解析为 `@x`。
- 附件 chip 与 `@` chip 共用 `DraftRef { kind: "workspace"|"attachment"; path; name?; size?; mime? }`，发送时都进引用块；
  目录引用 path 以 `/` 结尾（`@dir/`），文件引用不带尾随 `/`；
  `attachment` 额外展示大小/类型/缩略图：用 `FileReader.readAsDataURL(file)` 生成 `data:` URL（Tauri CSP `img-src 'self' data:` 已允许），
  **不用 `URL.createObjectURL`/`blob:`**（CSP 未允许 blob；不改 CSP）；卸载时清空引用。
- 文件名 sanitize（host 与客户端各一层）：仅 basename（去掉 `/`、`\`、`..`、控制字符/空白首尾），≤128 字节，空则 `attachment`；
  `conversation_id` 先经 workspace 归属校验（`list_conversations_in`）；最终路径**必须**经 `safe_join_under(workspace_root, [".zeokeel","attachments",conv_id,file_name])`，
  父目录/`attachments`/`.zeokeel` 任何一级是 symlink 都拒绝（不写盘），不复用「拼接 + 名称 sanitize」自行实现（C17/P0-1）。
- 校验：客户端按扩展名/大小快速失败；host 再按解码后字节数、allowlist（扩展名 + 图片/PDF magic 前 8 字节）、空文件校验，
  以 host 结果为准；拒绝时不写盘。

### 2.4 模型标签与用量

- UiValue 新增 `modelLabel: string`（app 计算：personal = `settings.provider.model` 或空 → 未配置；enterprise = 已有
  `gatewayModel` state 或空 → 未选择；标签旁提示来源 `provider`/`gateway`），`UiActions.openModelSettings()`（= 现有
  `openSettings()`；**不新增 pane 深链**，用户在设置内进入「模型」pane；spec R4 与本处语义一致）。
- 用量：UiValue 已有 `messages`（含 assistant `usage`）；composer 用 `readUsage`/`usageFromReply` 的既有格式化，
  计算 `sum(messages.usage)` + 当前流式消息 usage；无 usage 状态用 i18n `usageUnreported`。**不新增事件/调用**。
- `/` 命令：复用 app 的 `commands`/`runCommand`。`cancel`/`retry` 条目新增到该 catalog（`retry` 使用 controller 现有
  `retrySend`，`cancel` 使用 `cancel`）；`disabled` 按 `streaming`/`lastSendText` 计算，执行入口仍在 app（ui-chat 只发 id）。
- `@` 文件候选：`UiActions.searchWorkspaceFiles(query)` 由 app 实现：首次调用 `workspace.files` 拉取并缓存（当前会话 workspace），
  之后本地过滤；workspace 切换时清缓存。返回 `{path, kind}[]`（最多 300，沿用 host 上限）；UI 过滤隐藏 `.git`/`.zeokeel` 后截断 50
  （附件路径由 chip 直接携带，不经候选列表）。
- 命令/文件菜单共用 `ComposerMenu` 渲染（listbox/option/aria），但**状态隔离**（`docs/28` §4.7：composer `/` popup 与 ⌘K palette
  按 surface 分开状态）。

### 2.5 键盘/IME/焦点

- 输入框改为**受控原生 `<textarea>`**（保留现有 `ComposerPrimitive.Root` 的容器样式 class 与发送/停止按钮样式；
  不再使用 `ComposerPrimitive.Input`，以便同一处管理触发菜单、IME 与 chips；不引入第三方编辑器/不自写富文本）。
  Enter：菜单打开 → 选中；否则发送；Shift+Enter 换行；Esc 关闭菜单；↑↓ 在菜单打开时仅移动高亮；
  composing 期间（`compositionstart`→`compositionend`）所有菜单/发送触发禁用，compositionend 后恢复。
- 发送按钮使用 app 的 `ui.actions.send(composeOutgoingText(text, chips))`，成功后清空 textarea/chips，失败保留草稿；
  停止按钮继续用 `ComposerPrimitive.Cancel`（runtime 不变）。
- 菜单关闭后焦点回输入框（不新增 focus trap；不使用 `useDialogFocus` 以免抢 composer 焦点）；
  菜单为 `role="listbox"` + `aria-activedescendant`，textarea `aria-expanded/aria-controls/aria-autocomplete="list"`。
- 附件拖放目标为 composer 容器（`onDragOver/onDrop`）；键盘路径：附件按钮 → 文件选择；chips 删除按钮可达。

### 2.6 发送与渲染

- composer 的发送按钮/Enter 直接调用 `ui.actions.send(composeOutgoingText(text, chips))`（不经过 assistant-ui runtime 的
  `onNew` 组合逻辑；runtime 与 `ChatView` 不改），发送成功后清空 textarea/chips，失败保留草稿（与现有发送失败重试一致）。
- `AssistantThread` 的 `UserText` 使用 `parseUserContent` 渲染正文 + 引用 chip 列表（引用块不展示原文）；
  无块时行为不变。刷新/回放走 `session.messages` 持久化文本，天然恢复。

### 2.7 备选与取舍

| 方案 | 优点 | 缺点 | 结论 |
| --- | --- | --- | --- |
| A. 受控 assistant-ui composer + 文本引用块 + 新 IPC 存附件 | 复用现有 runtime/发送链，契约最小，附件工具可读 | chip 非内联原子 token（显示在输入框上方）；引用块出现在持久化文本中 | **采用** |
| B. 换 Lexical/自研 contenteditable 实现内联原子 chip | 与 DSH 形态最接近 | 违反"不自己写编辑器/焦点管理"，IME/undo/a11y 风险大，超预算 | 放弃（Phase 3 评估） |
| C. `chat.send` 增加结构化 references/attachments 字段 | 数据模型干净 | 契约+host+回放+兼容面变大，超 16 文件预算 | 放弃（若 D6 选 B 再拆 spec） |
| D. 附件存 `$ZEOKEEL_HOME` 而非 workspace | 不污染 workspace | 模型/工具（fs 根=workspace）无法读取，需新增读取 IPC | 放弃（D2-B） |

## 3. 备选与取舍（实现细节）

- 受控输入：`useComposerInput`（assistant-ui 0.15.32 稳定导出，`unstable/` 路径）作为首选；若升级移除该 API，
  回退为原生 `<textarea>` + 现有 `ComposerPrimitive` 外观（不引入第二套编辑器库），并在 design 风险评估中记录。
- 附件上传：串行（≤2 并发）上传，单文件失败不阻塞其它文件；发送时只允许全部成功（或用户移除失败项）。

## 4. 安全与隐私

- 威胁/滥用：附件类型伪装（扩展名/magic 不一致）、超大 base64 内存放大、路径穿越（`../`、绝对路径、Windows 分隔符、
  控制字符）、workspace 外写入、会话/工作区不匹配、通过引用块注入指令（引用块仅是文本，模型按用户输入处理；
  不做 shell/命令执行）。缓解：host 解码后大小校验、magic 校验、文件名 sanitize、conversation 归属校验、
  **`safe_join_under` 逐组件 no-follow + canonicalize 父目录 + starts_with(root)**（父目录/attachments symlink 逃逸拒绝，见单测）、
  附件上限 1 MiB（serve `/ipc` body limit 内）+ token 鉴权/CORS 本地来源。
- 密钥/令牌：不涉及；附件内容按用户数据对待，不写日志、不进 i18n、不跨 workspace。
- 审计：无新增审计（本地个人/企业员工工具行为，弱管控）；错误只记本地诊断日志（不含附件内容）。

## 5. 发布与回滚

- 灰度/强制更新：无（客户端随发布版本）；旧客户端不调用新命令，兼容。
- 回滚：回退 commit；已写附件文件可保留（孤儿文件，需后续清理；记录 KNOWN-LIMITATIONS）；已发送消息中的引用块为普通文本，回滚后按文本渲染（可读）。
- 客户端兼容：新命令 additive；`/ipc` body limit 提升向后兼容（旧客户端小 payload 不受影响）。

## 6. 测试计划

| 层 | 用例 | 命令 |
| --- | --- | --- |
| T0 组合 | 命令 schema/TS 生成、compose 不变 | `zk compose --edition personal --profile default --channel official --check`；`bash scripts/schema-check.sh` |
| T1 kit 单测 | `safe_join_under`：`..`/绝对/workspace 外/父目录 symlink/文件名 symlink 拒绝且不写盘 | `cargo test -p zk-kit-tools` |
| T1 host 单测 | attachment 校验/落盘/拒绝/归属（含 1 MiB 边界、symlink 父目录） | `cargo test -p zk-desktop` |
| T1 UI 单测 | `/`/`@` 菜单、IME、引用块 compose/strip、chip 去重/上限、命令目录单一来源、模型/用量 | `pnpm --dir packages/ui/ui-chat/web test:unit`；`pnpm --dir apps/desktop/web test:unit` |
| T3 e2e | `/` 执行、`@` 引用发送、附件选择上传、刷新恢复、chip 渲染、axe | `pnpm e2e`（m1-serve 部分，真实 serve+SQLite） |
| T4 门禁 | typecheck/lint/i18n/workspace 检查 | `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build`；`bash scripts/i18n-check.sh` |

## 7. 文档更新

- [ ] `docs/19` §2：输入行状态（`/`/`@`/附件/模型/用量）、附件错误码/上限口径（`attachment/too-large`、`attachment/type-not-allowed`、`attachment/invalid`、`attachment/workspace-unavailable`）
- [ ] `docs/26` §5：`workspace.attachment_save` 命令 + `attachment/*` 错误码登记（SSOT）
- [ ] `KNOWN-LIMITATIONS.md`：附件上限/位置/不直传多模态/孤儿文件/无 GC；chip 非内联原子 token
- [ ] ADR/D：无新增（复用既有 `docs/16/27` 包边界与 `docs/26` workspace 规则）

## 8. 风险

| 风险 | 概率 | 影响 | 缓解 | 验证 |
| --- | --- | --- | --- | --- |
| 附件 base64 内存放大 | 低 | 中 | 单文件 1 MiB 上限 + 解码后校验；客户端先按大小拒绝；不做大文件流式 | host 单测 + e2e |
| 父目录 symlink 逃逸（P0-1） | 中 | 高 | 单一 `safe_join_under`（no-follow + canonicalize 父 + starts_with）；host 经生成 facade 调用；父目录/attachments/文件名 symlink 单测 | `cargo test -p zk-kit-tools -p zk-desktop` |
| 原生 textarea 自管 IME/键盘（Enter/Shift+Enter/Esc/↑↓） | 中 | 中 | 明确状态机 + composition 事件单测；e2e 键盘路径；不实现富文本/撤销栈，保持原生行为 | 单测 + e2e |
| 引用块被用户文本误识别 | 低 | 低 | 固定标记+精确解析；仅尾部块；单测边界 | 单测 |
| 附件写 workspace 触发 vcs/变更列表 | 低 | 低 | `.zeokeel/` 已用于 goal/schedule；文件树默认过滤；文档说明 | e2e/评审 |
| 命令目录漂移 | 中 | 中 | 单一 catalog + 同一 runCommand；单测断言 id 集合 | 单测 |
