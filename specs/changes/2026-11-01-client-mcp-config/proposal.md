# 2026-11-01-client-mcp-config · 提案

- 状态：`done`
- Owner：zk-audit（Phase 1c-2，从 `2026-11-01-client-config-approval-mcp` 拆出）
- 日期：2026-11-01
- 关联：`docs/reviews/2026-11-01-client-ui-parity-specs.md`（CHANGES-REQUESTED；本稿按其 P1-3 拆分建议、
  P1-8/P1-9/P1-10/P1-15 与 P2-8/P2-9 修复）；`research/client-dsh-parity-2026-11-01.md` §4/P1-2、§7；
  `target/audits/zk-client-inventory.md` §5/A2；`docs/07` §6.1；`docs/16`；`docs/24` §1；`docs/04` §1.5；
  `packages/client/connector-mcp/README.md`；父 spec `2026-11-01-client-config-approval-mcp`
- 评审：docs/reviews/2026-11-01-client-ui-parity-specs.md
- 变更预算：约 **21 个工作项 / 约 27 个文件**（契约 6、connector-mcp 3、desktop 2、新 UI 包 8、editions 3、e2e 2、ui-kit 1；其中契约含 3 个 crate 的 src+tests）+ 文档 6；拆分自父 spec（父 spec 原 34 文件 → 父约 16 + 本约 27），目标单一（MCP 本地配置链路）

## 1. 背景 / 问题

ZeoKeel 客户端**已有可用的 MCP 客户端实现**（`packages/client/connector-mcp`，stdio JSON-RPC + HTTP(S)，`McpServerConfig` 契约
`packages/contracts/connector/src/lib.rs:170-199`），但**用户无法配置**：

1. **personal 只能 env**：配置来源只有 `ZEOKEEL_MCP_SERVERS`（`packages/client/connector-mcp/src/lib.rs:29,172-183`），
   README 明确「运行期来源为企业 catalog 或本地 env」；没有保存/编辑/测试入口。
2. **enterprise 只有 catalog**：登录后 `auth.configure` 拉取 catalog → `update_from_catalog`（`apps/desktop/src/ipc.rs:669-683`），
   但**未登录/无 settings 文件时仍可能走 env**（`configured_servers` 现对空 catalog 回落 env，`connector-mcp:244-252`），
   与「企业 catalog 唯一执行配置来源（T4）」口径不一致。
3. **评审发现的实现细节问题**（本 spec 必须一并修）：
   - **P1-8 两级校验**：`validate_server` 被 catalog（`servers_from_catalog`，`connector-mcp:186-218`）与本地 env 共用
     （`connector-mcp:102-148`）；若把「本地上限/id 字符集」直接加进共享 validate，会**收紧企业 catalog**（既有配置可能被整体拒绝）。
   - **P1-9 `mcp.test` 无缓存探测**：`list_server_tools` 走全局 `SESSIONS` 缓存（`connector-mcp:261,406-413,442-444`），
     拿它做「测试连接」会把草稿 server 的会话/子进程留在进程里；且同步阻塞 IPC 线程（P1-9）。
   - **P1-15 personal-only 清单**：`editions/personal-only.toml:3` 是**数据清单**（`zk validate` 读取，`tools/zk/src/main.rs:1376-1416`），
     新 personal-only 包必须加入该 `packages` 列表，不能只写注释。
4. **独立评审结论**：本部分原属 `client-config-approval-mcp`，被判定「34 文件超拆分阈值」（P1-3），因此**拆为独立 spec**；
   Lead 决策 T4：**Phase 1 企业版本地 MCP 不允许，且 enterprise 下 env `ZEOKEEL_MCP_SERVERS` 也不生效（catalog-only）**，
   需写明 C1/C6 映射并先落 docs/24 §1 / docs/07 §6.1（任务 1；不新增 ADR）。

不做的后果：MCP 后端能力继续只能靠 env/服务端，UI 里无法新增/测试；企业版仍可能出现 env 与 catalog 双源；
两级校验问题会让 catalog 升级后出现「企业连接器突然不可用」的回归。

## 2. 目标

- **G1 本地 MCP 配置可管理（personal）**：`AppSettings.mcp.servers: Vec<McpServerConfig>` 承载非 secret 配置
  （stdio command/args/env allow-list、http(sse) url、timeout），设置 UI 提供增删改 + **测试连接**；保存即热生效（重载 MCP 会话），失败不落盘。
- **G2 两级校验，不回归企业 catalog（P1-8）**：把**现有** `validate_server` 规则**原样**提升为
  `zk-contract-connector::McpServerConfig::validate()`（catalog 行为逐字节不变）；**新增**的本地上限/id 字符集/
  `oauth_required` 拒绝只放在 `McpSettings::validate()`（仅 settings 路径）。测试覆盖「catalog 旧合法但超本地上限仍可用」。
- **G3 来源优先级与 edition 口径（T4/P1-10）**：
  `catalog（已安装，含空列表）> personal 本地 settings（已安装，含空列表）> env（仅 personal 且无 settings 文件）`；
  **enterprise：catalog-only —— 本地 settings 不安装、env 也不生效**（未登录 = 无 MCP，诚实不可用）。
  在 `docs/24` §1 / `docs/07` §6.1（不新增 ADR） 落定 C1/C6 映射（构建期 edition 差异，不是服务端管控）。
- **G4 测试连接不污染进程状态（P1-9）**：新增**无缓存**探测路径（fresh transport：spawn/connect → initialize → tools/list →
  drop/close/kill），断言不写入全局 `SESSIONS`；命令采用两段式（`mcp.test` 启动 → `mcp.test.status` 轮询，沿用
  `oauth.authorize`/`oauth.status` 先例），有硬超时（≤15s），不改变当前生效配置。
- **G5 UI 包与组合（P1-15）**：新增 personal-only UI 包 `zeokeel/ui-settings-mcp`（四件套）贡献 `ui.settings.pane`
  （order 70）；`editions/personal.toml` 映射角色、`editions/enterprise.toml` exclude、**`editions/personal-only.toml`
  的 `packages` 数据列表**加入该包；`zk validate` 断言 enterprise 闭包不含它。
- **G6 文档先行 + 依赖登记 + 门禁**：任务 1 先落 docs（`docs/24` §1 / `docs/07` §6.1、`docs/07`、`docs/16`、`docs/24` §1、
  `docs/04` §1.5、`docs/26` §5（`mcp.test`/`mcp.test.status` 两条 IPC 命令））；**`zk-contract-config → zk-contract-connector`（P2-8）与 `zk-contract-ipc → zk-contract-connector`（P1-13.1）** 两条依赖在 `docs/24`/`docs/04` 登记（含 public-api 影响）；
  C10/C11 生成物门禁（`export_schema`/`schema-check.sh`）进 tasks/acceptance。

## 3. 非目标

- 审批策略、`connectors.*` 契约接线：属父 spec `2026-11-01-client-config-approval-mcp`（本 spec 不写）。
- 审批卡/工具行/`always-allow`/`usage`：属 `2026-11-01-client-approval-tool-rendering`。
- 设置 IA（pane slot/revision/外壳）：属 `2026-11-01-client-settings-ia`；本 spec 只贡献一个 pane。
- **本地 MCP OAuth**：本地配置没有 `authorize_url`/`token_url` 元数据，无法跑 PKCE loopback；Phase 1 本地 server
  仅 stdio / 无 OAuth 的 http(s)，`oauth_required=true` 在本地设置中拒绝（企业 catalog 的 OAuth 流程不变）。
- 企业版本地 MCP（D2 已定不允许）；服务端 catalog 形状/管理台不改。
- 远端连接器生命周期（长驻进程守护/健康检查/崩溃重连）：KNOWN-LIMITATIONS 既有残余，不属本 spec。
- 独立 MCP store / per-server CRUD IPC / per-server revision：Phase 1 用 `AppSettings.mcp`（M-1）。

## 4. 用户与场景

| 用户 | 场景 | 期望 |
| --- | --- | --- |
| 个人用户 | 想接本地 MCP server（自建 stdio server 或本机 http） | 设置「MCP」pane 添加 server、测试连接看到 tool 列表、保存后 agent 能调用 `mcp_call` |
| 个人用户 | 配置写错/服务不可达 | 保存时字段级报错；测试连接给出可读错误，不改当前生效配置 |
| 从 env 迁移的用户 | 已有 `ZEOKEEL_MCP_SERVERS` | fresh install 仍走 env；已有 settings 文件的用户按 release note 在 UI 重建（M-4） |
| 企业用户 | 管理员启停连接器 | catalog 下发后生效（父 spec 的 `connectors.*` 接线）；本 spec 保证企业不走 env/local |
| 企业管理员 | 不希望客户端私加 MCP server | 企业构建无 MCP pane、`settings.set` 拒绝本地段、env 不生效（catalog-only） |

## 5. 成功指标

- `AppSettings.mcp` 保存/回读/校验全绿；**两级校验回归测试**：catalog 旧规则合法但超过本地上限的 server 仍能被 catalog 路径接受（P1-8）。
- 来源优先级测试：catalog（含空）> local（含空）> env；enterprise 下 env/local 均不生效（`connectors`/MCP 均为空并诚实不可用）。
- `mcp.test`：合法草稿返回 tool 列表；非法/不可达/超时返回封闭错误码；**探测后全局 `SESSIONS` 数量不变**、子进程退出（P1-9）；
  当前生效配置不变。
- 设置 UI：新增/编辑/删除/测试连接可用；六态、键盘、axe 0 serious/critical、zh/en 无硬编码。
- 组合：`zk validate` 绿且断言 enterprise 闭包不含 `zeokeel/ui-settings-mcp`；`zk compose --check` 绿；生成物 `--check` 绿（C10/C11）。
- e2e：`scripts/m1-e2e.sh` 下 MCP pane 添加 fake stdio server → 测试连接返回 tools → 保存 → 刷新保留（证据落 `target/evidence/2026-11-01-client-mcp-config/`）。

## 6. 风险与未知

| 风险/未知 | 影响 | 缓解/验证 |
| --- | --- | --- |
| 提升 `validate` 到契约导致 catalog 回归（P1-8） | 高 | 现有规则原样搬移（不改语义/上限），本地上限只在新 `McpSettings::validate`；catalog 兼容 golden 测试 |
| `mcp.test` 残留会话/子进程（P1-9） | 高 | 独立无缓存探测路径 + drop 时 kill；断言 `SESSIONS`/进程数不变；超时上限 ≤15s |
| 两段式探测增加 IPC 面（`mcp.test.status`） | 中 | 沿用 `oauth.authorize/status` 先例；命令名/状态枚举封闭；UI 轮询可取消 |
| enterprise env 关闭造成既有 env 用户「升级后 MCP 消失」 | 中 | T4 已定 catalog-only；CHANGELOG/release note + 未登录时诚实报不可用（不静默） |
| 新 personal-only UI 包遗漏 `personal-only.toml` 清单（P1-15） | 中 | 任务显式改数据列表 + `zk validate` 断言；enterprise compose 构建验证 |
| settings.json 变大/无 per-server revision（M-1 已定 AppSettings） | 低 | 上限（16 servers/参数/环境/长度）+ 单测；per-server revision 留后续 |
| 契约依赖 `zk-contract-config → zk-contract-connector`（P2-8） | 中 | 在 docs/24 §1、docs/04 §1 登记方向与兼容责任；无环（connector 契约无契约依赖） |
| 企业 e2e 需真实 server/PG | 中 | 企业侧由父 spec 的 `login-e2e.sh` 覆盖；本 spec e2e 用 personal `serve` + fake stdio server |

## 7. 决策（已定）

> 本 spec 保持 `draft` 直至独立评审复评通过（C16/C19）；下列决策**不再待拍板**（Lead 2026-11-01 定稿）。

**背景决策（前期已定）**

- [x] **T2 拆分**：MCP 部分从 config spec 拆出为独立 spec（本文件）。
- [x] **T4 企业版 catalog-only**：Phase 1 企业**不允许**本地 MCP；enterprise 下 **env `ZEOKEEL_MCP_SERVERS` 也不生效**；
      C1/C6 映射写入本 spec（R3/R8）与 `docs/24` §1 + `docs/07` §6.1；任务 1 = docs-first。
- [x] **Phase 1 结构**（用户已批 A 方案）：settings-ia → approval-tool-rendering → config-approval-mcp → **mcp-config**。

**本轮决策（Lead，2026-11-01）**

- [x] **M-1 `AppSettings.mcp.servers`**（不建独立 store）：复用 settings 存储/校验/失败不落盘/revision/pane 草稿；
      上限（16 servers/32 args/32 env/长度）防膨胀；per-server revision 留后续。
- [x] **M-2 本地 `oauth_required=true` 保存时拒绝**（本地无 authorize/token 元数据，无法完成 PKCE），提示改用企业 catalog。
- [x] **M-3 两段式 `mcp.test`（启动→返回 `test_id`）+ `mcp.test.status`（轮询 `pending|ok|error`）**，沿用 oauth 先例；
      探测无缓存（一次性 transport、drop 回收、硬超时 ≤15s）。
- [x] **M-4 env 不自动导入**：`ZEOKEEL_MCP_SERVERS` 仅在 personal 且无 settings 文件时生效；已有 settings 文件用户按
      release note 在 UI 重建（避免后台改写用户配置 / 清空后被灌回）。
- [x] **M-5 新 personal-only 包 `zeokeel/ui-settings-mcp`**（`ui.settings.pane` order 70；构建期 edition 差异，
      先例 `ui-settings-provider`）；enterprise exclude + `editions/personal-only.toml` 数据清单。
- [x] **M-6 不新增 ADR**：决策落 `docs/24` §1 / `docs/04` §1.5（契约依赖登记）+ `docs/07` §6.1 + `docs/16` + `docs/38`；
      任务 1 = docs-first。
- [x] **预算例外批准**（Lead 2026-11-01）：`client-mcp-config ~27 文件`、`config-approval-mcp ~16 文件`；
      理由 = 用户已批 Phase 1 四段结构 + 每份 spec 按 3 阶段可独立合并（详见 §8）。

## 8. 变更预算与拆分

- **Lead 批准例外（2026-11-01）**：评审 P1-3 的 10 文件阈值对本轮不适用——用户已批准 Phase 1 结构（四段 spec），
  且每份 spec 均按 3 个可独立合并阶段交付；本 spec ~27 文件、父 spec ~16 文件。
- **拆分记录（P1-3/T2）**：原 `client-config-approval-mcp` 34 文件，按评审建议拆出本 spec（MCP 配置/两级校验/`mcp.test`/
  `ui-settings-mcp`/企业 env 口径），父 spec 保留审批策略 + `connectors.*` 接线（约 14 个工作项/~16 文件）。
  两个 spec 各自目标单一、可独立合并。
- 本 spec 约 21 个工作项 / 约 27 个文件 + 文档 6；分 3 个可独立合并阶段：
  A 契约+运行时（任务 1–4）→ B 宿主+edition（任务 5–9）→ C UI+e2e+文档（任务 10–13）。
- **跨 spec 顺序与单 owner**：见 `tasks.md` §0（settings-ia → approval → config → **mcp-config**；本 spec 是最后阶段，
  对 `ipc 契约`/`editions`/`docs`/`e2e` 承担最终合并 owner）。

## 9. 复用与已有实现（C17）

- 已搜索（2026-11-01，`target/debug/zk`）：
  - `zk find mcp` → **0 命中**（`tool.mcp` 未登记 `@capability`，本 spec 不新增 capability id）
  - `zk find connector` → `contract.connector` `packages/contracts/connector/src/lib.rs`（1）
  - `zk find settings` → `store.settings.json`（1）；`zk find config` → `contract.config`（1）
- 可复用：`zk-contract-connector::McpServerConfig`（`packages/contracts/connector/src/lib.rs:170-199`，唯一 DTO）、
  `zk-connector-mcp` 的 `validate_server`（`:102-148`，提升到契约后委托）、`servers_from_catalog`/`update_from_catalog`/
  `reload_connectors`/`clear_catalog_config`/`configured_server_ids`（`connector-mcp:186-294`）、
  `list_server_tools`（`:442-451`，仅供 UI 展示，不用于无缓存探测）、settings 管线（`zk-contract-config` +
  `store-settings-json` + `settings.set` `apps/desktop/src/ipc.rs:531-592`）、`masked_settings`、`ui-settings-provider`
  personal-only 包四件套先例（`packages/ui/ui-settings-provider/zk.toml`）、settings-ia 的 `ui.settings.pane`/`SettingsPaneProps`。
- 本次新增可复用能力：`McpServerConfig::validate()`（从 client 包提升到 `zk-contract-connector`，既有
  `@capability contract.connector`）；`zk_connector_mcp::update_local_servers`/`clear_local_servers`/`set_env_fallback_enabled`/
  `test_server`（`tool.mcp` 角色内实现，不新增包）。
- 为什么不能复用/扩展：本地配置需要与 settings 同一份持久化/校验/失败不落盘语义（独立 store 会复制三套管线，C17）；
  无缓存探测不能用 `list_server_tools`（其全局会话缓存会污染进程状态，评审 F22/P1-9）。
