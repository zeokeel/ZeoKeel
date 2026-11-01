# 2026-11-01-client-composer-surface · 行为规格

> 只写"系统必须怎样"，不写实现选型；实现见 `design.md`。

## 1. 需求

### R1 · `/` 命令菜单

- **WHEN** 用户在 composer 输入框的词首输入 `/`（且不在 IME composing 中）
- **THEN** 在输入框上方打开命令候选菜单，候选来自**与 ⌘K 全局面板同一份命令目录**（同一数据源、同一执行入口），
  按 group/title/hint 展示；输入继续过滤（大小写不敏感），无匹配时显示空态
- **AND** `↑`/`↓` 移动高亮、`Enter` 执行、`Esc` 关闭且**不丢失草稿文本**；`disabled` 命令可见但不可执行；
  执行成功后清除触发文本（`/query`），焦点回到输入框；执行失败走既有 toast 错误链，不清空已输入正文
- **AND** 首期命令集 = 既有 client 命令（personal 现 4 条 + 本 spec 新增取消/重试 = 6 条；enterprise 额外技能/连接器/应用 = 9 条）；
  `/cancel` 仅在流式中可执行，`/retry` 仅在有上一条发送文本且非流式中可执行；不出现 Host/技能命令

### R2 · `@` 文件引用

- **WHEN** 用户在词首输入 `@`（或点击 composer 引用按钮）且当前会话有 workspace
- **THEN** 打开 workspace 文件候选菜单，数据来自 `workspace.files`（现有有界列表；不新增文件搜索 IPC/索引）；
  输入过滤 path（大小写不敏感），目录/文件均可见并标注 kind；目录引用以 `@path/` 形式加入 chip/引用块（模型可用文件/搜索工具读取目录）
- **AND** `Enter`/点击把选中项加入草稿为**引用 chip**（原子条目：不可被文本编辑破坏，可删除），
  并从输入框移除 `@query` 触发文本；重复引用去重；`Esc` 关闭且不清空正文
- **AND** 文件列表为空/加载中/错误/离线时，菜单显示对应状态与可读提示；不阻塞用户继续输入普通文本
- **AND** 引用上限与附件合并计数 ≤5 条/消息；超出时禁用新增并提示

### R3 · 附件（拖放 / 选择 / 预览 / 上限 / 失败）

- **WHEN** 用户拖放文件到 composer、或点击附件按钮选择文件
- **THEN** 对每个文件做客户端快速校验（类型 allowlist：png/jpeg/webp/gif/pdf/txt/md/json/csv/log；单文件 **≤1 MiB**（IPC JSON body 上限内）；总数 ≤5），
  合法文件显示上传中 chip（名称/大小/类型；图片显示本地缩略图，使用 FileReader `data:` URL，不依赖 `blob:`/CSP 变更），上传成功变为可发送 chip；同一条消息最多 5 个
- **AND** 类型不支持/超限/空文件/上传失败时显示可读错误（i18n），**草稿保留**，可移除失败项或重试；不发送半成品附件
- **AND** 附件保存位置为当前 workspace 的 `.zeokeel/attachments/<conversation_id>/`（工具可读；见 design §2.2）；
  附件名称 sanitize（仅 basename、长度上限、拒绝路径分隔符/控制字符）；
  写入路径必须经单一 kit helper `zk_kit_tools::safe_join_under` 校验（逐组件 no-follow、canonicalize 父目录、`starts_with(root)`、拒绝绝对/`..`），
  父目录 symlink 指向 workspace 外时**拒绝写入**（host 单测覆盖）
- **AND** 附件仅作为「workspace 文件 + 引用」进入消息（不做图片字节直传模型）；模型可用既有文件/图片/文档工具读取

### R4 · 模型显示

- **WHEN** composer 渲染（且非空会话）
- **THEN** 底部显示当前模型标签：personal 读取 `AppSettings.provider.model`（未配置显示「未配置模型」），
  enterprise 显示既有 gateway model（未选择显示「未选择网关模型」）；标签带可读 aria-label
- **AND** 点击标签打开设置并**定位到模型相关 pane**（personal `provider` / enterprise `enterprise`；通过既有 `SettingsPanes` id 的可选 `openSettings(paneId)` 深链；本轮只读展示、不改变模型；切换仍走设置 pane 的既有保存/revision 语义）
- **AND** 模型变更（设置保存/网关模型切换）后标签自动更新；错误/离线不阻塞输入

### R5 · 用量/上下文指示

- **WHEN** 本轮有 provider 报告的 usage（含流式中 `usage` 事件与完成后的 `AgentUsage`）
- **THEN** composer 显示本轮用量（input/output/cached/total）与会话累计（当前会话已持久化 assistant usage 求和），
  数值只来自既有 `AgentUsage`/`usage` 事件，不新增 provider 调用
- **AND** 无 usage 时显示「用量未报告」（不显示 0、不估算上下文窗口百分比）；数值变化在流式/完成时更新，刷新后与会话持久化一致
- **AND** 指示器为只读、可被屏幕阅读器读取；不引入新的事件类型/契约

### R6 · 键盘与可达性

- **WHEN** 用户使用键盘或辅助技术
- **THEN** `/`/`@` 菜单：输入框保持焦点、`aria-expanded`/`aria-controls`/`aria-activedescendant` 正确、
  候选为 `listbox`/`option` 语义、`Esc` 关闭并回输入框；菜单打开时方向键不移动光标，关闭后恢复正常编辑
- **AND** `Enter` 发送（菜单未打开且非 IME composing）、`Shift+Enter` 换行、`Esc` 关闭菜单；
  IME composing 期间的 Enter/`/`/`@` 不触发发送/菜单；`/`/`@` 仅在词首触发（正文中间的字符按普通文本）
- **AND** 菜单/附件/模型/用量控件在 compositor 空态/加载/错误/离线/流式中/已取消六态下不白屏、不丢失焦点管理

### R7 · 引用块与发送

- **WHEN** 用户发送消息且草稿含 `@` 引用或附件
- **THEN** 发送的用户消息文本 = 用户正文 + 规范化引用块（`--- referenced files ---` + 每行 `@<workspace-relative-path>`；
  格式见 design §2.3），引用块用于模型读取文件；正文为空但存在引用/附件时**允许发送**（只发引用块）；
  路径编码：仅 workspace 相对路径、拒绝 `\n`/`\r`/`\0` 等控制字符、块标记只允许出现在消息尾部（parser 精确匹配），
  path 以 `@` 开头时按 `@@path` 形式编码/解析（不产生伪条目）
- **AND** 消息持久化后，UI 把引用块渲染为 chip/列表（不展示原始块）；刷新/重载历史后仍能识别并展示；
  引用路径全部为 workspace 相对路径，去重且按添加顺序稳定
- **AND** 引用块之外的正文按现有 Markdown/文本规则渲染；无引用/附件的消息行为与现在一致（不追加空块）

## 2. 边界与错误

| 场景 | 期望行为 | 错误码/提示 |
| --- | --- | --- |
| 附件 >1 MiB / 总数 >5 / 空文件 | 拒绝该文件，保留草稿与其它 chip | `attachment/too-large`、`attachment/limit`、`attachment/empty`（i18n 文案） |
| 附件类型不在 allowlist（含可执行/脚本/二进制） | 拒绝并提示支持的扩展名 | `attachment/type-not-allowed` |
| 附件 base64 非法/解码后大小不符/文件名含路径分隔符 | 拒绝，不写盘 | `attachment/invalid` |
| 附件保存时 workspace 不可写/会话不属于该 workspace | 拒绝，不写盘，提示重试 | `attachment/workspace-unavailable`、`session/workspace-mismatch` |
| `workspace.files` 不可用/为空 | `@` 菜单空态/错误态，可继续输入 | 既有 IPC 错误映射 |
| `/` 无候选 / 命令 disabled | 空态；不可执行（不报错） | — |
| 流式中发送 | 保持现有：发送禁用/停止按钮可见；`/cancel` 可执行 | 既有行为 |
| 离线 | 发送禁用（现有）；附件上传为本地 IPC，失败时显示错误并保留草稿 | 现有离线提示 |
| 超长输入 | 保持现有 4000 字符计数行为（本 spec 不新增限制） | 既有文案 |
| 模型未配置/网关模型未选 | 标签显示「未配置」；点击仍打开设置 | — |

## 3. 权限与可见性

- 角色：个人用户/企业员工均可在自己 workspace 的会话中使用 `/`、`@`、附件、模型标签与用量；无管理员/审计员专属面。
- 内容可见性：`@`/附件只列/读当前 workspace（C18）；不跨 workspace，不读工作区外路径。
- 服务端是否下发策略：否（弱管控 C6；本 spec 纯客户端输入面，不改变权限/审批语义）。
- 附件写入受当前 workspace 可写性约束；企业版不因此获得本地/网关之外的能力。

## 4. 数据与兼容

- 新增 IPC：`workspace.attachment_save`（payload/result 见 design §2.2；additive，旧客户端不调用无影响）。
- 新增磁盘数据：`<workspace>/.zeokeel/attachments/<conversation_id>/<uuid>-<sanitized-name>`（用户可见文件，不写 SQLite）。
  无 schema 迁移；会话消息内容新增约定块（普通文本，旧客户端/旧 UI 仍可显示）。
- 引用块格式向后兼容：无引用/附件时不写块；旧会话无块按普通文本渲染。
- 幂等/重试：同一文件重复上传生成不同文件名（不覆盖）；上传失败可重试；发送本身仍由现有 `chat.send` 语义保证（非幂等，重复发送=新消息）。
- 回滚：删除附件目录/回退 commit 即可；已发送消息中的引用块是普通文本，回滚后按文本显示（可读、不破坏）。

## 5. 验收断言（可执行）

- [ ] kit 单测：`zk_kit_tools::safe_join_under` 拒绝 `..`/绝对路径/workspace 外/父目录 symlink 指向外部，且不写盘；`cargo test -p zk-kit-tools`。
- [ ] host 单测：`workspace.attachment_save` 校验（类型/大小/base64/文件名/会话归属/路径不逃逸）与成功写入路径；
      超限/非法返回上述错误码；`cargo test -p zk-desktop` 通过。
- [ ] 引用块边界单测：路径含换行/控制字符/`@` 开头/含块标记文本/无正文，compose+strip 不产生伪条目；`pnpm --dir packages/ui/ui-chat/web test:unit`。
- [ ] DOM/单测：`/` 打开、过滤、↑↓/Enter/Esc、disabled 命令、执行后清空触发文本；`@` 过滤、加 chip、去重、超限禁用；
      引用块 compose/strip 单测（`pnpm --dir packages/ui/ui-chat/web test:unit`）。
- [ ] 命令目录单一来源：`rg` 仅一处定义；⌘K 与 `/` 的 id 集合一致（单测断言）。
- [ ] IME：compositionstart 期间 Enter 不发送、`/`/`@` 不开菜单；`pnpm` 单测。
- [ ] 模型标签：personal/gateway 两种来源、未配置态；用量：有 usage/无 usage/流式更新（app 单测 + e2e）。
- [ ] e2e（真实 serve + fake upstream，追加到 `e2e/m1-serve.spec.ts`）：`/` 执行新建会话；`@` 引用文件并发送；
      附件经 file chooser 上传后经 `/ipc workspace.files` 断言 `.zeokeel/attachments/...` 存在；引用块以 chip 显示且刷新后仍在；
      axe 0 serious/critical。
- [ ] 门禁：`pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build`；`bash scripts/i18n-check.sh`；
      `cargo test -p zk-desktop`；`cargo test --workspace export_schema` + `bash scripts/schema-check.sh`；`zk compose --check`。
- [ ] 兼容/回滚：无引用消息行为不变；删附件目录后 UI 显示「文件不存在」降级而非白屏（e2e 或单测）。

## 6. 已定决策（用户 2026-11-01 批准推荐项）

- [x] **D1 命令集边界**：只复用既有 client 命令目录（**现 personal 4/enterprise 7；本 spec 新增 cancel/retry 后 6/9**），`/` 与 ⌘K 共用 app 的唯一 catalog 与 `runCommand`；不接 Host/skill 命令。
- [x] **D2 附件**：保存到 `<workspace>/.zeokeel/attachments/<conversation_id>/`；单文件 ≤1 MiB、每条 ≤5 个、8 类 allowlist；新增 `workspace.attachment_save` IPC。
  现有 `serve /ipc` axum `Json` 2 MiB body limit 足够（1 MiB 原始字节 base64≈1.37 MiB），本 spec 不改 `serve`；提高上限另立 spec。
- [x] **D3 模型**：只读标签（personal provider.model / enterprise gateway model）+ 点击打开设置并定位模型相关 pane（`provider`/`enterprise`，可选 `openSettings(paneId)` 深链）；不做内联切换/模型发现。
- [x] **D4 context**：只显示既有 usage（本轮 input/output/cached/total + 会话累计），不显示 context window 百分比、不新增 provider 调用。
- [x] **D5 `@`/附件范围**：`@` 只做 workspace 文件/文件夹；附件仅作为 workspace 文件 + 引用块，不直传多模态字节给模型。
- [x] **D6 发送语义**：引用/附件由客户端合并为规范化引用块追加到用户文本；`chat.send` 契约不变；模型用文件工具读取引用。

