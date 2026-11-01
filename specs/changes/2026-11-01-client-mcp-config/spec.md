# 2026-11-01-client-mcp-config · 行为规格

> 只写「系统必须怎样」，不写实现选型；实现见 `design.md`。
> 本 spec = MCP 本地配置链路（personal）。审批策略/`connectors.*` 接线见父 spec `2026-11-01-client-config-approval-mcp`；
> 审批卡/工具行/`always-allow` 见 `2026-11-01-client-approval-tool-rendering`；设置 IA 见 `2026-11-01-client-settings-ia`。
> 状态：`draft`（M-1…M-6 已定（Lead 2026-11-01，见 proposal §7）；独立评审复评通过前不进入实现，C16/C19）。

## 1. 需求

### R1 · 本地 MCP 配置的存储（personal）

- **WHEN** personal 用户保存 MCP 配置
- **THEN** 配置持久化在 `AppSettings.mcp.servers`（`Vec<McpServerConfig>`，非 secret，随 `settings.get/set` 读写）；
  新增字段全部 `#[serde(default)]`，`AppSettings.schema` 保持 `1`。
- **AND** 校验失败时 `settings.set` **整体拒绝、不落盘**（与原 provider/embedding 失败语义一致，`apps/desktop/src/ipc.rs:531-592`）。
- **AND** 配置上限（写入契约 `McpSettings::validate`，防 settings.json 膨胀）：

  | 项 | 上限 |
  | --- | --- |
  | server 数 | 16 |
  | 单 server `id`/`name` | ≤128 字符；`id` 仅 `[A-Za-z0-9._-]` |
  | `args` 条数 / 单条长度 | 32 / 2048 |
  | `env` 键值对 / 键长 / 值长 | 32 / 128 / 2048 |
  | `url` | ≤2048 |
  | `timeout_ms` | 0（默认）..=600000 |
  | 重复 | 同 `id` 必须拒绝 |

- **AND** 本地 server 的 `oauth_required = true` **必须拒绝保存**（本地无 authorize/token 元数据，无法完成 PKCE；提示改用企业 catalog）。

### R2 · 两级校验（catalog 不回归，P1-8）

- **WHEN** 校验来自企业 catalog 的 server
- **THEN** 使用**现有规则原样**（`validate_server` 语义逐字节保持：id/name 非空、transport 匹配、http(s) 仅 loopback 明文、
  url 长度/字符、secret-like env 拒绝），**不套用**本地上限与 id 字符集 —— 既有合法 catalog 升级后仍可用。
- **AND** 现有 `validate_server` 规则提升为 `zk-contract-connector::McpServerConfig::validate()`（单一实现，
  `zk-connector-mcp::validate_server` 转为委托；catalog 路径 `servers_from_catalog` 行为不变）。
- **WHEN** 校验来自本地 settings（personal）
- **THEN** 先走 `McpServerConfig::validate()`，再走 `McpSettings::validate()` 的本地上限/id 字符集/重复/`oauth_required` 拒绝。
- **AND** 有回归测试证明「catalog 旧合法、但超出本地上限」的 server 仍被 catalog 路径接受、被本地 settings 路径拒绝。

### R3 · 来源优先级与 edition 口径（T4/P1-10）

- **WHEN** 解析当前生效的 MCP server 列表
- **THEN** 按以下**单一来源**优先级（不合并）：
  `enterprise catalog（已安装；含空列表）> personal 本地 settings（已安装；含空列表）> env（仅 personal 且无 settings 文件）`。
- **AND** **enterprise 为 catalog-only**：本地 settings 不安装；**env `ZEOKEEL_MCP_SERVERS` 也不生效**；
  未登录/无 catalog 时 MCP 诚实不可用（不静默、不回退 env）。
- **AND** personal：设置文件存在时以 settings 为准（含空列表）；无设置文件（fresh install/测试）时保留 env 行为。
- **AND** 空列表语义：一旦某来源被「安装」，即使列表为空也视为权威（不回落下一级），避免「企业全员禁用后又从 env 冒出本地 server」。
- **AND** enterprise 保存非空 `AppSettings.mcp.servers` 必须被拒绝（`settings.set` 返回可读错误），boot 清理持久化的本地 MCP 段
  （对齐 embedding/media 的 C7 清理先例，`apps/desktop/src/ipc.rs:269-295`）。

### R4 · 生效与热更新

- **WHEN** 本地 MCP 配置保存成功或应用启动
- **THEN** 配置被安装到 MCP client 并**重载会话**（下一次 `mcp_call`/`mcp_list_tools` 使用新配置）；无需重启。
- **AND** 安装失败（校验/构建 transport 失败）时 `settings.set` 返回错误且**不落盘**（不出现「已保存但未生效」）。

### R5 · 测试连接（`mcp.test` / `mcp.test.status`；P1-9）

- **WHEN** 用户在 MCP pane 点「测试连接」
- **THEN** 前端调用 `mcp.test`（携带**当前草稿** server 配置），宿主**启动独立探测**并立即返回 `{test_id}`
  （沿用 `oauth.authorize`/`oauth.status` 两段式先例，`packages/contracts/ipc/src/lib.rs:125-133`、`apps/desktop/src/ipc.rs:1282-1330`）。
- **WHEN** 前端轮询 `mcp.test.status {test_id}`
- **THEN** 返回 `{state: "pending"|"ok"|"error", tools?: string[], error?: string}`；`ok` 时 `tools` 为 tool 名列表
  （≤200 条、单条 ≤128 字符）；`state` 为**封闭枚举**（`pending`/`ok`/`error`，契约 `McpTestState`）；`error` 为封闭错误码
  （`mcp/validation`、`mcp/connect-failed`、`mcp/timeout`、`mcp/test-in-flight`、`mcp/unsupported`）。
- **AND** 探测必须**无缓存**：使用一次性 transport（spawn/connect → `initialize` → `tools/list` → 关闭），
  **不得**写入全局会话缓存（`SESSIONS`，`packages/client/connector-mcp/src/lib.rs:261-294`），结束后子进程/连接必须回收；
  有测试断言「探测前后全局会话数不变」。
- **AND** 单次探测硬超时 ≤15s（取 `timeout_ms` clamp；默认 10s）；超时后返回 `mcp/timeout`，不改变当前生效配置。
- **AND** 该命令只能由主窗口 IPC 调用（WASM 应用窗口既有 capability 已禁止 `ipc_call`，不变）。

### R6 · 设置 UI（personal-only pane）

- **WHEN** personal 用户打开设置
- **THEN** 存在「MCP」pane（由新包 `zeokeel/ui-settings-mcp` 通过 `ui.settings.pane` 贡献，order 70），支持：
  列表、新增、编辑、删除、测试连接、保存；enterprise 构建中**不存在该 pane**（构建期 exclude，非运行期分支）。
- **AND** 六态覆盖（空/加载/错误/离线/流式中/已取消）：

  | 状态 | 可观察行为 |
  | --- | --- |
  | 空 | 无 server → 空态 + 「添加」入口；不显示空白表单 |
  | 加载 | `settings.get` 未完成 → 外壳加载态，不渲染可编辑列表 |
  | 错误 | `settings.get` 失败 → 错误态 + 重试；测试连接失败 → 该行内联错误 |
  | 离线 | 本地编辑/保存仍可用（本地 IPC）；测试连接给出可读失败（不假装成功） |
  | 流式中 | 「测试连接」探测中 → 该行 pending、按钮禁用、可取消；保存中 → 控件禁用 |
  | 已取消 | 取消测试 → 停止轮询、恢复按钮；关闭设置 → 丢弃草稿、恢复打开前值 |

- **AND** 键盘可达、Esc 关闭、焦点回位；axe 0 serious/critical；所有文案走 i18n（zh-CN/en-US），无硬编码。

### R7 · 组合与 personal-only 清单（P1-15）

- **WHEN** 构建 personal/enterprise 组合
- **THEN** `editions/personal.toml` 映射 `ui.slot.settings-mcp = zeokeel/ui-settings-mcp`；
  `editions/enterprise.toml` 的 `exclude` 包含该包；**`editions/personal-only.toml` 的 `packages` 数据列表**加入
  `zeokeel/ui-settings-mcp`（该文件是 `zk validate` 读取的数据清单，不是注释，`editions/personal-only.toml:3`）。
- **AND** `zk validate` 通过且能断言 enterprise 闭包不含该包；`zk compose --check` 对 personal 生成
  `SettingsPanes`（order 70）快照、对 enterprise 不生成该 pane。

### R8 · 文档先行、依赖登记与门禁

- **WHEN** 开始实现
- **THEN** 任务 1（docs-first）已完成：`docs/15` §C 登记 D 编号或 ADR（C1/C6 映射：企业 catalog-only 属构建期 edition 差异，
  不是服务端管控）；`docs/07` §6.1、`docs/16`、`docs/24` §1、`docs/04` §1.5 更新。
- **AND** `zk-contract-config → zk-contract-connector` 依赖在 `docs/24` §1 / `docs/04` §1 登记（P2-8），
  并写明「settings schema 内含 connector DTO」的兼容责任（connector DTO 破坏性变更需同步 config schema 评审）。
- **AND** C10/C11：schema/TS/生成物由生成器写；`cargo test --workspace export_schema && git diff --exit-code`、
  `bash scripts/schema-check.sh` 进 tasks/acceptance；无手改 `@generated`。

## 2. 边界与错误

| 场景 | 期望行为 | 错误码/提示 |
| --- | --- | --- |
| `settings.set` 本地 server 校验失败（空 id/重复/超上限/字符集/secret-like env） | 拒绝保存、不落盘，字段内联错误 | `mcp/validation`（保存错误含具体 server id） |
| 本地 `oauth_required=true` | 拒绝保存 | `mcp/oauth-unsupported-local` |
| `mcp.test` 草稿校验失败 | 拒绝探测 | `mcp/validation` |
| `mcp.test` 连接失败/不可达 | 返回错误、不保存、不改生效配置 | `mcp/connect-failed` |
| `mcp.test` 超时（>15s） | 返回超时、探测结束并回收资源 | `mcp/timeout` |
| `pending` 期间再次 `mcp.test`（并发探测） | 拒绝新探测、不覆盖在飞探测 | `mcp/test-in-flight` |
| 探测期间全局会话被污染 | 禁止（测试断言 `SESSIONS` 数量不变） | 测试失败（`cargo test -p zk-connector-mcp`） |
| enterprise 保存非空本地 MCP | 拒绝并提示企业以 catalog 为准 | `settings/invalid: mcp.local-forbidden-in-enterprise` |
| enterprise 无 catalog / 未登录 | MCP 诚实不可用（不回退 env） | `mcp/unsupported`（调用 `mcp_call` 时既有 `MCP_CAPABILITY_UNAVAILABLE`） |
| 无 settings 文件 + env 非法 | 启动不失败；MCP 不可用并记录诊断日志（不回显 env 值） | 无 UI 错误（诚实不可用） |
| `server.url` 非 https 且非 loopback | 拒绝保存 | `mcp/validation`（既有规则） |

## 3. 权限与可见性

- **角色**：本地能力，与角色无关；企业员工不能通过客户端新增本地 MCP（T4）。
- **C1/C6 映射（T4，需 docs/15 D 或 ADR 落定）**：企业版 = 个人版 + 企业管控包；MCP 本地配置属**个人版专属能力**，
  通过**构建期组合**（`ui-settings-mcp` personal-only 包 + enterprise exclude + 客户端来源策略）表达，
  不是服务端对工具/网络的管控（C6 弱管控不变：服务端不审批、不限制工具；企业只是以 catalog 作为分发来源）。
- **内容可见性**：本地 MCP 配置只在本机；不上传、不同步、不进 catalog。
- **敏感数据**：本地配置**不允许**静态 secret（既有 `validate` 的 secret-like env 拒绝，ADR-060）；OAuth token 仍只在
  OS keychain（catalog 流程），本地配置不涉及；`mcp.test`/日志/错误不得回显 env 值或 token。

## 4. 数据与兼容

- 契约（additive）：`AppSettings.mcp: McpSettings`（`servers: Vec<McpServerConfig>`，默认空）；
  `zk-contract-connector::McpServerConfig::validate()` 新增方法（规则 = 现有 `validate_server`，不含本地上限）；
  `zk-contract-ipc`：`MCP_TEST = "mcp.test"`、`MCP_TEST_STATUS = "mcp.test.status"` + `McpTestPayload`/`McpTestResult`/
  **`McpTestStatusResult`**（`state`/`tools`/`error`）与**封闭枚举 `McpTestState { Pending, Ok, Error }`**（serde snake_case）；
  `zk-contract-config` 新增依赖 `zk-contract-connector`（无环）；**`zk-contract-ipc` 新增依赖 `zk-contract-connector`**
  （`McpTestPayload.server: McpServerConfig`，无环；Cargo 写作用域见 tasks 任务 5）。
- 兼容：旧设置文件（无 `mcp`）→ 无本地 server；旧客户端忽略 `mcp` 字段；**catalog 行为不变**（R2 两级校验）；
  env 行为：personal 无 settings 文件时仍生效，有 settings 文件后以 settings 为准（CHANGELOG/release note，M-4）。
- 幂等：`update_local_servers`/`clear_local_servers` 幂等；`mcp.test` 只读探测，不产生持久化副作用；
  `mcp.test` 同一时刻只允许一个探测（`pending` 期间重复调用 → `mcp/test-in-flight`）；`test_id` 唯一。
- 生成物：schema/TS/`docs/generated` 由生成器写（C11）。

## 5. 验收断言（可执行）

- [ ] **契约**：`cargo test -p zk-contract-config mcp`（默认/回读/上限/重复/id 字符集/`oauth_required` 拒绝/失败不落盘语义）；
  `cargo test -p zk-contract-connector`（`McpServerConfig::validate` 规则与旧实现逐条一致）。
- [ ] **两级校验回归（P1-8）**：`cargo test -p zk-connector-mcp catalog_limits` —— catalog 旧合法但超本地上限仍接受；
  本地 settings 同配置被拒。
- [ ] **优先级/edition（T4）**：`cargo test -p zk-connector-mcp precedence` —— catalog（含空）> local（含空）> env；
  enterprise 下 env 关闭后不回退；`cargo test -p zk-desktop enterprise_mcp` —— boot 清理 + `settings.set` 拒绝。
- [ ] **探测无缓存（P1-9）**：`cargo test -p zk-connector-mcp test_server_no_cache` —— 探测前后全局会话数不变、
  子进程已回收；`cargo test -p zk-desktop mcp_test` —— 合法/非法/超时/并发 id 错误码。
- [ ] **组合（P1-15）**：`zk validate`（enterprise 闭包不含 `zeokeel/ui-settings-mcp`）；
  `zk compose --edition personal --profile default --channel official --check`（`SettingsPanes` 含 order 70）；
  enterprise 组合构建 `cargo check` 通过。
- [ ] **C10/C11**：`cargo test --workspace export_schema && git diff --exit-code`；`bash scripts/schema-check.sh`；
  `zk capability --check`；无手改生成物。
- [ ] **前端**：`pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build`；`bash scripts/i18n-check.sh`；
  `pnpm --dir packages/ui/ui-settings-mcp test:unit`。
- [ ] **e2e（真实运行器）**：`bash scripts/m1-e2e.sh --grep "MCP"` —— personal `serve` + fake stdio MCP server（`e2e/mcp-config.spec.ts` 的 `test.describe` 内 `test.skip(process.env["ZEOKEEL_E2E_SKIP_SERVE"] === "1", "mock stage")`：第一段 mock 自动 skip，第二段真实 serve 执行）：
  MCP pane 添加 server → 测试连接返回 tools → 保存 → 刷新保留；六态/axe 断言。
- [ ] **文档与门禁**：`python3 scripts/spec-check.py`、`python3 scripts/docs-check.py`、`python3 scripts/review-check.py`。

## 6. 决策（已定；无未决项）

与 `proposal.md` §7 **同一份**（避免决策状态自相矛盾，评审 P1-2）；全部为「已定」，本 spec 仅因**独立评审未复评**而保持 `draft`（C16/C19）：

- [x] M-1 `AppSettings.mcp.servers`（不建独立 store；上限防膨胀；per-server revision 留后续）
- [x] M-2 本地 `oauth_required=true` 保存时拒绝（无 authorize/token 元数据，无法 PKCE）
- [x] M-3 两段式 `mcp.test` → `mcp.test.status`（无缓存探测、硬超时 ≤15s、可取消）
- [x] M-4 env 不自动导入（`ZEOKEEL_MCP_SERVERS` 仅 personal 且无 settings 文件时生效）
- [x] M-5 新 personal-only 包 `ui-settings-mcp`（`ui.settings.pane` order 70；enterprise exclude + personal-only 清单）
- [x] M-6 不新增 ADR；决策落 `docs/15` §C D 编号 + `docs/24` §1 / `docs/04` §1.5 + `docs/07`/`docs/16`/`docs/38`；任务 1 docs-first
