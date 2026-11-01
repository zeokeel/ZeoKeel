# 2026-11-01-client-config-approval-mcp · 提案

- 状态：`done`
- Owner：zk-audit（Phase 1c）
- 日期：2026-11-01
- 关联：`docs/reviews/2026-11-01-client-ui-parity-specs.md`（CHANGES-REQUESTED，本稿按其 P0-1/P1-1/P1-2/P1-3/P1-7/P2-1/P2-3/P2-4/P2-5/P2-6/P2-9 修复）；
  `research/client-dsh-parity-2026-11-01.md` §4/P0-4、§7 Phase 1；`target/audits/zk-client-inventory.md` §5/A1、A8；
  `docs/38` §2；`docs/19`；`docs/26` §5；`docs/07` §6.1；`KNOWN-LIMITATIONS.md` §3.6；
  拆出的 MCP 子 spec：`2026-11-01-client-mcp-config`
- 评审：docs/reviews/2026-11-01-client-ui-parity-specs.md
- 变更预算：拆分后本 spec 约 **14 个工作项 / 约 16 个文件**（契约 3、agent-rig 2、desktop 1、desktop-web 4、UI 2、e2e 2）+ 文档 5（docs/07、19、26、38、KNOWN-LIMITATIONS）+ CHANGELOG；
  仍略超模板 10 文件阈值（拆分理由与已批例外见 §8）

## 1. 背景 / 问题

审计与独立评审确认两处「配置链路断裂」：

1. **审批策略字段是死字段**：`AppSettings.auto_approve_read_only` 能存能读（`packages/contracts/config/src/lib.rs:586-587`、默认 `:615`）、
   表单层有映射（`packages/ui/ui-settings/web/src/settings-validation.ts:199,323`），但 **UI 无控件、运行时无消费者**：
   agent-rig 只按静态 `ToolSpec.requires_approval` 决定是否弹审批（`packages/client/agent-rig/src/lib.rs:272,303,322`）。
   用户无法表达「哪些工具需要确认」，字段也不生效。
2. **`connectors.*` 三条宿主命令未进契约、Web 零调用**：宿主已实现 `connectors.configure/reload/clear`
   （`apps/desktop/src/ipc.rs:630-650`），但 `packages/contracts/ipc/src/lib.rs:80-146` 无这三条，Web 侧 `rg connectors\. apps/desktop/web/src` 0 命中；
   后果：企业连接器启停/目录更新后**本地 MCP catalog 不重载**，要重新登录才生效。而 `docs/07:198-200` 与
   `KNOWN-LIMITATIONS.md:99-100` 声称「更新事件可经 IPC … 触发（web 侧接线）」——**文档与代码冲突**（C12：先改文档再改代码）。

独立评审 `docs/reviews/2026-11-01-client-ui-parity-specs.md` 就本 spec 的结论：**CHANGES-REQUESTED**。
其中 P0-1（审批「安全不变式」与 4 个 `mutating && !requires_approval` 工具矛盾）、P1-3（34 文件超拆分阈值）、
P1-7（未知枚举降级无法用普通 serde 枚举实现）为阻塞项；P1-8/P1-9/P1-10/P1-15（MCP 相关）随 MCP 拆分移出。
本稿按 Lead 决策（T1 弱不变式、T2 拆分、T4 企业 catalog-only）修复：**MCP 部分已拆到 `2026-11-01-client-mcp-config`**，
本 spec 只保留「审批策略消费链 + `connectors.*` 契约接线」。

## 2. 目标

- **G1 审批策略真正生效（弱不变式，策略求值范围内；S4 always-allow 为用户显式授权例外）**：新增封闭枚举 `ApprovalPolicy { ask(默认), auto_read_only, strict }`，
  由 agent-rig 在每次工具调用前按 `ToolSpec.requires_approval` + `ToolSpec.mutating` 求值。
  **安全不变式（T1 选项①）**：任何策略都**不得自动批准 `requires_approval == true` 的工具**（不可再弱化的硬约束）。
- **G2 诚实处理 4 个「mutating 但声明免审批」工具**：`schedule`/`job_kill`/`goal`/`mcp_call` 在 `ask`（默认）下
  不询问用户（既有行为，工具作者声明自洽），在 spec/design 显式列出并给理由 + 负向测试断言，不伪装成「所有 mutating 都会问」。
- **G3 审批事件链边界清晰**：本 spec 只负责「策略说不需要询问时**不产生任何审批事件**」；
  `always-allow（会话允许）`、审批决策 wire、事件对序列归 spec `2026-11-01-client-approval-tool-rendering`（单 owner，见 §8/tasks §0）。
- **G4 `connectors.*` 进契约并接线**：3 条命令进 `zk-contract-ipc`（常量 + payload/result DTO + schema/TS 生成）；
  Web 在「目录拉取成功 / 连接器启停 / 重新加载 / 登出」时调用；幂等、失败可见。
- **G5 文档先行与口径修正**：审批策略与 `connectors.*` 契约先落到 `docs/38`/`docs/19`/`docs/26`/`docs/07`（任务 1），
  再改代码；`KNOWN-LIMITATIONS` 的「web 侧接线」口径按实现修正。
- **G6 门禁全绿**：`spec-check`/`docs-check`/`review-check`、`zk validate`、`zk compose --check`、`zk capability --check`、
  `bash scripts/schema-check.sh`、`cargo test --workspace export_schema && git diff --exit-code`、
  `cargo test -p zk-contract-config -p zk-contract-ipc -p zk-agent-rig -p zk-desktop`、
  `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build`、`bash scripts/i18n-check.sh`、
  `bash scripts/agent-e2e.sh`、`bash scripts/login-e2e.sh`。

## 3. 非目标

- **MCP 配置**（`AppSettings.mcp`、`McpServerConfig::validate` 提升、`mcp.test`、`ui-settings-mcp` pane、企业 env 口径）：
  已拆到 `2026-11-01-client-mcp-config`，本 spec 不写。
- **审批卡 UI 字段**（`docs/19` §4.1 九项）、工具行持久化/`role=tool` 渲染、`usage` 事件、`always-allow` 会话记忆、
  `approval.decide` wire 迁移：归 `2026-11-01-client-approval-tool-rendering`。
- **设置 IA**（pane slot/revision/外壳）：归 `2026-11-01-client-settings-ia`；本 spec 只消费其 `ui.settings.pane`
  与「工具与审批」pane（order 40）。
- 企业审批/策略下发/工具白名单/网络限制/审批卡扩展：不做（C6 弱管控；ADR-030/032）。
- 把 `ToolSpec` 语义改为「`mutating && !requires_approval` 为非法」：不做（T1 选项②/③未采用）；4 个工具声明保持现状。

## 4. 用户与场景

| 用户 | 场景 | 期望 |
| --- | --- | --- |
| 个人用户 | 不想每次 `fs.write`/`bash` 都确认，或想让只读/交互工具也不打扰 | 设置「工具与审批」选 `ask`/`auto_read_only`/`strict`，保存后下一轮生效（免重启） |
| 个人用户 | 对 mutating 工具想更严 | `strict`：每次工具调用都先确认（含 `fs.read`/`glob`） |
| 安全敏感用户 | 想知道哪些 mutating 工具默认不确认 | 文档/spec 显式列出 4 个工具与理由，不隐藏 |
| 企业员工 | 管理员启停连接器后立刻要用 | 连接器面板启停后**无需重登**，本地 MCP 按新 catalog 重载 |
| 企业管理员 | 不希望客户端私改连接器执行配置 | `connectors.*` 只接受 catalog，Web 不能注入任意 server 列表（MCP 本地配置属另一 spec 且企业禁用） |

## 5. 成功指标

- **真值表**（3 策略 × 4 组合）全绿，且**弱不变式**有负向测试：`requires_approval == true` 时任何策略都返回「需要询问」。
- 4 个 `mutating && !requires_approval` 工具在 `ask`（默认）下**不询问**；按 C-1=A，`auto_read_only`/`strict` 下**询问**，有单测断言并显式记录。
- 未知枚举值（如 `"approval_policy":"bogus"`）不导致 `settings.load()` 失败，降级为 `ask`（有 4 形态 + 未知值测试）。
- `strict` 下 `fs.read`（只读）出现审批卡、`ask` 下不出现，切换策略后**下一轮**生效（`bash scripts/agent-e2e.sh`）。
- 企业连接器：启停后 Web 依次调用 `connectors.configure`（携带最新 catalog）→ host 重载；「重新加载」→ `connectors.reload`；
  登出 → `connectors.clear`；三条命令有契约 schema 与 dispatch 单测（`bash scripts/login-e2e.sh` 覆盖 UI 链路）。
- 文档冲突清零：`rg "web 侧接线" docs KNOWN-LIMITATIONS.md` 无命中（改为已接线口径）。
- 全部门禁（G6）绿，证据落 `target/evidence/2026-11-01-client-config-approval-mcp/`。

## 6. 风险与未知

| 风险/未知 | 影响 | 缓解/验证 |
| --- | --- | --- |
| `ToolSettings` 新增 `Option<ApprovalPolicy>` 影响「默认-only 不发布 → env fallback 生效」语义 | 中 | 规范化：`ask` 存 `None`；只有 `auto_read_only`/`strict` 非默认；4 种文件形态单测 |
| `auto_read_only` 语义（C-1=A 已定）影响用户预期 | 中 | 真值表按 A 定稿（只读自动 + mutating 全询问）；若后续改选只改 R1 一行 + 枚举 |
| 旧字段映射（C-2）引发误判 | 中 | `false→strict`、`true→ask` + 迁移/幂等测试；CHANGELOG 写 deprecated |
| 未知枚举降级（P1-7）用普通 derive 会加载失败 | 高 | 自定义 `Deserialize`（字符串→`parse`，未知→`Ask`）+ 未知值测试；schema 仍列 3 值 |
| 与 approval spec 在 `agent-rig`/`ipc.rs` 同文件 | 高 | 跨 spec 串行 + 单 owner（tasks §0）；边界写进 R2 |
| `connectors.*` payload 形状（C-3 已定：沿用现状）与 host 一致性 | 中 | `{catalog}` + `{configured:n}`；host 现有分支已兼容对象/字符串（`ipc.rs:630-642`） |
| 企业 e2e 依赖本机 PG（`scripts/login-e2e.sh`） | 中 | 无 PG 时标 UNVERIFIED，用 dispatch 单测 + web 单测兜底；不冒充已验 |
| `strict` 对纯交互工具的 UX 双重交互（P2-6） | 中 | C-5 已定：Phase 1 不豁免，记为已知 UX（spec §2 + acceptance §5）；Phase 2 再评估豁免清单 |
| docs 先行被跳过 | 高 | 任务 1 = 文档落定，未完成不得开始契约/代码任务 |

## 7. 决策（已定）

> 本 spec 保持 `draft` 直至独立评审复评通过（C16/C19）；下列决策**不再待拍板**（Lead 2026-11-01 定稿）。

**背景决策（前期已定）**

- [x] **T1 弱不变式**：安全不变式 = 「任何策略都不得自动批准 `requires_approval == true` 的工具」；
      显式列出 4 个 `mutating && !requires_approval` 工具（`schedule`/`job_kill`/`goal`/`mcp_call`）与「工具作者声明自洽」理由；
      真值表保留 `(false,true)` 组合并在 `ask` 下断言不询问（诚实说明）；范围限定在策略求值，S4 例外归 approval spec。
- [x] **T2 拆分**：MCP 部分拆为 `2026-11-01-client-mcp-config`；本 spec 保留审批策略 + `connectors.*` 接线。
- [x] **Phase 1 结构**（用户已批 A 方案）：settings-ia → approval-tool-rendering → config-approval-mcp → mcp-config 串行。

**本轮决策（Lead，2026-11-01）**

- [x] **C-1 选 A**：`auto_read_only` = 只读工具自动放行 + **所有 mutating 工具仍询问** →
      `should_ask = requires_approval || mutating`；真值表按此定稿（含 `(false,true)` 组合在 `auto_read_only` 下为「询问」）。
- [x] **C-2**：旧字段映射 `auto_approve_read_only=false → strict`、`true → ask`（字段 deprecated；映射后默认行为不变）。
- [x] **C-3**：`connectors.*` 形状沿用现状 —— `connectors.configure {catalog}` → `{configured:n}`；
      `connectors.reload {}` → `{reloaded:true}`；`connectors.clear {}` → `{cleared:true}`；错误码封闭集合见 spec §2。
- [x] **C-4**：审批策略控件放 settings-ia「工具与审批」pane（order 40）；MCP 单独 personal-only pane（见 MCP spec）。
- [x] **C-5**：Phase 1 `strict` **不豁免**纯交互工具；「先审批再提问」的双重交互作为**已知 UX**记入 spec §2 与
      acceptance §5 剩余风险（Phase 2 再评估豁免白名单）。
- [x] **C-6**：**不新增 ADR**；决策落 `docs/07` §6.1 + `docs/38` §2；任务 1 = docs-first（未完成不得开工）。
- [x] **预算例外批准**（Lead 2026-11-01）：`config-approval-mcp ~16 文件`、`client-mcp-config ~27 文件`；
      理由 = 用户已批 Phase 1 四段结构 + 每份 spec 按 3 阶段可独立合并（详见 §8）。

## 8. 变更预算与拆分

- **Lead 批准例外（2026-11-01）**：评审 P1-3 的 10 文件阈值对本轮不适用——用户已批准 Phase 1 结构（settings-ia +
  approval-tool-rendering + config-approval-mcp + mcp-config 四段），且每份 spec 均按 3 个可独立合并阶段交付；
  本 spec ~16 文件、MCP spec ~27 文件。
- **拆分记录（P1-3/T2）**：拆分前 34 文件（评审核算），按评审建议拆出 `2026-11-01-client-mcp-config` 后，
  本 spec 约 **14 个工作项 / 约 16 个文件**（契约 3、agent-rig 2、desktop 1、desktop-web 4、UI 2、e2e 2 + 文档 5 + CHANGELOG 1）；
  MCP 目标（`AppSettings.mcp`、校验提升、`mcp.test`、`ui-settings-mcp`、企业 env 口径）全部移出。
- **跨 spec 顺序与单 owner**：见 `tasks.md` §0（settings-ia → approval-tool-rendering → config-approval-mcp → mcp-config；
  `ipc 契约`/`agent-rig`/`ui-kit`/`i18n`/`docs` 的最终合并 owner 已指定）。

## 9. 复用与已有实现（C17）

- 已搜索（2026-11-01，`target/debug/zk`）：
  - `zk find approval` → `kit.approval` `crates/zk-kit-approval/src/lib.rs`（1 match）
  - `zk find connector` → `contract.connector` `packages/contracts/connector/src/lib.rs`（1 match）
  - `zk find settings` → `store.settings.json` `packages/client/store-settings-json/src/lib.rs`（1 match）
  - `zk find config` → `contract.config` `packages/contracts/config/src/lib.rs`（1 match）
- 可复用：`zk-contract-tool::ApprovalGate`（`packages/contracts/tool/src/lib.rs:55-75`）、`kit.approval::GlobalGate`、
  `zk-contract-config` settings/RuntimeSettings 管线、`AgentRuntimeConfig` 每轮热读（`packages/client/agent-rig/src/config.rs:455-475`
  `default_agent_config`、`lib.rs:163-169` `effective_config`）、`apps/desktop/src/ipc.rs` 的 `apply_connector_catalog`/
  `sync_connector_catalog`/`restore_connector_oauth_tokens` 先例（`:338-410`）、settings-ia 的 `ui.settings.pane` 与
  `SettingsPaneProps`（依赖 spec）。
- 本次新增可复用能力：`ApprovalPolicy::should_ask`（放 `zk-contract-config`，单一求值实现，登记在既有
  `@capability contract.config` 下，不新增 capability id）；`connectors.*` 的 IPC DTO（`zk-contract-ipc`，既有 capability）。
- 为什么不能复用/扩展：审批策略需要 `ToolSpec.mutating` + 逐轮热读，只有 agent-rig 的 DynamicTool 回调同时拥有
  `spec` 与 `ApprovalGate`（`packages/client/agent-rig/src/lib.rs:260-350`）；放到桌面 broker 需要第二份「工具是否只读」
  知识（C17 重复，且 broker 只收到 name/args）。`connectors.*` 已在宿主实现（只差契约与接线），无第二份实现。
