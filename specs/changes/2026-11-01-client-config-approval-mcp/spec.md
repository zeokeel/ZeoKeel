# 2026-11-01-client-config-approval-mcp · 行为规格

> 只写「系统必须怎样」，不写实现选型；实现见 `design.md`。
> 本 spec 范围 = 审批策略消费链 + `connectors.*` 契约接线。MCP 配置/`mcp.test`/企业 MCP 来源见
> `2026-11-01-client-mcp-config`；审批卡字段/工具行/`always-allow`/审批决策 wire 见
> `2026-11-01-client-approval-tool-rendering`；设置 IA（pane slot/revision）见 `2026-11-01-client-settings-ia`。
> 状态：`draft`（C-1…C-6 已定（Lead 2026-11-01，见 proposal §7）；独立评审复评通过前不进入实现，C16/C19）。

## 1. 需求

### R1 · 审批策略封闭枚举、语义与弱不变式

- **WHEN** 客户端读取并应用 `AppSettings`
- **THEN** 本地工具审批策略取以下**封闭三值**之一（默认 `ask`），取值与求值如下（单一实现 `ApprovalPolicy::should_ask(requires_approval, mutating)`）：

  | 策略 | 语义（`should_ask`） |
  | --- | --- |
  | `ask`（默认） | `requires_approval`：按工具作者声明询问（= 现状行为） |
  | `auto_read_only` | 只读工具自动批准；**所有 mutating 工具都询问** → `requires_approval \|\| mutating`（C-1 已定 A） |
  | `strict` | 每次工具调用都询问（含只读/交互类工具）→ `true` |

  > C-1 已定（Lead 2026-11-01）：**A**。因此 `auto_read_only` 会询问 4 个 `mutating && !requires_approval` 工具
  > （见下方「诚实说明」），这是该档与 `ask` 的可观察差异。

- **AND** **安全不变式（T1 弱不变式，硬约束）**：任何策略在 `requires_approval == true` 时都必须询问；
  即 `policy == ask` 时不得返回 false，`auto_read_only`/`strict` 亦然。**在策略求值（`should_ask`）范围内不存在自动批准
  `requires_approval == true` 工具的路径**；唯一例外是 S4「始终允许本会话」——它是用户在先前审批中**显式授权**后的
  always-allow 命中（归 approval spec，`design.md §2.0`），不是本 spec 的自动批准。
- **AND** **诚实说明（T1）**：当前有 4 个工具 `mutating == true && requires_approval == false`，它们**工具作者声明自洽**、
  **在 `ask`（默认）下不询问**；`auto_read_only`（按 C-1=A）与 `strict` 会询问它们。4 个工具：

  | 工具 | 证据 | 作者声明自洽的理由 |
  | --- | --- | --- |
  | `schedule` | `packages/client/tool-schedule/src/lib.rs:79-80` | 只写 workspace 内 `.zeokeel/schedules.json`；自动执行另有 `AppSettings.agent.schedule.enabled` 显式开关（默认关闭） |
  | `job_kill` | `packages/client/tool-job-kill/src/lib.rs:15-16` | 只终止本进程内由 `bash_persistent`/jobs 创建的子进程，影响面限于 agent 自己的后台任务 |
  | `goal` | `packages/client/tool-goal/src/lib.rs:43-44` | 只维护会话级 goal 状态（rounds/status），无工作区/系统副作用 |
  | `mcp_call` | `packages/client/connector-mcp/src/lib.rs:873-874` | 受 MCP server 自身权限约束；调用的是用户/企业已配置的 connector（能否执行由 server 决定） |

- **AND** `strict` 的已知 UX：交互类工具（`ask_user_question`/`todo`/`present`）也会先走审批，再展示问题/卡片
  （C-5 已定：Phase 1 不豁免，记录为已知 UX；Phase 2 再评估豁免白名单）。
- **AND** 策略只影响**客户端本地**工具执行；服务端不下发策略（C6），个人版与企业版本地工具审批都适用。

### R2 · 审批事件链边界（与 approval spec 的单 owner 切分）

- **WHEN** 策略求值为「不需要询问」
- **THEN** 本次工具调用**不产生** `ApprovalRequested`/`ApprovalResolved`，也不调用 `ApprovalGate.decide`，直接执行。
- **AND** 事件序列**统一引用** approval spec 的**《审批事件序列表》**（`specs/changes/2026-11-01-client-approval-tool-rendering/design.md §2.0`，行 **S1/S2/S3a–S3c/S4/S5**；唯一真源，不复制第二份）：
  策略不需要询问 = **S5**（无任何 `approval_*` 事件）；需要询问 = **S1**（允许一次）/**S2**（拒绝）/
  **S3a**（显式取消 → `resolved{cancel,timed_out:false}` + `turn_cancelled`）/**S3b**（超时 → `resolved{reject,timed_out:true}`）/
  **S3c**（断线/窗口关闭 → `resolved{reject,timed_out:false}`，不当作 cancel）；`always-allow` 命中 = **S4**（成对 `auto:true`，不阻塞）。
- **AND** 本 spec **不定义** `always-allow（会话允许）`、`approval.decide` wire 值、拒绝/取消事件与审批卡字段：
  这些归 `2026-11-01-client-approval-tool-rendering`（其 `design.md §2.0` 为唯一真源）。
- **WHEN** 策略求值为「需要询问」
- **THEN** 事件序列与决策语义完全按 **S1/S2/S3a–S3c/S4** 执行（超时 fail-closed；无通道/退出按 S3c）。
- **AND** 若 `always-allow` 命中导致跳过询问（S4），本 spec 不重复判定；策略求值只决定「是否进入审批流程」。

### R3 · 策略的读取时机与热生效

- **WHEN** 用户在设置里保存新的审批策略（`settings.set`）
- **THEN** **下一轮** agent turn 即按新策略求值（无需重启、无需重建 agent）。
- **AND** 未保存设置（fresh install / 默认-only 文件）时运行时按默认 `ask`，既有 env/默认 fallback 语义不变：
  `ask` 必须序列化为「策略缺省」，不得让 `ToolSettings` 因策略字段变成「非默认」而抑制
  `ZEOKEEL_TOOL_TIMEOUT_MS`/`ZEOKEEL_APPROVAL_TIMEOUT_MS` 的 fallback（`docs/38` §3 既有口径）。

### R4 · 旧字段兼容与迁移

- **WHEN** 设置文档存在旧字段 `auto_approve_read_only` 且**没有**显式策略
- **THEN** 按唯一映射求值：`true → ask`、`false → strict`（C-2 已定），随后规范化为显式状态。
- **WHEN** 设置被保存（`settings.set`）或启动时设置文件已存在
- **THEN** 写回规范化结果（幂等）：

  | 生效策略 | `tools.approval_policy` | 旧字段 `auto_approve_read_only` |
  | --- | --- | --- |
  | `ask` | 缺省（`None`） | `true` |
  | `auto_read_only` | 显式 `auto_read_only` | `true` |
  | `strict` | 显式 `strict` | `false` |

- **AND** 旧版本客户端读新文件时忽略未知策略字段，仍按旧行为工作；`auto_approve_read_only` 自本版本起标记 deprecated（CHANGELOG 记录）。

### R5 · 未知/非法策略值降级（P1-7）

- **WHEN** 设置文档中的策略值为未知字符串（如 `"bogus"`）或类型非法
- **THEN** **不得**导致 `settings.load()` 失败或阻止启动；按 `ask` 降级处理（不把原文当策略）。
- **AND** 降级是确定性的、可测试的：4 种文件形态（无字段 / null / 合法值 / 未知值）× 旧字段两值均有单测。
- **AND** 不打印/回显原始未知值（避免把用户输入当数据处理）；`settings.get` 返回规范化后的值。

### R6 · `connectors.*` 命令进 IPC 契约

- **WHEN** 客户端/测试需要操作宿主连接器配置
- **THEN** `zk-contract-ipc` 提供以下 3 条命令常量与 DTO（schema/TS 由生成器产出）：

  | 命令 | 输入 | 输出 | 错误码（封闭集合） |
  | --- | --- | --- | --- |
  | `connectors.configure` | `{catalog}`（`{items:[...]}` 对象或 JSON 字符串，沿用宿主现行为） | `{configured: number}` | `connectors/catalog-invalid`、`connectors/apply-failed` |
  | `connectors.reload` | `{}` | `{reloaded: true}` | `connectors/reload-failed` |
  | `connectors.clear` | `{}` | `{cleared: true}` | `connectors/clear-failed` |

- **AND** 形状与命名按 **C-3 已定**沿用现状（上表）；命令为**幂等**：重复调用不产生重复 MCP 会话、不重复安装 OAuth token、不改变已登录态。
- **AND** `connectors.configure` 收到非法 catalog 时**不覆盖**当前生效配置（保留旧配置，返回错误）。
- **AND** 这三条命令不改变连接器 catalog 的服务端形状，也不允许 Web 注入任意 MCP server（catalog 仍是唯一来源）。

### R7 · Web 侧接线时机

- **WHEN** 以下时机发生
- **THEN** Web 调用对应命令，失败必须回到可见错误（面板错误/ toast），不允许静默吞掉：

  | 时机 | 调用 | 代码锚点 |
  | --- | --- | --- |
  | 登录/令牌刷新后目录拉取成功 | `connectors.configure {catalog}`（原始响应） | `apps/desktop/web/src/App.tsx:118-190` |
  | 连接器启停后重新拉取目录成功 | `connectors.configure {catalog}` | `apps/desktop/web/src/App.tsx:428-434` |
  | 用户点「重新加载连接器」 | `connectors.reload {}` | 新增于连接器面板/`App.tsx` 注入 |
  | 登出 | `connectors.clear {}` | `apps/desktop/web/src/App.tsx:104-115` |

- **AND** 初始同步可保留 host `auth.configure` 内的 `sync_connector_catalog`（既有行为，幂等兜底，不重复破坏）；
  Web 只负责把「最新 catalog」搬到宿主。
- **AND** `store.ts` 的 `fetchConnectors` 返回 items + 原始 catalog，不在 Web 解析 `config`（解析归宿主）。

### R8 · 设置 UI（依赖 settings-ia）

- **WHEN** 用户打开设置的「工具与审批」pane（settings-ia 的 order 40 pane；**C-4 已定**：审批控件放此处，MCP 单独 personal-only pane）
- **THEN** 可看到并编辑审批策略（三选一 + 每个选项的说明文案），旧的「自动批准只读」控件被替换/移除，不留死控件。
- **AND** 六态覆盖（P2-4）：

  | 状态 | 审批控件/连接器接线中的可观察行为 |
  | --- | --- |
  | 空 | 策略缺省 → 显示默认 `ask`；连接器列表为空 → 显示空态（不发请求） |
  | 加载 | `settings.get` 未完成 → pane 显示外壳加载态，不渲染可编辑控件 |
  | 错误 | `settings.get` 失败 → 错误态 + 重试；`connectors.*` 失败 → 面板内联错误 |
  | 离线 | personal 本地保存仍可用；enterprise 连接器操作失败给出可读错误（不假装成功） |
  | 流式中 | 保存中（saving）→ 控件禁用、显示保存中，不重复提交 |
  | 已取消 | 关闭设置丢弃草稿并恢复打开前值；保存取消不落盘 |

- **AND** 键盘可达、Esc 关闭、焦点回位；axe 0 serious/critical；新增文案走 i18n（zh-CN/en-US），无硬编码。

## 2. 边界与错误

| 场景 | 期望行为 | 错误码/提示 |
| --- | --- | --- |
| 策略值未知/非法（手改 JSON） | 降级 `ask`，启动不阻塞、不回显原文 | 无 UI 错误（诊断不打印原文） |
| `settings.set` 中策略非法（非字符串/类型错） | 拒绝保存（不写盘），内联错误 | `settings/invalid: tools.approval_policy` |
| `connectors.configure` catalog 非法/缺 items | 不更新配置、保留旧配置 | `connectors/catalog-invalid` |
| `connectors.configure` 应用失败（会话/校验） | 返回错误，旧配置继续生效 | `connectors/apply-failed` |
| `connectors.reload`/`clear` 内部失败 | 返回错误，UI 可见；不破坏登录态 | `connectors/reload-failed` / `connectors/clear-failed` |
| 审批等待超时/无通道/应用退出 | 拒绝（现状不变，fail-closed） | 现有 `denied by approval policy` |
| `strict` 下交互类工具（`ask_user_question` 等） | 先审批再提问（已知双重交互，C-5） | 无错误，属 UX 已知行为 |
| `ask` 下 4 个免审批 mutating 工具 | 不询问（工具作者声明）；文档明示 | 无错误（诚实说明） |

## 3. 权限与可见性

- **角色**：本 spec 全部为**本地客户端**能力，与角色无关；企业员工同样可配本地审批策略。
- **内容可见性**：审批策略只在本机；不上传、不同步、不进企业 catalog。
- **服务端是否下发策略**：**否**（C6 弱管控；企业不做审批、不限制工具）。服务端在 enterprise 下发连接器 catalog（既有行为，本 spec 不改形状）。
- **敏感数据**：本 spec 不新增持久化敏感数据；`connectors.*` 只搬运服务端已 sanitize 的 catalog（无 secret，ADR-060）。

## 4. 数据与兼容

- `zk-contract-config`：`ApprovalPolicy`（新枚举，snake_case）+ `ToolSettings.approval_policy: Option<ApprovalPolicy>`
  （`#[serde(default)]`，`None` = `ask`）+ `normalize_approval_policy()`/`effective_approval_policy()`；
  `AppSettings.schema` 保持 `1`（additive）。
- `zk-contract-ipc`：3 个命令常量 + `ConnectorsConfigurePayload`（`catalog: serde_json::Value`）+
  `ConnectorsConfigureResult { configured: u64 }`；`reload`/`clear` 用 `EmptyPayload` + 小结果 DTO。
  本 spec **不新增** cross-contract 依赖（`zk-contract-config → zk-contract-connector` 由 MCP spec 引入并登记，P2-8）。
- 兼容：旧设置文件（无策略）按 R4 求值；旧客户端忽略新字段；`auto_approve_read_only=false` 的用户升级后得到 `strict`。
- 幂等：`normalize_approval_policy` 幂等；`connectors.*` 幂等。
- 生成物：schema/TS/`docs/generated` 由生成器重写，禁止手改（C11）。

## 5. 验收断言（可执行）

- [ ] **弱不变式**：`cargo test -p zk-contract-config approval_policy` —— 3 策略 × 4 组合真值表；
      `requires_approval==true` 的行对三种策略都为「询问」；`ask` 下 `(false,true)` 为「不询问」（4 工具）。
- [ ] **端到端策略**：`cargo test -p zk-agent-rig approval_policy` —— 用 fake tool + 记录型 sink 断言
      `strict` 询问只读工具、`ask` 不询问、`auto_read_only` 询问 mutating、任何策略都不自动批准 `requires_approval` 工具；
      auto 路径**无** `ApprovalRequested`/`ApprovalResolved`。
- [ ] **热生效**：改已发布 snapshot 后下一轮即变；`bash scripts/agent-e2e.sh`（真实 personal rig）覆盖
      `strict` 下 `fs.read` 出审批卡、`ask` 下不出现、切换后下一轮生效。
- [ ] **迁移/降级**：`cargo test -p zk-contract-config approval_migration` —— `true→ask`、`false→strict`、幂等、未知值降级。
- [ ] **connectors 契约**：`cargo test -p zk-contract-ipc export_schema` 生成含 3 命令 DTO 的 schema；
      `bash scripts/schema-check.sh` 绿。
- [ ] **connectors dispatch**：`cargo test -p zk-desktop connectors_dispatch` —— configure/reload/clear 成功、
      catalog 非法保留旧配置、幂等（重复调用不重复会话）。
- [ ] **Web 接线**：`pnpm --dir apps/desktop/web test:unit` 覆盖「目录拉取→configure」「toggle→configure」
      「重新加载→reload」「logout→clear」调用序列与失败可见（fake transport）。
- [ ] **企业 UI 链路**：`bash scripts/login-e2e.sh`（需本机 PG；无 PG 记 UNVERIFIED）——连接器面板启停后无错误、
      本地 MCP 生效（配合 MCP spec 的 fake server）；`e2e/m3-client-content.spec.ts` 扩展。
- [ ] **a11y/i18n**：审批控件与连接器面板键盘可达/axe 0 serious-critical；`bash scripts/i18n-check.sh` 绿。
- [ ] **文档与门禁**：`python3 scripts/spec-check.py`、`python3 scripts/docs-check.py`、`python3 scripts/review-check.py`、
      `zk validate`、`zk compose --edition personal --profile default --channel official --check`、
      `zk capability --check`、`cargo test --workspace export_schema && git diff --exit-code`、
      `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build`。
- [ ] **文档冲突清零**：`rg "web 侧接线" docs KNOWN-LIMITATIONS.md` 无命中。

## 6. 决策（已定；无未决项）

与 `proposal.md` §7 **同一份**（避免决策状态自相矛盾，评审 P1-2）；全部为「已定」，本 spec 仅因**独立评审未复评**而保持 `draft`（C16/C19）：

- [x] C-1 `auto_read_only` = 只读自动放行 + 所有 mutating 询问（A）
- [x] C-2 旧字段映射 `false→strict`、`true→ask`
- [x] C-3 `connectors.*` 形状沿用现状（`{catalog}`→`{configured}`；`reload/clear`→`{reloaded:true}`/`{cleared:true}`）
- [x] C-4 审批控件放 settings-ia「工具与审批」pane（order 40）；MCP 单独 personal-only pane
- [x] C-5 Phase 1 `strict` 不豁免纯交互工具（双重交互记为已知 UX，Phase 2 再评估）
- [x] C-6 不新增 ADR；决策落 `docs/38` §2 + `docs/07` §6.1；任务 1 docs-first
