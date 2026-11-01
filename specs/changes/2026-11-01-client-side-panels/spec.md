# 2026-11-01-client-side-panels · 行为规格

> 只写"系统必须怎样"，不写实现选型；实现见 `design.md`。

## 1. 需求

### R1 · 右栏文件树（懒加载）

- **WHEN** 用户打开右栏 files tab（或切换 workspace/刷新）
- **THEN** 显示根目录直属项（目录优先/名称排序），目录可展开/折叠；展开时按需加载该目录直属子项（不一次遍历全树）
- **AND** 每个目录行显示展开状态与加载/错误态（加载失败可在该行重试，不阻塞其它目录）；刷新按钮保持已展开路径并重载可见项
- **AND** 列表隐藏 `.git` 与 `.zeokeel`（workspace 元数据）；**符号链接不跟随、不展示**（design 明确；预览也拒绝 symlink 逃逸）
- **AND** 旧 `workspace.files` 调用（只带 workspace_id）行为不变：返回 depth≤2、≤300 项的有界列表
- **AND** 空工作区/加载中/错误/离线六态不白屏；点击文件触发 R2 预览

### R2 · 文件预览（文本/图片；其它诚实降级）

- **WHEN** 用户点击文件（或预览中的刷新/reload）
- **THEN** 在 files tab 内显示预览（树保持可返回）：
  - 文本（UTF-8）：显示内容，>256 KiB 截断并显示「已截断」；不可解码/二进制 → 降级；
  - 图片（png/jpeg/webp/gif）：显示图片（≤8 MiB），超限/解码失败 → 降级；
  - PDF/Office/HTML/其它二进制或超限：显示**降级说明**（文件名、大小、支持类型），不白屏、不阻塞右栏
- **AND** 路径安全：只允许 workspace 相对路径；拒绝绝对路径、`..`、符号链接（含父目录 symlink）逃逸、工作区外 canonical path（错误可读）
- **AND** 文件不存在/权限错误/过大显示对应错误态，可重试/关闭预览；预览不修改文件

### R3 · 右栏真实终端

- **WHEN** 用户打开 terminal tab 并新建/选择一个终端会话
- **THEN** 显示当前会话列表（可新建/切换/关闭）与输出区/输入行；输入命令执行后输出追加（stdout/stderr），
  执行中显示运行态、超时可读提示；关闭会话后从列表移除；输出可清空
- **AND** 命令由用户通过 IPC 触发，**不是 agent 工具调用**，不生成工具行/审批；不拼接 host shell 命令（输入原样传入持久 shell）
- **AND** 当前平台不支持该 shell（如缺少 bash/pwsh）时明确报错「终端不可用」并说明平台限制，不白屏
- **AND** 每会话输出/历史有界（UI 限制显示条数/字节；host 单次输出上限≤64 KiB，超出标记截断）；会话 id 合法性与数量上限沿用现有持久终端规则（≤8/会话）

### R4 · 右栏 tabs/宽度/折叠持久化（会话级）

- **WHEN** 用户切换 tab、拖拽/键盘调整右栏宽度、折叠/展开右栏
- **THEN** 状态按**会话**持久化（刷新/重启/切回会话恢复）：当前 tab、宽度（px）、折叠状态；首次使用默认 files + 288px + 展开
- **AND** 宽度有上下限（≥240px、≤720px 且不超过视口 50%），中栏保持可读；窄窗（<768px）自动收起右栏（保留用户展开状态，窗口变宽后恢复）
- **AND** 宽度调整可用键盘（左右方向键，焦点在分隔条）且分隔条有可读 aria；状态解析失败/脏数据回默认值，不崩溃

### R5 · 会话操作（rename / fork / copy id / pin / archive）

- **WHEN** 用户在侧栏会话行打开 `⋯` 菜单
- **THEN** 提供 rename（非空标题 ≤200 字符，内联/弹窗编辑，确认后更新列表标题）、fork、copy id（复制到剪贴板，有成功提示）、
  pin/unpin（置顶/取消置顶）、archive/unarchive（归档/恢复）
- **AND** **C18**：所有操作保持 `workspace_id` 不变；fork **只能**在源会话所属 workspace 内创建新会话并复制消息（新 id/标题可改，
  新会话不复制 pin/archive 状态）；跨 workspace 不允许（菜单不提供）
- **AND** archive 仅使会话从**默认列表**隐藏（不硬删、消息保留、可在「显示已归档」筛选下打开与 unarchive）；pin 的会话排在同 workspace 的非 pin 会话之前
- **AND** 操作失败（存储/校验/会话不存在）显示可读错误，列表不变；成功后列表/激活态一致（fork 后选中新会话）

### R6 · 会话全文检索入口

- **WHEN** 用户在侧栏搜索框输入查询并选择「内容」模式（默认仍为标题/路径模式）
- **THEN** 调用 `session.search`（当前 workspace、查询、limit 默认 20/最大 50）返回命中：会话 id/标题、消息 id、角色、内容片段；
  结果按会话分组/列表展示，点击结果打开对应会话
- **AND** 查询为空/仅空白不请求；查询长度 ≤200 字符；无命中/加载/错误有明确状态；请求防抖（≥250ms）、输入变化取消旧结果
- **AND** 检索为大小写不敏感子串匹配（与现有 `session_search` 工具一致），不跨 workspace，不读归档以外的其它 workspace；
  归档会话命中包含 `archived` 标记并可打开；不做索引/正则/高亮/滚动定位（点击仅打开会话）
- **AND** 隐私：检索结果与打开会话同权限（当前 workspace 的本地数据），不额外写日志/不跨 workspace 返回；
  已知密钥形态**不额外脱敏**（会话正文本身在 UI 可见），但不得把结果写日志或上报；显示 snippet 而非整段文本（tool 输出保持完整 content，见 design §2.3）

### R7 · 质量与边界

- **WHEN** 任意上述功能在空/加载/错误/离线/流式中/已取消场景
- **THEN** 对应区域显示六态占位/提示，可重试或返回，不白屏、不丢失其它面板状态
- **AND** 所有文案走 i18n（zh/en）；axe 无 serious/critical；键盘可达（tab 切换/树展开/菜单/终端输入/搜索）；不新增 slot/包/运行期加载（C2）
- **AND** 不改变 agent/审批/网关/权限语义；右栏与搜索均为只读浏览（终端命令与文件会话操作由用户显式触发）

## 2. 边界与错误

| 场景 | 期望行为 | 错误码/提示 |
| --- | --- | --- |
| `workspace.files` 目录不存在/不可读 | 该目录行错误态 + 重试；其它目录不受影响 | `workspace/path-not-found`、`workspace/not-a-directory` |
| `workspace.file_read` 路径逃逸/绝对路径/符号链接逃逸 | 拒绝，不读文件 | `workspace/path-escape` |
| 文件不存在/权限/非 UTF-8 | 降级/错误提示，可关闭 | `workspace/path-not-found`、`file/unsupported` |
| 文本 >256 KiB / 图片 >8 MiB / PDF 等 | 成功返回但 `truncated`/`kind=unsupported`，UI 诚实降级 | `file/too-large`、`file/unsupported` |
| 终端会话 id 非法/未打开/未关闭不存在 | 拒绝并保持 UI 状态 | `terminal/session-invalid`、`terminal/session-not-open` |
| 终端命令超时（≤10s）/输出过大 | 返回超时/截断标记，会话仍可用 | `terminal/timeout`、`terminal/truncated` |
| 平台无可用 shell | 明确不可用提示，terminal tab 空态 | `terminal/unavailable` |
| rename 空标题/超长/会话不属于 workspace | 拒绝，不改列表 | `session/invalid`、`session/workspace-mismatch` |
| fork 源会话不存在/跨 workspace | 拒绝 | `session/not-found`、`session/workspace-mismatch` |
| archive/fork 会话仍在流式 | UI 禁用菜单项并提示稍后（host 不新增 busy 错误） | — |
| 搜索查询空/超长/limit 非法 | 空结果/裁剪到合法范围 | `session/invalid` |
| localStorage 脏数据/配额满 | 回默认/仅本会话内存态，不崩溃 | — |

## 3. 权限与可见性

- 角色：个人用户/企业员工均可使用右栏、会话操作、检索；无管理员/审计员专属面；无服务端策略。
- 内容可见性：文件/终端只在当前 workspace 根内（preview 拒绝逃逸）；检索只查当前 workspace 的会话（C18）。
- 服务端是否下发策略：否（弱管控 C6）；终端命令由用户显式触发，不经审批链（非 agent 工具调用）。

## 4. 数据与兼容

- 新增字段：`Conversation.pinned`、`Conversation.archived`（bool，`#[serde(default)]`，默认 false）。
- 迁移：SQLite schema **v6**，仅 `ALTER TABLE conversations ADD COLUMN pinned/archived INTEGER NOT NULL DEFAULT 0`（幂等 `has_column`）；
  旧库 v5/v4 迁移后旧行 false；无数据回填。`list_conversations_in` 返回全部（含 archived，带标记），排序 `pinned DESC, updated_at_ms DESC, id DESC`。
- 新 IPC（additive）：`workspace.files` 可选字段扩展、`workspace.file_read`、`terminal`、`session.update`、`session.fork`、`session.search`；
  旧客户端不调用无影响；旧 `workspace.files` 调用（只 workspace_id）行为不变。
- 引用/编码：文件读取结果 `encoding = utf8|base64`，文本截断标记；不写数据库。终端不持久化输出（进程内）；会话操作持久化 SQLite。
- 幂等：pin/archive/rename 幂等（相同值成功）；fork 非幂等（每次新会话）；搜索只读。
- 回滚：回退 commit；SQLite v6 仅加列，旧二进制忽略新列可读；已归档/置顶数据在回滚后不影响旧 UI（字段被忽略）。

## 5. 验收断言（可执行）

- [ ] 契约/存储：`Conversation` 新字段默认值/旧 JSON 兼容；`SessionStore` 新方法默认实现不破坏第三方/测试实现（默认 Invalid；SQLite 覆盖）；
      `cargo test -p zk-contract-session`。
- [ ] 迁移：v5→v6 幂等（重复打开不报错）、旧行 pinned/archived=false、列表排序 pinned 优先；`cargo test -p zk-store-session-sqlite`。
- [ ] fork：同 workspace 复制全部消息、新 id/标题、workspace 不变、不复制 pin/archive；默认 trait 实现 + sqlite 路径单测。
- [ ] 检索：kit 公共函数与 `session_search` 工具结果一致（同一查询）；host IPC 边界（空/超长/limit/跨 workspace）；`cargo test -p zk-kit-session -p zk-tool-session-query -p zk-desktop`。
- [ ] 文件：旧 `workspace.files` 无参行为不变；`path/depth/limit` 扩展裁剪正确；`workspace.file_read` 文本/图片/降级/逃逸/超限；
      host 单测（`cargo test -p zk-desktop`）。
- [ ] 终端：open→send→list→close；未知 session/非法 id/超时/平台不可用错误码；host 单测。
- [ ] UI/e2e（m1-serve 追加）：文件树展开/刷新保持；文本/图片预览与 PDF 降级；终端命令输出；会话 rename/pin/archive/fork/copy id；
      内容搜索命中并打开会话；tab/宽度/折叠刷新恢复；窄窗自动收起；axe 0 serious。
- [ ] 门禁：`pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build`；`pnpm e2e`；`bash scripts/i18n-check.sh`；
      `cargo test --workspace export_schema` + `bash scripts/schema-check.sh`；`zk compose --check`；`zk capability --check`。
- [ ] 兼容/回滚：旧库（v5）打开后读写正常；回滚后归档/置顶数据不导致旧代码崩溃（字段忽略）。

## 6. 已定决策（用户 2026-11-01 批准推荐项）

- [x] **D1 终端**：复用 `zk-kit-tools::persistent`（现有持久 bash 会话）经新 `terminal` IPC（open/send/close/list，超时≤10s）；不新增 PTY 服务/前端 xterm。
- [x] **D2 能力面板**：拆出 `specs/changes/2026-11-01-client-activity-panels/`（jobs/subagent/agent-team/goal/todo/schedule/plan 展示/present + activity tab order 40 + turnTail，预算≤14）；本 spec 只预留 docs/16 tab order 40。
- [x] **D3 检索/schedule**：检索走 `session.search` IPC + `zk_kit_session::search` 单一实现；schedule 管理留给 activity-panels，经 `zk-tool-schedule` 的公开 `Tool::call_json`（list/update/delete）读写 `.zeokeel/schedules.json`，不新建第二套文件解析/校验。
- [x] **D4 plan**：不做 plan mode；activity-panels 不实现 plan 工具/状态机，turnTail 仅把 assistant Markdown 中可识别的计划标题段落折叠展示（「展示计划文本」）。
- [x] **D5 archive/fork**：archive 仅从默认列表隐藏（不硬删、可筛选/恢复）；fork 在**同 workspace** 内复制消息到新会话（`workspace_id` 不变，C18）。
- [x] **D6 PDF/预览**：文本（≤256 KiB）+ 图片（≤8 MiB）在线预览；PDF/Office/HTML/二进制/超限诚实降级（CSP `frame-src 'none'`，不改 CSP/不内嵌）。

