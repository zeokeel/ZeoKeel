# 2026-11-01 · Phase 1 客户端三份 spec（settings IA / 审批渲染 / 配置接线）C19 独立评审

- 评审对象：
  - `specs/changes/2026-11-01-client-settings-ia/`（Lead 作者，5 文件）
  - `specs/changes/2026-11-01-client-approval-tool-rendering/`（Lead 作者，5 文件）
  - `specs/changes/2026-11-01-client-config-approval-mcp/`（zk-audit 作者，5 文件）
  - 基线 commit：`HEAD 6a6d24a`（`git rev-parse HEAD`）；**三份 spec 均为工作树未提交修改**（`git status --short` 仅 7 个 spec 文件 M，无产品代码/生成物改动），故评审对象 = HEAD + 下列文件 SHA256（2026-11-01 14:0x 工作树快照）。
- 评审者：`spec-reviewer`（只读评审；本会话唯一写入 = 本文件）。
- 与实现/作者关系（独立性）：**独立**。未参与三份 spec 的写作（作者为 `lead` 与 `zk-audit`）、未参与任何实现、未修改任何 spec/代码；`research/client-dsh-parity-2026-11-01.md` 与 `target/audits/*` 仅作「待核验的声明」，本文全部事实结论以当前代码/生成物/命令输出为准（不照抄 spec 或审计结论）。
- 关联 spec：`2026-11-01-client-settings-ia`、`2026-11-01-client-approval-tool-rendering`、`2026-11-01-client-config-approval-mcp`。
- 结论：**CHANGES-REQUESTED**（P0=4 / P1=16 / P2=10）。
- 说明：三份 spec 的用户已拍板背景（A 方案、Phase 1 = 设置 IA + P0 修复 + 配置接线）本文按既定前提评审；本文不重新裁决产品范围，只裁决「spec 与代码/约束是否一致、是否可实现、是否可验收」。

## 0. 评审对象快照（SHA256）

```text
57a3e61e44f1feaed17f35edc3e481e96bf431c7bb26ac81dbc313e24acd676a  specs/changes/2026-11-01-client-settings-ia/acceptance.md
bd3f59b20a27f0d49cb1df0821d35a68f2171848a0be5f3807988d9811070a54  specs/changes/2026-11-01-client-settings-ia/design.md
eb3917022c833d2ba1fc51e36fa514f16f32f4fd2f33ea356907f92157b780d8  specs/changes/2026-11-01-client-settings-ia/proposal.md
9a664bbcd49c769192341479b94e00a46f725536cef3bd5c04cb1ad0050913d9  specs/changes/2026-11-01-client-settings-ia/spec.md
cb740a43db67c9bfbace6d4f919edf6e42cd864ba0ba742123667a111b1d08d9  specs/changes/2026-11-01-client-settings-ia/tasks.md
cc4e8853e399fdbd4bd14e52fafe467b6b83b56328ffefef6677e9b265586353  specs/changes/2026-11-01-client-approval-tool-rendering/acceptance.md
d7560f262cf6ccfa7376c3057784604f4355e173cb8e2ac14194c529638be1de  specs/changes/2026-11-01-client-approval-tool-rendering/design.md
750b829ef03b1ce2e3cbc101c374c7d399c1866dad7f2055c9d1667c90201448  specs/changes/2026-11-01-client-approval-tool-rendering/proposal.md
f6cf0fa4a96dfe7cfd0a6c5c23a45f203560272ed4b120b1a0e4a70bbbc7fbf5  specs/changes/2026-11-01-client-approval-tool-rendering/spec.md
4b8765fac2d28eb8245ca10dcdbe048073ecfbf1e189ed0315da461eb211aceb  specs/changes/2026-11-01-client-approval-tool-rendering/tasks.md
270f66398f2dfd85b9c20962fecc9c3ee7148eca8eb09791e23dc9f0763449b4  specs/changes/2026-11-01-client-config-approval-mcp/acceptance.md
019221e46d7ea786e85315f0a4740db6e76503be7bad986631131c196ec33deb  specs/changes/2026-11-01-client-config-approval-mcp/design.md
030d0ab77b0c2612387400bbd0263d7d9eb6d59e0a656adce310cac62a88e49b  specs/changes/2026-11-01-client-config-approval-mcp/proposal.md
a1e9d65fcbf4b37a765cd18fbe475bba0636de34f8d7e76dd2ba19584b6452f3  specs/changes/2026-11-01-client-config-approval-mcp/spec.md
1de16c5930c2eb0bb289d95d9201cdda492fe26fa194c8026188d21c6c39d71d  specs/changes/2026-11-01-client-config-approval-mcp/tasks.md
```

> 记录失效条件（`docs/reviews/README.md`）：上述任一文件变更即本记录失效，须重新评审。复评前请勿把 `proposal.md` 状态改为 `approved`。

## 1. 事实核验（spec 声明 ↔ 代码；命令与 exit code 均为本次独立执行）

| # | spec 声明（出处） | 本次执行的核验命令 | 实际结果 | 判定 |
| --- | --- | --- | --- | --- |
| F1 | 设置面 = 一个 618 行单页 dialog，内容区 `index.tsx:165-606`（settings-ia proposal:15、parity §0） | `wc -l packages/ui/ui-settings/web/src/index.tsx`；`sed -n '160,180p' packages/ui/ui-settings/web/src/index.tsx` | `618`；第 165 行起 `<form>`，第 606 行附近 dialog 结束 | 成立 |
| F2 | `AppSettings` 40 个叶子字段、personal 可编辑 37（settings-ia proposal:12-13） | 结构递归计数脚本（python3，config/src/lib.rs）；`grep -n 'autoApproveReadOnly' packages/ui/ui-settings/web/src/index.tsx` | 40 个叶子字段；`autoApproveReadOnly` 在 `index.tsx` 无控件（grep exit=1）；40 − schema/auto_approve/server_url = 37 可编辑 | 成立 |
| F3 | `settings.set` 提交整份文档且无 revision 栅栏（settings-ia proposal:19；config spec:159） | `sed -n '245,249p' packages/contracts/ipc/src/lib.rs`；`grep -n 'SETTINGS_GET =>\|SETTINGS_SET =>' apps/desktop/src/ipc.rs` | `SettingsSetPayload { settings: AppSettings }` 仅一个字段；`ipc.rs:527/531` 分别 `masked_settings` 与全量保存，无 revision | 成立 |
| F4 | 唯一注入点 = `renderProvider`（`ui.settings.provider` slot），其余分区写死在 dialog（settings-ia proposal:17） | `grep -n 'renderProvider' packages/ui/ui-settings/web/src/index.tsx`；`cat packages/ui/ui-settings-provider/zk.toml`；`grep -n 'ui.settings.provider' tools/zk/src/main.rs` | `index.tsx:40,166`；provider 包 `slot = "ui.settings.provider"`；生成器 `main.rs:2780` 映射 | 成立 |
| F5 | 生成器「一个 slot 只绑定一个组件」，多贡献需改生成器（settings-ia proposal:65；design:27） | `sed -n '2770,2784p' tools/zk/src/main.rs`（`slot_binding` 返回单个名字）；`sed -n '2866,2871p' tools/zk/src/main.rs`（每个 contribution 生成 `export const {binding} = cN["component"]`） | 同一 slot 名重复绑定会互相覆盖；无数组/list 机制 | 成立 |
| F6 | `connectors.*` 三条命令未进契约、Web 零调用（config proposal:25-29） | `grep -n 'connectors\.' packages/contracts/ipc/src/lib.rs`（exit=1，0 行）；`grep -rn 'connectors\.' apps/desktop/web/src`（exit=1，0 行）；`grep -n '"connectors.configure"\|"connectors.reload"\|"connectors.clear"' apps/desktop/src/ipc.rs` | 契约无；Web 无；宿主 dispatch 在 `ipc.rs:630/643/647` | 成立 |
| F7 | agent-rig 审批判定点 = `spec.requires_approval`（`lib.rs:272,303`），与字段值无关（config proposal:20） | `grep -n 'requires_approval\|approval.decide' packages/client/agent-rig/src/lib.rs` | `:272` 读取、`:303` 判定、`:322` 调 gate；全文件无 `auto_approve_read_only` | 成立 |
| F8 | `ApprovalBroker`/`BridgeSink` 存在，`ToolCallFinished` 写 `role="tool"` JSON（approval proposal:16-18；design:21,24） | `grep -n 'impl ApprovalGate for ApprovalBroker' apps/desktop/src/approval.rs`；`sed -n '64,78p' apps/desktop/src/events.rs` | `approval.rs:83`；`events.rs:64-78` 写 `{name,result}` + role tool | 成立 |
| F9 | `role="tool"` 行被当 assistant 原文渲染（approval proposal:17） | `sed -n '140,155p' apps/desktop/web/src/state/reducer.ts`；`sed -n '45,60p' packages/contracts/session/src/lib.rs` | `toChatMessage` 把非 user 全部 → assistant（`reducer.ts:141-153`）；Message 注释确认 role 可为 tool | 成立 |
| F10 | `usage` 事件被 parseAgentEvent default→null（approval proposal:19-20） | `grep -n 'case "' apps/desktop/web/src/protocol.ts`；`sed -n '665,672p' apps/desktop/web/src/protocol.ts`；`grep -n 'Usage {' packages/contracts/agent/src/lib.rs` | 无 `case "usage"`（grep exit=1）；`protocol.ts:669-670` default return null；契约 `agent/src/lib.rs:140` 有 `Usage` | 成立 |
| F11 | i18n 门禁当前红（approval proposal:21-22） | `bash scripts/i18n-check.sh`；`sed -n '35,60p' apps/desktop/web/src/main.tsx` | **exit=1**，报 `main.tsx:47`「ZeoKeel 启动失败」、`:49`「请截图此页面并反馈问题。」 | 成立 |
| F12 | `auto_approve_read_only` 有存储/回读/表单/i18n、无控件、无运行时消费者（config proposal:17-21） | `grep -rn 'auto_approve_read_only' packages apps crates --include=*.rs --include=*.ts --include=*.tsx`；`grep -n 'autoApproveReadOnly' packages/ui/ui-settings/web/src/index.tsx` | 仅出现于 config 契约/测试、`ui-kit/web/src/types.ts:122`、`settings-validation.ts:199,323`、`protocol.ts:416-417`；UI 无控件（exit=1）；agent-rig/工具无消费 | 成立 |
| F13 | MCP 配置来源仅 env + 企业 catalog（config proposal:22-24） | `sed -n '29p'`、`grep -n 'fn servers_from_env\|fn servers_from_catalog\|fn update_from_catalog' packages/client/connector-mcp/src/lib.rs` | `SERVERS_ENV` 在 `:29`；`servers_from_env`/`servers_from_catalog`/`update_from_catalog` 均无 settings 路径 | 成立 |
| F14 | `docs/07:198-200` 与 `KNOWN-LIMITATIONS.md:99-100` 声称 connectors「web 侧接线」与代码冲突 | `sed -n '195,205p' docs/07-server-domains.md`；`sed -n '95,102p' KNOWN-LIMITATIONS.md`；配合 F6 | 两处均写「web 侧接线」，但契约/Web 均无实现 | 成立（文档需先改，C12/C19） |
| F15 | personal-only 机制：`editions/personal-only.toml:3` 清单 + `editions/enterprise.toml:94` exclude（config design:16） | `cat editions/personal-only.toml`；`sed -n '88,96p' editions/enterprise.toml`；`grep -n 'ui.slot' editions/personal.toml` | `personal-only.toml` 第 3 行是 `packages = [...]` 数据列表；enterprise `:94` exclude；personal `:58` 映射 provider | 成立（但 design §7 把该文件写成「注释同步」有误，见 P1-15） |
| F16 | 复用证据：`zk find approval/connector/settings/config` 各 1 命中、`mcp` 0（config proposal:155-177；acceptance:84-97） | `./target/debug/zk find approval` 等 5 条 | 分别为 `kit.approval`、`contract.connector`、`store.settings.json`、`contract.config`、0；exit 全 0 | 成立 |
| F17 | 「把 `@capability` 登记到 `ui-kit/web/src/types.ts` + `docs/generated/CAPABILITY-INDEX.md`（由 zk capability 生成）」（settings-ia design:28、proposal:91） | `sed -n '3070,3106p' tools/zk/src/main.rs`（扫描根 = `packages/<group>/<pkg>/src` + `crates/zk-kit-*/src`）；`sed -n '3165,3186p' tools/zk/src/main.rs`（`walk_rs` 只收 `.rs`）；`grep -n 'ui\.' shared/contracts/capability-index.json`（0 命中） | 扫描器只读 Rust `src/`，不读 `web/src/*.ts`；INDEX 45 条，`ui.*` 为 0 条 | **不成立**（见 P1-4） |
| F18 | 「`ApprovalDecidePayload` 复用枚举，无需新命令」且 `Cancel` 不可达（approval design:10；proposal:15） | `sed -n '250,258p' packages/contracts/ipc/src/lib.rs`；`grep -n 'decision == "approved"' apps/desktop/src/ipc.rs`；`grep -n '"approved" \| "rejected"' apps/desktop/web/src/state/controller.ts`；`grep -n 'ApprovalDecidePayload' -r .` | 契约 DTO 用 `ApprovalDecision` 枚举（`allow_once/reject/cancel`）；但宿主 dispatch `:735` 用字符串 `decision == "approved"`，前端 `controller.ts:667` 只发 `approved/rejected`；该 DTO 在运行期无调用者 | 契约 DTO 与实现已漂移（见 P1-11） |
| F19 | 工具行「归并到前一条 assistant 消息」（approval proposal:32） | `sed -n '64,78p' apps/desktop/src/events.rs`；`sed -n '1776,1783p' apps/desktop/src/ipc.rs`；`sed -n '1842,1856p' apps/desktop/src/ipc.rs`；`sed -n '649,652p' packages/client/store-session-sqlite/src/lib.rs` | 工具行在 `ToolCallFinished` 时立即 append（created_at=当时）；assistant 消息在整轮结束时 append（created_at=完成时）；会话查询 `ORDER BY created_at_ms ASC, id ASC` ⇒ 持久化顺序是 user → tool… → assistant，**工具行在前、assistant 在后** | **声明方向错误**（见 P0-3） |
| F20 | 刷新后工具卡状态含 `pending/running/success/failed/cancelled`（approval spec:36-37） | `grep -n 'denied by approval policy\|tool call cancelled\|ToolCallFinished' packages/client/agent-rig/src/lib.rs` | 拒绝路径 `:345-358` 与取消路径 `:357-358` 提前 return，**不发** `ToolCallStarted/ToolCallFinished`；仅 `:397/:422` 成功/失败发 Finished；BridgeSink 只持久化 Finished | 拒绝/取消工具行不会落库，五态不可达（见 P0-3） |
| F21 | 「当前 41 个工具里没有 (true,false) 组合」+「任何策略都不得自动批准 mutating 工具」（config design:93；spec:18,21-29） | 脚本枚举 `mutating: true` 且 `requires_approval: false` 的 ToolSpec；`grep -n 'requires_approval: false'` 于 tool-schedule/job-kill/goal/connector-mcp | 存在 4 个 mutating 且不声明审批的工具：`schedule`（tool-schedule:79-80）、`job_kill`（tool-job-kill:15-16）、`goal`（tool-goal:43-44）、`mcp_call`（connector-mcp:873-874）；按 design 真值表 `ask/auto_read_only` 对 (requires_approval=false, mutating=true) 返回 false ⇒ **会不经审批直接执行** | **安全不变式不成立**（见 P0-1） |
| F22 | `mcp.test` 「临时进程/连接在使用后关闭」，不改变生效配置（config spec:161） | `grep -n 'fn with_session' -A8 packages/client/connector-mcp/src/lib.rs`；`grep -n 'static SESSIONS'`；`grep -n 'fn list_server_tools'` | `with_session` 走全局 `SESSIONS` 缓存（`:261`）并复用（`:406-413`）；`list_server_tools` 用 `with_session`（`:442-444`），无显式 close/kill | 与「临时关闭」声明不符（见 P1-9） |
| F23 | 把 `validate_server` 提升到契约、「规则原样 + 上限」（config design:228-233） | `sed -n '102,148p' packages/client/connector-mcp/src/lib.rs`；`grep -n 'fn valid_server_id' packages/client/connector-mcp/src/lib.rs`；`grep -n 'validate_server(&server)' packages/client/connector-mcp/src/lib.rs` | 现有 validate 不校验 id 字符集（`valid_server_id` 是独立函数 `:307-313`）、无长度/条数上限；catalog 路径 `servers_from_catalog` `:200` 复用同一 validate | 新上限/字符集若放进共享 validate 会一并收紧企业 catalog（见 P1-8） |
| F24 | 审批卡字段覆盖 `docs/19` §4.1 100%（approval proposal:57） | `sed -n '91,110p' docs/19-client-ux-flows.md`；`sed -n '16,23p' specs/.../approval/spec.md` | §4.1 还要求「网络目标（域名/端口）」「超时…可延长一次」「记住范围（仅该会话/该工具/该路径）」；§4.2 还有「危险操作二次确认」；spec R1/R2 未覆盖 | 100% 声明不成立（见 P1-12） |
| F25 | 门禁/结构现状（作为 spec 可验收性的基线） | `python3 scripts/spec-check.py`、`python3 scripts/docs-check.py`、`python3 scripts/review-check.py`、`./target/debug/zk validate`、`./target/debug/zk compose --edition personal --profile default --channel official --check`、`./target/debug/zk capability --check`、`bash scripts/schema-check.sh`、`bash scripts/i18n-check.sh` | spec-check **exit 0**（0 errors / 3 draft warnings）；docs-check **0**（41 files）；review-check **0**（28 records，评审前）；zk validate **0**；compose --check **0**；capability --check **0**（45 entries）；schema-check **0**；i18n-check **1**（即 F11） | 基线绿（i18n 红） |

> 表中未标注 exit 的行，命令均返回 exit 0；显式标注 `exit=1` 的是「grep 无命中」或「i18n 门禁红」（属预期事实，不是评审失败）。F25 的完整门禁 exit code 见行内。
> 未执行：`cargo test --workspace`、`pnpm -r typecheck/lint/test:unit/build`、`pnpm e2e`（本评审只读、未改代码；且三份 spec 仍 `draft`，C16 禁止开工）。上述门禁应由实现阶段按 spec 验收表执行。

## 2. C1–C19 合规检查

| 约束 | 判定 | 依据/对应项 |
| --- | --- | --- |
| C1 个人版优先 / 企业=个人+管控包 | OK | 新包 `ui-settings-mcp` 用 `[role].editions=["personal"]` + enterprise exclude（config design:11,16）；无企业基线复制。 |
| C2 构建期组合 | OK（一处措辞风险） | pane 贡献/包选择均构建期；`SettingsPaneProps.edition` 运行期分支建议改为组合/构建期注入（P2-7）。 |
| C3 一切皆包 / 四件套 | 风险 | 新包四件套任务已列（config tasks:17）；但「contract 验证」依赖提升到契约 crate 属公共 API 变更，需 public-api 证据（见 P1-8 的兼容性要求）。 |
| C4 契约唯一稳定缝 | OK（记录耦合风险） | `zk-contract-config → zk-contract-connector` 为 contract→contract，符合「只依赖 zk-contract-*」；connector 契约无契约依赖（已核 `packages/contracts/connector/Cargo.toml`），无环。但 settings schema 从此耦合 connector DTO，需在 docs/24/04 记录（P2-8）。 |
| C5 requires 只写角色 | OK | edition 映射写角色（`ui.slot.*`），未写包名到 requires。 |
| C6 企业弱管控 | **待拍板** | 客户端本地审批策略符合 docs/04 §1.5 C1；但 D2「企业版本地 MCP 不允许」与「客户端能力完整」的措辞张力未在 ADR/docs 落定（P1-10 与 T4）。 |
| C7 企业 AI 只走网关 | OK | 三 spec 未改 provider/gateway 路径；settings-ia 仅迁移既有 enterprise 分区。 |
| C8 无部门/用户组 | n/a | 不涉及。 |
| C9 TS/Rust 最严格检查 | OK（spec 层） | 设计片段无 `any`/`as`/`unwrap`；实现阶段按 `docs/18`。 |
| C9b 零 unsafe | OK（spec 层） | 无 unsafe/FFI 变更。 |
| C10 外部输入运行时校验 | **风险** | 契约新增字段/命令普遍缺少「生成 schema → Ajv/guard + 负向测试」的验收项（P1-5）；settings-ia 完全未提 C10/ADR-059。 |
| C11 生成物只由生成器写 | **风险** | settings-ia/approval 改了契约却未把 `cargo test --workspace export_schema`、`schema-check.sh` 放进 tasks/acceptance（P1-5）。 |
| C12 决策先文档后代码 | **违规风险** | 新增机制（多贡献 pane slot、审批策略枚举、MCP 进 settings）尚无 docs/ADR 落定；docs 更新被排到 tasks 末尾（P1-2）。 |
| C13 更新通道固定 | OK | 未触碰 update 实现/通道。 |
| C14 渠道决定登录地址形态 | 风险 | settings-ia 迁移 `server_url` 到「企业」pane，但未把「渠道锁定只读 / 改地址需重登」列成回归验收（P2-10）。 |
| C15 参考仓库只作参考 | OK | 三 spec 未引入 DSH 依赖/资产；`research` 仅事实参考。 |
| C16 无 spec 不开工 | 风险 | 五件套齐全、状态 `draft` ✓；但 config proposal §7 已勾选 D1–D8「已定」而 spec §6 仍列「未决问题」，决策状态自相矛盾（P1-2）。 |
| C17 复用/单实现 | **风险** | capability 登记位置错误（P1-4）；审批元数据在 agent-rig 重新推导工具路径/diff，与工具层知识重复（P1-16）。 |
| C18 对话归属工作区 | n/a | 不涉及会话归属语义变更。 |
| C19 发现即文档/评审后施工 | **本评审执行** | 本文为唯一写入；spec 仍 `draft`，批准前不得写代码。 |

## 3. 三份 spec 的依赖顺序与写作用域冲突

**已声明的依赖**（成立）：config → settings-ia（config spec 前置说明 `spec.md:4-5`；tasks:17,18 标「settings-ia 任务 2/4/3/5」）；config 自身四阶段（proposal:147-153）。

**未声明顺序的写作用域重叠（P1-1）**——以下文件被两份或三份 spec 同时声明写，且 approval 与另外两份无任何依赖声明：

| 文件/目录 | settings-ia（tasks 行） | approval（tasks 行） | config（tasks 行） | 冲突 |
| --- | --- | --- | --- | --- |
| `packages/contracts/ipc/src/lib.rs` | :9（任务 1） | — | :13（任务 4） | settings-ia ↔ config 无顺序 |
| `packages/contracts/tool/src/lib.rs` | — | :9（任务 1） | 设计声明不改（design:23） | 需确认 config 不碰 |
| `packages/client/agent-rig/src/lib.rs`、`config.rs` | — | :10（任务 2） | :14（任务 5） | **同一条审批闭包被两 spec 改写，无依赖** |
| `apps/desktop/src/ipc.rs` | :9（任务 1） | :11（任务 3） | :15,:16（任务 6,7） | **approval ↔ config 无依赖** |
| `apps/desktop/src/events.rs`、`approval.rs` | — | :11（任务 3） | 明确不写（tasks:36-37） | OK |
| `packages/ui/ui-kit/web/src/types.ts`、`index.tsx` | :11（任务 3） | :12（任务 4） | :18（任务 9） | **三方重叠** |
| `packages/ui/ui-settings/web/src/**` | :12-:14（任务 4,5,6） | — | :18（任务 9） | config 仅依赖 settings-ia 3/5，未覆盖 4/6 |
| `apps/desktop/web/src/state/controller.ts` | :15（任务 7） | :15（任务 7） | — | approval ↔ settings-ia 无依赖 |
| `apps/desktop/web/src/store.ts` | :15（任务 7） | — | :19（任务 10） | settings-ia ↔ config 无顺序 |
| `apps/desktop/web/src/protocol.ts` | :15（任务 7） | :15（任务 7） | — | approval ↔ settings-ia |
| `apps/desktop/web/src/i18n.tsx` | :15（任务 7） | :16（任务 8） | :19（任务 10） | **三方重叠** |
| `apps/desktop/web/src/App.tsx` | :14（任务 6） | — | :19（任务 10） | settings-ia ↔ config 无顺序 |
| `apps/desktop/web/src/state/reducer.ts` | — | :15（任务 7） | — | OK |
| `editions/personal.toml` / `enterprise.toml` / `personal-only.toml` | 设计提到但 tasks 未列 | — | :17（任务 8） | 需确认 settings-ia 不改 editions |
| `e2e/*.spec.ts`、`e2e/serve-m1.sh` | :16（任务 8） | :17（任务 9） | :21（任务 12） | 文件大多不同，但 `pnpm e2e` 运行器/配置需统一（P1-14） |
| `shared/contracts/ui-contributions.json`（生成物）、`apps/desktop/web/src/generated/registry.ts` | :10（任务 2 生成） | — | :20（任务 11 生成） | 同一生成物两份变更，须串行生成 |
| `docs/16`、`docs/19`、`docs/28`、`research/client-dsh-parity-2026-11-01.md` | :17（任务 9） | :18（任务 10） | :22、:23（任务 13,14） | **同一文档三 spec 回写，无合并规则** |

**处理要求（P1-1 修法）**：在每份 tasks.md 增加「跨 spec 合并顺序」小节，二选一：
① 串行三阶段：settings-ia（含契约/生成器/ui-kit/ui-settings）→ approval（agent-rig/ui-chat/desktop）→ config（config/connector/ipc/agent-rig/desktop/UI）；或
② 保留并行但指定文件 owner（同一文件只允许一个 spec 实现，另一 spec 以任务后置方式排队），并在共享任务板建 integration task；对 `ipc.rs`、`agent-rig/src/lib.rs`、`ui-kit/web/src/types.ts`、`i18n.tsx`、`docs/19` 等重叠文件必须先有一份合并后的设计（尤其 approval R3 与 config R2 的审批链，见 P0-2）。

## 4. 变更预算与拆分（含 config 28 文件判断）

- 模板硬口径：`specs/templates/proposal.md:8`「变更预算：<预计文件数/行数；**超过 10 文件或 400 行需拆分**>」；§8 要求「是否需要拆成多个 spec：是/否；若是，列出子 spec」。三份 spec 均 >10 文件（settings-ia ~15、approval ~23、config 自述 ~28），只有 config 达到 3–4 倍阈值且含三个可独立交付的目标（审批策略、MCP 配置、`connectors.*` 接线）。
- config 自述预算与实际清单不一致：proposal:144-146 分项相加 = 4+3+3+8+4+3+2+7 = **34 文件**（不是 28），且 1200–1600 行是 400 行阈值的 3–4 倍。
- **判断（P1-3）**：config 应按其自身 proposal:152-153 的备选拆分，**至少把 MCP 部分拆成独立 spec `2026-11-01-client-mcp-config`**（含 G3/G4、`mcp.test`、`ui-settings-mcp`、`AppSettings.mcp`/`McpServerConfig::validate` 提升），本 spec 保留审批策略 + `connectors.*` 接线（约 14 文件）；若用户要保留「三 spec = Phase 1」的已批结构，则必须在 proposal §8 记录**用户批准的显式例外**、修正文件数/行数、并把 3 个阶段写成可独立合并的顺序（现 tasks 已分阶段，但依赖与验收未按阶段切分）。
- settings-ia/approval 超 10 文件但目标单一（IA / 聊天渲染正确性），建议保留但补「预算例外」记录；approval 内的 i18n 门禁修复是否属同一目标请作者在 proposal §8 说明（可 P2 处理）。

## 5. 可验收性检查（tasks.verify / 成功指标 / e2e 路径）

| 问题 | 证据 | 判定 |
| --- | --- | --- |
| approval e2e 用错运行器：`pnpm e2e --grep "approval\|tool"` 走 `scripts/m1-e2e.sh`（compose `--profile mock`，`agent.loop=agent-loop`，见 `scripts/m1-e2e.sh:8`；`profiles/mock.toml`） | 现有审批 e2e 在 `e2e/agent-rig.spec.ts:34,46`（由 `scripts/agent-e2e.sh` + `playwright.agent.config.ts` 跑）；`playwright.config.ts:7` 明确 `testIgnore: ["**/agent-rig.spec.ts"]` | 命令不可验收；应改为扩展 `scripts/agent-e2e.sh` / `e2e/agent-rig.spec.ts`，并明确真实 personal 组合（P1-14） |
| config e2e `approval-policy` 同样依赖 agent-rig，不能用 mock profile 验证；`mcp-config`/连接器用例需真实 personal serve 变体 | `scripts/m1-e2e.sh:8-13`（先 mock 再 restore + `m1-serve`/`m1-a11y` 显式清单，新 spec 文件不在第二段清单内） | 命令需写明运行器与 profile（P1-14） |
| settings-ia/approval 改了契约却没把生成物门禁写进 tasks/acceptance | settings-ia tasks/acceptance 全文无 `schema-check`/`export_schema`（grep exit=1）；approval 同 | P1-5 |
| config 任务 14 verify =「`plan §4 清单全绿`」不是可执行命令 | config tasks:23 | 不可验收（P2-5） |
| approval 成功指标「§4.1 100%」与 R1/R2 实际字段不符 | F24 | 不可验收（P1-12） |
| config 成功指标「任何策略下不存在自动批准 mutating 工具的路径」当前即被打破 | F21 | 不可验收（P0-1） |
| 设置 e2e 证据目录未定义（acceptance 表格只写「—」/`target/evidence/...`） | settings acceptance:20、approval acceptance:18、config acceptance:27 | 需指定预期证据路径（P2-9） |
| 其他 tasks.verify 可执行性 | settings/approval/config 其余 `cargo test -p ...`、`zk ...`、`pnpm ...`、`bash scripts/i18n-check.sh` 均为真实命令 | OK |

## 6. P0（阻塞：矛盾 / 不可实现 / 安全）

### P0-1 · config：审批「安全不变式」与其真值表/现状矛盾（mutating 工具会被自动执行）
- 位置：`spec.md:18`（不变式）、`spec.md:21-29`（R2）、`design.md:85-95`（真值表与「诚实说明」）、`tasks.md:10-14`。
- 问题：spec 规定「任何取值都不得自动批准 `mutating == true` 的工具」，但 R1 对 `ask` 的定义是「仅按 requires_approval 询问」；实际代码有 **4 个 mutating 且 `requires_approval: false`** 的工具（`schedule` tool-schedule/src/lib.rs:79-80、`job_kill` tool-job-kill/src/lib.rs:15-16、`goal` tool-goal/src/lib.rs:43-44、`mcp_call` connector-mcp/src/lib.rs:873-874）。按 design 真值表，`ask`/`auto_read_only` 对 (requires_approval=false, mutating=true) 返回 false ⇒ 自动执行 ⇒ 不变式不成立；`auto_read_only` 的「只读不询问，mutating 仍按 requires_approval 询问」同样放行这 4 个工具。
- 修法（需用户拍板，见 T1）：三选一并写入 spec/design 与负向测试（6 组合表要扩成含 (false,true) 的 4×3 表）：
  ① 把不变式改成可实现的弱不变式「任何策略都不得自动批准 `requires_approval == true` 的工具」，并明确列出「mutating 但声明免审批」的 4 个工具与理由（不改变默认行为）；
  ② 把 `ask` 定义为「requires_approval OR mutating 都询问」并同步改这 4 个工具的声明（会改变默认行为，需发布说明与用户确认）；
  ③ 新增 `ToolSpec` 语义（如 `mutating && !requires_approval` 视为配置错误）并在 `zk validate`/conformance 中拒绝。
- 状态：**OPEN**。

### P0-2 · approval ↔ config：审批事件链与「始终允许（本会话）」状态机互相矛盾且双实现
- 位置：approval `spec.md:25-29`（R3）、`design.md:50-52`（agent-rig `Mutex<HashSet>` + broker `HashSet` 两份 always-allow）；config `spec.md:21-29`（R2「不产生孤立 ApprovalResolved」「requested→decided 成对或缺省」）、`design.md:78-83`（策略短路不发事件）。
- 问题：
  1. approval R3 说「后续调用不再弹审批，**事件链仍发出 `approval_resolved{allow_session}`**」；若没有 `ApprovalRequested` 就是 config R2 明确禁止的**孤立 ApprovalResolved**；若仍发 Requested 则「不再弹审批」不成立。两份 spec 对同一事件链给了互斥约束。
  2. always-allow 记忆出现两份：agent-rig runtime 的 `Mutex<HashSet<(conversation,tool)>>` 与宿主 broker 的 `HashSet<(conversation,tool)>`；两处清理时机/命中语义不同，谁先求值未定义（policy=ask + always-allow 命中 vs 策略短路）。
- 修法：指定单一 owner（建议：策略求值 `should_ask` 在 agent-rig，always-allow 也在 agent-rig 同一处求值并**只发一对** `ApprovalRequested{auto:true}→ApprovalResolved{allow_session}` 或**都不发**并新增 `approval_skipped` 事件；宿主 broker 只做 IPC 阻塞，不再持有会话允许集合）；在 config R2 与 approval R3 引用同一事件序列表，并加跨 spec 集成测试（strict/ask × always-allow 命中/未命中的事件序列 golden）。
- 状态：**OPEN**。

### P0-3 · approval：工具行重放设计（归并方向/五态）与实际持久化顺序、事件发射路径矛盾
- 位置：`proposal.md:32`（「归并到前一条 assistant 消息」）、`spec.md:31-39`（R4）、`design.md:54-56`（`toolRows.ts`/`mergeToolRows`）；tasks.md:10,15。
- 问题（两点，均已用代码核验）：
  1. **方向错误**：工具行在 `ToolCallFinished` 时立即持久化（`apps/desktop/src/events.rs:64-78`），assistant 消息在整轮结束才持久化且 `created_at_ms = now`（`apps/desktop/src/ipc.rs:1776-1783`、`1842-1856`），会话按 `created_at_ms ASC, id ASC` 读取（`packages/client/store-session-sqlite/src/lib.rs:651`）⇒ 实际顺序是 user → tool… → assistant。按「前一条 assistant」归并会把本工具卡挂到**上一轮** assistant 上（跨轮错挂），或直接找不到。
  2. **状态不可达**：拒绝路径（`agent-rig/src/lib.rs:345-358`）和取消路径（`:357-358`）在发 `ToolCallStarted/Finished` 之前就 return；BridgeSink 只持久化 `ToolCallFinished`（events.rs:64）。因此「被拒绝/被取消」的工具调用根本不会落库，R4 的 `cancelled` 态与「拒绝后刷新仍显示工具卡」不可实现。
- 修法：冻结 turn 归属键——给 `Message` 增加 `turn_id`（`#[serde(default)]`）或在 turn 开始持久化 assistant 占位行；`toolRows.parse` 归并到**同一 turn 的 assistant**（而不是「前一条」）；为 denied/cancelled 增加终态事件（如 `ToolCallFinished{status}` 在拒绝/取消分支也发，或新增 `ToolCallDenied/Cancelled`），并写「真实 store 写入顺序」的 golden（中文/emoji 分片）+ e2e 刷新一致性断言。
- 状态：**OPEN**。

### P0-4 · settings-ia：revision 契约/存储/任务不闭环，按现文无法实现
- 位置：`spec.md:28-34`（R4）、`spec.md:59-64`（数据与兼容）、`design.md:32-33`（「持久化到设置文件旁的 revision 字段**或**存储层元数据」）、`design.md:44`；`tasks.md:9`（任务 1 只列 contract ipc + desktop）。
- 问题：
  1. 契约中**不存在 `SettingsGetResult`**（`grep SettingsGetResult packages/contracts/ipc/src/lib.rs` exit=1）；当前 `settings.get` 直接返回 `masked_settings()`（`apps/desktop/src/ipc.rs:527-530`），`settings.set` 也返回 `masked_settings()`（`:588-590`）——没有 revision 的承载位置/返回位置。
  2. revision 存哪里未定（设置 JSON 内字段 vs 存储元数据 vs 内存），直接决定 `SettingsStore`（`packages/contracts/config/src/lib.rs:653-658`）与 `packages/client/store-settings-json` 是否要改；而 tasks 写作用域**没有任何 store-settings-json 任务**，任务 1 只覆盖 `contracts/ipc` + `apps/desktop`。
  3. R3「每个 pane 单独保存」与 design §2.4「底部保存/取消统一由外壳管理」矛盾；若按 R3 每次保存都带旧 `expected_revision`，R4 的冲突拒绝会把「只改了本 pane」也拒掉；若先重读最新文档再合并，则绕过 R4。
- 修法：冻结一套设置保存协议并写进 spec/design：
  ① 新增 `SettingsGetResult { settings: AppSettings, revision: u64 }`、`SettingsSetResult { settings: AppSettings, revision: u64 }`（或明确 settings.set 后重新 get）；
  ② revision 存储二选一并明确实现：推荐「`store-settings-json` 旁挂元数据（不改 settings.json schema，避免旧客户端回写把 revision 归零）」，并把 `packages/client/store-settings-json/**` 加入任务与写作用域；
  ③ 保存语义二选一并删除矛盾文字：整窗口单一保存（recommended，R3 改为「外壳合并后统一保存」）或逐 pane 保存+冲突时允许「仅合并本 pane 字段」；
  ④ 补 `SettingsGetResult/SettingsSetResult` 的 schema/TS 生成与 conformance 测试（见 P1-5）。
- 状态：**OPEN**。

## 7. P1（应修）

### P1-1 · 三份 spec 写作用域重叠 + approval 与其它两份无依赖声明
- 位置/证据：见 §3 表格（`tasks.md` 行号）；`approval/tasks.md:21-25` 的「明确不写」未覆盖 `agent-rig/src/lib.rs`/`ipc.rs` 与 settings-ia/config 的交叉。
- 问题：按现 tasks 并行实现会在同一审批闭包、同一 IPC dispatch、同一 ui-kit 类型/i18n 文件上竞写；尤其 approval R3 与 config R2 的审批链（P0-2）。
- 修法：每份 tasks 增加「跨 spec 顺序/owner」小节（推荐串行 settings-ia → approval → config），并在 proposal「变更预算与拆分」记录；跨 spec 的审批链设计先合并成一份（或由 Lead 出一页 integration 设计）再各自开工。
- 状态：**OPEN**。

### P1-2 · C12/C19 文档先行缺口 + 决策状态自相矛盾
- 位置：`settings-ia/design.md:95`（「ADR/D 已记录：**待定**（pane slot 机制）」）；`config/proposal.md:106-140`（D1–D8 全勾「已定」）vs `config/spec.md:192-201`（D1–D8 全列「未决问题/需用户拍板」）；`config/acceptance.md:54`（D1–D8 结论→docs）；三份 tasks 均把 docs 回写排在最后（settings-ia tasks:17、approval tasks:18、config tasks:22）。
- 问题：多贡献 pane slot 是新的组合/UI 契约机制，审批策略封闭枚举/MCP 进 `AppSettings` 是新配置语义——按 C12/`docs/11` §9「先文档（ADR/docs）→独立评审→approved spec→代码」，这些决策应在开工前落到 `docs/16`/`docs/24`/`docs/13`（或 `docs/15` D 编号）并被引用；现文只有 spec 内描述，且 docs 任务是收尾项。
- 修法：在 tasks 最前插入「决策落文档」任务（settings-ia：pane slot 机制 + 多贡献排序写 `docs/16` §3/`docs/24`，若属通用机制补 ADR；config：D1/D2/D3/D4/D5 落 `docs/15`/`docs/13` ADR 或 `docs/38`/`docs/07`；approval：`docs/19` §4.1/§4.2 与 `docs/38` §2 事件/决策枚举）；把 `config/spec.md §6` 与 proposal 的决策状态统一；`acceptance.md` 引用本文评审结论。
- 状态：**OPEN**。

### P1-3 · config 预算超拆分阈值，需拆分或用户批准例外
- 位置：`config/proposal.md:10-11`（自述 ~28 文件/1200–1600 行「不拆」）、`:142-153`（分项与拆分备选）；`specs/templates/proposal.md:8`。分项相加实际 **34 文件**。
- 问题：模板「超过 10 文件或 400 行需拆分」；config 三个目标（审批策略 / MCP / connectors 接线）可独立交付且风险不同，bundling 使评审与回滚粒度变粗。
- 修法：按 proposal:152-153 拆出 `2026-11-01-client-mcp-config`（MCP 部分）；本 spec 保留审批策略 + `connectors.*` 接线；若坚持不拆，需在 proposal §8 写明用户批准例外、修正文件/行数、并把验收按阶段 A/B/C 拆成可独立合并的证据表。
- 状态：**OPEN**。

### P1-4 · C17 能力登记与 spec 声明不符（pane slot / `contract.approval`）
- 位置：`settings-ia/design.md:28`、`proposal.md:91`（声称登记到 `ui-kit/web/src/types.ts` + `docs/generated/CAPABILITY-INDEX.md`）；`approval/proposal.md:93`、`design.md:30`（新增 `@capability contract.approval`）。
- 问题：`zk capability` 只扫描 `packages/<pkg>/src`（Rust）与 `crates/zk-kit-*`，`walk_rs` 只收 `.rs`（`tools/zk/src/main.rs:3070-3106,3165-3186`），`ui-kit/web/src/types.ts` 的注释不会进 INDEX；`docs/generated/CAPABILITY-INDEX.md` 现存 0 条 `ui.*`。approval 的 `contract.approval` 是全新 capability id（INDEX 无，`contract.agent`/`contract.tool` 已存在），但 tasks 未写「在哪个 Rust 源写 `@capability contract.approval`」也未说明是否复用既有 id。
- 修法：settings-ia 把 `@capability ui.settings.pane` 写到被扫描的 Rust 源（如 `packages/ui/ui-kit/src/lib.rs`，与类型/注释同处），或明确「slot 不是 capability、不登记 INDEX」并删掉该声明；approval 明确二选一：复用 `contract.agent`（推荐，不改 INDEX）或新增 `contract.approval` 并在任务中加「加注释 + 重生成 INDEX」；两处都把 `zk capability --check` 落进 acceptance。
- 状态：**OPEN**。

### P1-5 · C10/C11：settings-ia 与 approval 改契约却缺生成物门禁
- 位置：`settings-ia/tasks.md:9-18`、`acceptance.md:9-20`；`approval/tasks.md:9-19`、`acceptance.md:11-19`（均无 `cargo test --workspace export_schema`、`bash scripts/schema-check.sh`；grep exit=1）。config 有（design:15、tasks:20、acceptance:21）。
- 问题：两份 spec 都改 `zk-contract-ipc`/`zk-contract-agent` 的 DTO/事件字段；ADR-059 说明 T22 后端已落地（`shared/contracts/schema/*.json` + `contracts.ts` + `SCHEMAS`），漏跑会让 `schema-check.sh`/`export_schema` 红或生成物漂移（C10/C11）。
- 修法：两份 tasks 各加一条「生成物重生成 + `cargo test --workspace export_schema && git diff --exit-code` + `bash scripts/schema-check.sh`」，并在 acceptance 表加对应行；settings-ia 另需在任务 1 说明新增 `SettingsGetResult/SettingsSetResult` 的 schema/TS 导出。
- 状态：**OPEN**。

### P1-6 · settings-ia：pane 排序/合并规则与内建 pane 的关系未定义
- 位置：`spec.md:11`（pane 列表 = contributions + 内建外壳项）、`design.md:39-43`（内建 `panes/general.tsx` 等文件 + provider 通过贡献挂入）、`spec.md:16-18`（「按 order 升序、包 id 字典序稳定排序」）。
- 问题：内建 pane 与贡献 pane 分属两套列表，`order` 只对 contributions 定义；provider（order=10，`ui-settings-provider/zk.toml`）与内建「模型与 Provider」pane、config 的 MCP（order=70）与内建「企业/关于」pane 的相对顺序没有规则；空贡献（enterprise 无 personal-only 贡献）时生成器输出/外壳空态也未定义（现生成器会为缺失 slot 造 stub，`tools/zk/src/main.rs:2875`）。
- 修法：推荐「所有 pane（含 ui-settings 内建）统一由 `ui.settings.pane` contributions 表达，ui-settings 在自身 `zk.toml` 声明内建 pane 并给 order」；在 spec 固化 order 表（general/agent/tools/enterprise/about + provider/MCP），定义空列表时 `SettingsPanes = []` 与外壳空态；补 generator 快照测试（personal/enterprise 两套 registry）。
- 状态：**OPEN**。

### P1-7 · config：`ApprovalPolicy` 未知值「降级 ask、不阻止启动」与普通 serde 枚举不可兼得
- 位置：`config/spec.md:19`（未知取值按 ask、不阻止启动）、`design.md:166-173`（`#[derive(Deserialize)] #[serde(rename_all="snake_case")] enum ApprovalPolicy`）、`tasks.md:10`。
- 问题：普通字符串枚举遇到未知值反序列化失败 ⇒ `settings.load()` 失败 ⇒ boot 路径 `apply_persisted_settings`（`apps/desktop/src/ipc.rs:302-343`）报错，与 R1 冲突；`docs/18:153` 也要求「未知枚举降级 Unknown」。
- 修法：在 design 固化实现（自定义 `Deserialize`/`Value` 先行解析，未知→`None`/`Ask`，保留原文字段可选）并加 4 形态 + 未知值测试；或把 R1 改成「未知值导致设置加载失败并进入可恢复错误态」但须说明不阻塞启动的降级路径。
- 状态：**OPEN**。

### P1-8 · config：`McpServerConfig::validate` 提升后，新上限/字符集将一并作用于企业 catalog（潜在回归）
- 位置：`config/design.md:215-233`（上限表 + 提升 validate）、`spec.md:53-58`（R5）；`packages/client/connector-mcp/src/lib.rs:102-148`（现 validate）、`:200`（catalog 也调用）、`:307-313`（`valid_server_id` 目前不在 validate 内）。
- 问题：`servers_from_catalog` 走同一 `validate_server`；新增 id 字符集（`[A-Za-z0-9._-]`）、args/env 条数、长度上限会改变 catalog 校验结果——已存在的企业 catalog 若 id/参数不合新规，升级后会整体拒绝/无法执行；spec 却称「规则不变 + 上限」。
- 修法：区分「catalog 既有规则」与「本地 settings 新上限」两级校验（如 `validate` 保持现有规则，`McpSettings::validate`/`update_local_servers` 追加本地上限与 id 字符集），或明确对 catalog 也收紧并给兼容性测试 + 迁移/发布说明 + 服务端 catalog 迁移方案。
- 状态：**OPEN**。

### P1-9 · config：`mcp.test` 复用带缓存的 `list_server_tools`，与「临时关闭/不改变生效配置」不符，且同步阻塞风险未解决
- 位置：`config/spec.md:74-81`（R7）、`:157-161`（§4.3「临时进程/连接在使用后关闭」）、`design.md:133-145`（D 段同步执行）；`packages/client/connector-mcp/src/lib.rs:261`（全局 `SESSIONS`）、`:406-413`（`with_session` 复用）、`:442-444`（`list_server_tools`）。
- 问题：`list_server_tools` 会把草稿 server 的会话/子进程放进全局缓存（fingerprint 键），可能长期驻留/与生效配置并存；R7「不改变当前生效配置」只对 settings 成立，对进程/会话缓存不成立。另 `ipc_call` 是同步 command，15s 硬超时会造成桌面 IPC 阻塞（spec 已列为已知限制，但未定缓解）。
- 修法：新增无缓存探测路径（`test_server`：spawn/connect → initialize → tools/list → drop/close/kill，并断言不写入 `SESSIONS`），或 `mcp.test` 使用独立 transport 并在结束后 `reload`/清理；把同步阻塞改为 async command（Tauri `spawn_blocking`）或先返回 job id；补「测试后会话数不变/进程退出」的断言。
- 状态：**OPEN**。

### P1-10 · config：企业「catalog 唯一来源」与 env fallback/C1/C6 未闭环（需 ADR）
- 位置：`config/spec.md:63-72`（R6 优先级：catalog > local > env「仅无设置文件」）、`config/acceptance.md:39-43`（企业无本地段）；`packages/client/connector-mcp/src/lib.rs:244-252`（空 catalog 回落 env）；`docs/04-contracts-and-api.md:35-39`（C1：服务端不控制工具/网络；客户端能力完整）；`AGENTS.md` C6。
- 问题：a) 企业 fresh install（无 settings 文件）仍会走 env `ZEOKEEL_MCP_SERVERS`，与「企业 catalog 唯一执行配置来源」字面冲突；b) D2「企业版本地 MCP 不允许」与 C1 个人版专属包机制/「客户端能力完整」的定位需要在 ADR/docs 解释（是构建期 edition 差异，不是服务端管控）。
- 修法：R6 明确 edition 维度：enterprise 下 env 也不生效（只 catalog；未登录=无 MCP 并诚实报不可用），或写明 env 仅本地开发/测试且不属「企业配置」；为 D2 补 ADR（或 `docs/15` D 编号）映射到 C1 personal-only 机制，并在 `docs/07`/`docs/04` §1.5 对齐措辞。
- 状态：**OPEN**。

### P1-11 · approval：审批决策 wire format 已与契约枚举漂移，spec 未定义迁移
- 位置：`approval/design.md:10`（「`ApprovalDecidePayload` 复用枚举，无需新命令」）、`spec.md:22`（四值映射）；`packages/contracts/ipc/src/lib.rs:252-258`（DTO 用 `ApprovalDecision`）；`apps/desktop/src/ipc.rs:728-739`（`decision == "approved"`）；`apps/desktop/web/src/state/controller.ts:667-679`（只发 `approved/rejected`）；`shared/contracts/ts/contracts.ts:281,803`（生成物已是 `allow_once/reject/cancel`）。
- 问题：契约 DTO/生成 TS 已是 `allow_once|reject|cancel`，但运行期两端用 `approved/rejected` 字符串且契约 DTO 在 dispatch 无调用者——这是 `Cancel` 不可达的真正根因。仅改 `ApprovalDecision` 枚举（加 `AllowSession`）不会自动接通；若同时改宿主为 typed DTO，旧 UI/旧 e2e/旧 wire 值会断。
- 修法：在 design 明确 wire 迁移：宿主 `approval.decide` 用 `serde_json::from_value::<ApprovalDecidePayload>`（或显式四值 parse），并保留 `approved/rejected` 旧值别名兼容（`serde(alias)`/映射层），前端/契约/生成 schema 同步；补 4 值 + 旧值兼容 + 非法值拒绝单测；把「Cancel 不可达」根因改成 wire 映射而非仅 UI 按钮。
- 状态：**OPEN**。

### P1-12 · approval：`docs/19` §4.1 100% 覆盖声明不实，缺「记住范围/网络目标/延长一次/二次确认」
- 位置：`approval/proposal.md:57`（100% 指标）、`spec.md:16-23`（R2 卡片字段与决策）；`docs/19-client-ux-flows.md:91-110`（§4.1 表格 + §4.2）。
- 问题：§4.1 还要求「读写范围含网络目标（域名/端口）」「超时可延长一次」「记住范围：仅本会话/仅该工具/仅该路径」；§4.2 要求「危险操作（删除、写入工作区外、shell 网络）二次确认」。spec 只做 per-tool allow_session、无 path scope、无网络目标、无延长、无二次确认。
- 修法：二选一并写清：① 补齐字段/交互（记住范围做成下拉、网络目标从 args 提取、延长一次由 broker 支持、二次确认）并加 e2e；② 把指标改为「覆盖 §4.1 的 X/Y 字段」+ 把未覆盖项列入 proposal §3 非目标与 acceptance 剩余风险（引用 docs/19 编号）。
- 状态：**OPEN**。

### P1-13 · approval：持久化 `args`/展示完整命令引入敏感数据面，无脱敏设计
- 位置：`approval/spec.md:14`（不得把密钥/token 放进 args/diff/日志）、`:34`（工具行补齐 `args`）、`proposal.md:32`；`packages/ui/ui-chat/web/src/ApprovalPart.tsx:44-45`（原样 `<pre>{args}</pre>`）；`apps/desktop/src/events.rs:65-67`（新增 args 后写入会话 DB）。
- 问题：审批卡按 docs/19 要求「完整命令/参数不可折叠」，而 R4 新增把 args 持久化到 SQLite；shell/pwsh 命令可包含 token/key/env，现有代码无任何参数脱敏（`ApprovalPart` 直接显示，BridgeSink 直接落库）。spec 声称「不新增泄漏面」与事实不符。
- 修法：定义脱敏策略（工具声明 `sensitive_arg_keys`/正则掩码，展示与持久化共用一份 redaction；diff/args 落库前过 redaction；日志禁止）；列出不可脱敏的已知边界（如自由文本 command 中的 secret）并落 `KNOWN-LIMITATIONS`；补「含 token 的 bash 调用 → 卡片/会话 DB 均掩码」的负向测试。
- 状态：**OPEN**。

### P1-14 · approval/config：e2e 验证命令用了错误的运行器/profile
- 位置：`approval/tasks.md:17`（`pnpm e2e --grep "approval|tool"`）、`approval/acceptance.md:18`；`config/tasks.md:21`（`pnpm e2e --grep approval-policy` 等）。
- 问题：`pnpm e2e` = `scripts/m1-e2e.sh`，先 compose `--profile mock`（`agent.loop=agent-loop`，无审批代码），且 `playwright.config.ts:7` 排除 `agent-rig.spec.ts`；现有审批 e2e 在 `e2e/agent-rig.spec.ts:34,46` 由 `scripts/agent-e2e.sh` 跑真实 personal rig。按现命令，审批相关用例要么不跑、要么在错误组合上跑。
- 修法：approval 的 e2e 任务改为「扩展 `scripts/agent-e2e.sh` + `e2e/agent-rig.spec.ts`」并写明 profile；config 的 `approval-policy` 同；`mcp-config`/连接器用例写明用 personal 真实 serve 变体（`e2e/serve-m1.sh` seeded settings）并把这些 spec 文件加入第二段显式清单；新增 `--grep` 前先验证命令真实可跑。
- 状态：**OPEN**。

### P1-15 · config：`editions/personal-only.toml` 是数据清单，不是「注释同步」
- 位置：`config/design.md:16`（排除机制）、`design.md:292`、`design.md:371`（「`editions/personal-only.toml` 注释同步」）、`tasks.md:17`（任务 8 写该文件）；`editions/personal-only.toml:3`（`packages = ["zeokeel/client-update-github", "zeokeel/ui-settings-provider"]`，由 `zk validate` 读取：`tools/zk/src/main.rs:1376-1416`）。
- 问题：新包 `ui-settings-mcp` 是 personal-only，必须加入该**清单**（否则 `zk validate` 的 personal-only leak 检查/CI 断言覆盖不到它）；design 写成「注释同步」会漏掉数据项。
- 修法：任务与 design 改为「把 `zeokeel/ui-settings-mcp` 加入 `editions/personal-only.toml:3` 的 packages 列表」，并加 `zk validate` 断言（enterprise 闭包不含该包）。
- 状态：**OPEN**。

### P1-16 · approval：审批元数据（cwd/read_paths/write_paths/diff）由 agent-rig 重新推导，重复工具层知识且越过工具抽象读文件
- 位置：`approval/design.md:43-50`（在 agent-rig 工具闭包内按 args 的 `path/paths/cwd` 提取相对化、按工具族映射 risk、`fs.write/fs.edit` 读文件生成 diff）、`tasks.md:10`。
- 问题：(1) 41 个工具的 args schema 各不相同，在 agent-rig 中心化猜测 `path/paths/cwd` 会产生第二份「参数→路径/影响面」知识（C17），且对 shell/worktree/mcp_call 无法准确；(2) agent-rig 直接读目标文件生成 diff，绕过工具/`host.fs-access` 抽象（agent-rig 目前只有 hooks 的 fs 用途），安全与错误处理（权限/大文件/编码）会与 tool-fs 分叉。
- 修法：把可选 `approval_metadata(&args, &context)` 放到 `Tool` trait/`ToolSpec` 侧（每个工具自己声明 read/write paths、risk、diff 生成），agent-rig 只做聚合与事件发送；或者把路径提取/diff 放进 `zk-kit-tools` 单一实现并让 tool-fs 与其共享；补「各工具族影响面」单测（含 workspace 外路径、无 path 工具）。
- 状态：**OPEN**。

## 8. P2（建议）

1. **P2-1 行号漂移**：`settings-ia/proposal.md:19` 引 `ipc/src/lib.rs:240-248`，实际 DTO 在 `:245-249`；`approval/proposal.md:17-18` 引 `events.rs:60-76`，实际块 `:64-78`；`approval/proposal.md:15` 引 `agent/src/lib.rs:163-172`，实际枚举 `:163-174`。修法：改为实际行号或去掉行号只给文件（OPEN）。
2. **P2-2 acceptance 章节号重复**：`settings-ia/acceptance.md:49,54` 与 `approval/acceptance.md:50,55` 都有两个 `## 6`（结论/复用证据）。修法：复用证据改 §7（OPEN）。
3. **P2-3 config design 自相矛盾**：`design.md:287` 称「`main.rs`/`serve.rs` 无需改」，同段又写「`main` 启动顺序调用 `mcp::configure_persisted_mcp`」；task 6 也写 `main.rs`。修法：统一为「`main.rs` 增加 boot 调用，`serve.rs` 不改」（OPEN）。
4. **P2-4 config R9 五态 vs AGENTS 六态**：`config/spec.md:104` 写「空态、加载、错误、校验错误、保存中五态」，`AGENTS.md` §7 要求六态（空/加载/错误/离线/流式中/已取消）。修法：按六态口径重写并加状态矩阵（OPEN）。
5. **P2-5 config 任务 14 verify 不可执行**：`config/tasks.md:23`「`plan §4` 清单全绿」不是命令。修法：改为实际门禁命令清单（OPEN）。
6. **P2-6 strict 策略的 UX 副作用未讨论**：`config/design.md:93-95` 的 `strict` 会询问只读工具（含 `ask_user_question`/`todo`/`present`/`mcp_list_tools`），可能出现「先审批再提问」的双重交互；需在 spec/非目标中决定是否给这些工具豁免（OPEN）。
7. **P2-7 `SettingsPaneProps.edition` 运行期分支**：`settings-ia/design.md:37-38` 把 `edition` 传进 pane props；C2 精神是构建期差异优先，建议 pane 差异用包/贡献表达，`edition` 仅用于文案（OPEN）。
8. **P2-8 契约依赖需登记**：`zk-contract-config → zk-contract-connector` 属新耦合（config design:11）。建议在 `docs/24` §1/`docs/04` §1 记录依赖方向与「settings schema 内含 connector DTO」的兼容责任（OPEN）。
9. **P2-9 证据路径未定义**：三份 acceptance 的 e2e/截图表为「—」或 `target/evidence/...`；建议预填 `target/evidence/<change>/`（含 axe.json/screenshots），与 `docs/11` §8 一致（OPEN）。
10. **P2-10 settings-ia 缺 C14 回归项**：`server_url` 迁入企业 pane 后，需验收「渠道锁定只读（`serverUrlLocked`）/改地址需重登 / 企业无 personal pane」；现 plan 只有 `App.tsx` 单测一项（tasks:14）。建议在 e2e/手工检查加 C14/ADR-042/044 断言（OPEN）。

## 9. 待拍板（本文不猜测，需用户/Lead 决策后回写）

- **T1（阻塞 P0-1）**：审批安全不变式三选一（弱化不变式并列出 4 个免审批 mutating 工具 / 改 `ask` 语义为 mutating 全询问并改工具声明 / 引入 `mutating && !requires_approval` 非法）。这会改变默认行为或安全口径，不能由评审代替决策。
- **T2（阻塞 P1-3）**：config 是否拆分出 `2026-11-01-client-mcp-config`。用户已批准「Phase 1 = 三 spec」的结构，拆分需用户确认；若不拆，需批准「预算例外」。
- **T3（阻塞 P0-4）**：revision 存储位置（`settings.json` 内字段 vs `store-settings-json` 旁挂元数据 vs 仅内存）与 `settings.set` 是否返回新 revision。
- **T4（P1-10）**：企业版本地 MCP 的 D2 决策与 C6/C1 的映射（构建期 edition 差异 vs 弱管控措辞）；需 ADR 或 `docs/15` D 编号。
- **T5（P1-12）**：审批卡是否补齐 docs/19 §4.1 的「记住范围（路径）/网络目标/延长一次」与 §4.2 二次确认；或把这些明确列为后续 Phase。
- **T6（P1-2）**：多贡献 `ui.settings.pane` 机制是否需要 ADR（还是仅 `docs/16`+`docs/24` 足够）；决策所有权在 Lead 还是需要用户拍板。

## 10. 复评安排

- 复评日期：**待定**（作者修复 P0/P1 后请求复评；本记录不预写复评结论）。
- 复评结论：**待复评**（同一评审者 `spec-reviewer` 执行；复评通过后在此处追加新的日期/结论，并把 `proposal.md` 的「评审」字段指向本文件）。
- 复评准入：① 本文全部 P0 关闭；② P1 关闭或有用户/Lead 批准的书面豁免（豁免须写进 spec proposal）；③ 对象文件 SHA256 与 §0 对照——若作者修改导致 hash 变化，需重新 diff 并确认只改了评审要求的部分；④ 复评后跑 `python3 scripts/spec-check.py`/`docs-check.py`/`review-check.py` 并记录 exit code。
- 作者需回写：三份 `proposal.md` 的「评审：」链接改为 `docs/reviews/2026-11-01-client-ui-parity-specs.md`；三份 `acceptance.md` 引用本记录；修复 P0/P1 后在 tasks 勾选并附证据。

## 11. 本次评审的门禁与只读证据

- 已执行（见 F25/F11/F16）：`spec-check` exit 0；`docs-check` exit 0（41 files）；`review-check` 评审前 exit 0（28 records）、写本记录后 exit 0（29 records）；`zk validate` exit 0；`zk compose --edition personal --profile default --channel official --check` exit 0；`zk capability --check` exit 0（45 entries）；`bash scripts/schema-check.sh` exit 0；`bash scripts/i18n-check.sh` **exit 1**；`python3 scripts/docs-command-check.py --self-test` exit 0。
- 只读声明：本会话未修改任何代码/spec/生成物；唯一新增文件 = 本评审记录。评审结束前 `git status --short` 仅显示三份 spec 的既有未提交修改 + 本文件（未跟踪）。
- 未验证（不冒充）：真实 Tauri/WebView 交互、`pnpm e2e`、`cargo test --workspace`、企业组合构建、DSH 侧实机点测（这些属实现/验收阶段证据，按 `docs/11` 要求由实现者补齐）。

## 12. 复评（2026-11-01）

- 复评者：`spec-reviewer`（同一评审员；独立性同 §头：未参与四份 spec 写作/实现，唯一写入仍为本记录）。
- 复评日期：2026-11-01（首评当日第二轮）。
- 复评对象：`HEAD 6a6d24a` + **20 个 spec 文件 SHA256**（15 个首评文件全部变更 + 新增 `2026-11-01-client-mcp-config` 5 文件）；对象 hash 见 §12.1；复评期间 hash 稳定（起止两次 `sha256sum` 排序后 `diff` 为空）。
- 关联 spec（四个 change id）：`2026-11-01-client-settings-ia`、`2026-11-01-client-approval-tool-rendering`、`2026-11-01-client-config-approval-mcp`、**`2026-11-01-client-mcp-config`**（新拆）。
- 复评输入：四份 spec 20 文件全文、首评 §6/§7 P0/P1/P2 清单、Lead 决策（T1–T6 / C-1–C-6 / M-1–M-6 / 预算例外）、当前代码（只读）。**未实现任何代码**（`git status` 无代码/生成物改动，符合 C16）。

### 12.1 复评对象 SHA256（20 文件）与差异范围

```text
6527320597d08978c45a91b9af683ef052fd9a8d76677212ad801bdfd5e31b7f  specs/changes/2026-11-01-client-approval-tool-rendering/acceptance.md
9acc996a8d9f2f7a888b3b24c2a9a8eedf1697bd5332e352b3cb3003e3a3cede  specs/changes/2026-11-01-client-approval-tool-rendering/design.md
4c56d95ae734d719515e38f81f95e377c5326066a985913f9444ca2025561914  specs/changes/2026-11-01-client-approval-tool-rendering/proposal.md
afd22b6e57ea6022ad7d648b7cc4a1c798122d1dec864452b61457cc5cead55b  specs/changes/2026-11-01-client-approval-tool-rendering/spec.md
55a4013ae2a2404d2401048f026eed113a41d61ff029d647c52f277772507b88  specs/changes/2026-11-01-client-approval-tool-rendering/tasks.md
9df315f68013835488438565e091e57d2536e218b3098592ddc69c1595ab0970  specs/changes/2026-11-01-client-config-approval-mcp/acceptance.md
f74f88854e829ce7f13c4e3fc1fa06db18ef6edb49138908fcb19e0b2fd06950  specs/changes/2026-11-01-client-config-approval-mcp/design.md
113519f7bb69fcccbf560fb3dda0492282665b098ba3027581655989d4439d7a  specs/changes/2026-11-01-client-config-approval-mcp/proposal.md
08a793c39727b88ebcb6175afb4e66cd8db6f810cd48b7526371e711d20c994c  specs/changes/2026-11-01-client-config-approval-mcp/spec.md
04e635a49d49bb4d2d3762b4cc5e14d0e9f7d4e74a2c8019803d1852243fa590  specs/changes/2026-11-01-client-config-approval-mcp/tasks.md
0f8f70dcf844ee2bdd6bfddb368a544a81cff24781a48e18aece9d63f3bcf5d5  specs/changes/2026-11-01-client-mcp-config/acceptance.md
e2b9dead4e012e2dbbcc08fcbe01a575b3526a96b73b96f23873a64506037699  specs/changes/2026-11-01-client-mcp-config/design.md
140607fbb1e019491d0108e89e0e1b412c18407b586a058c279efbcb9543440e  specs/changes/2026-11-01-client-mcp-config/proposal.md
ab97886507df33a6c805557241b06974e0300b630eb544df1ad5f8f0a7dec195  specs/changes/2026-11-01-client-mcp-config/spec.md
34796e534f3f895c0c7d986b79c10d84143b34587a660ebead1fb13da78e5da0  specs/changes/2026-11-01-client-mcp-config/tasks.md
1ac505f5e00819560480d54d0869a61ab4d5c0236b6e82e09e39805c6dd0498e  specs/changes/2026-11-01-client-settings-ia/acceptance.md
06995b90bd900998ffd255093c27c96be43f3a0dc1ebc8aecb93c9aa41f6c3da  specs/changes/2026-11-01-client-settings-ia/design.md
33b11daa3df2da698f261d5ea8701011f0f1d995604741615ff74133d961f882  specs/changes/2026-11-01-client-settings-ia/proposal.md
06f5e91109215e1583d4e6f78bb9a910160d7a548ccdb559cee7102c2ad2ddb6  specs/changes/2026-11-01-client-settings-ia/spec.md
8a7df2206df3fdd6de0541ed16c915539df8a1a23d871fb8a42c2ee411d860b9  specs/changes/2026-11-01-client-settings-ia/tasks.md
```

差异范围抽查（"只改要求项"）：
- 与首评对象（§0 的 15 个 hash）对照：15/15 均变更（作者重写），新增 5 文件为拆分 spec。**无法用 `git diff` 直接对比首评快照**（首评对象当时也未提交），因此以「逐条内容对照 + 当前工作树范围」核验：
  - `git diff --stat` 仅 15 个 spec 文件（+1378/−463）；`git status --porcelain` 排除 spec 目录与本评审记录后为空 ⇒ **无代码/生成物/契约改动**（C16：未开工）。
  - 四份 spec 均新增 `评审条目处置/关闭对照` 表（settings acceptance:8、approval proposal:141-158、approval acceptance:93-109、config/ mcp proposal §7）；抽查未受影响章节保持原范围（settings R5 a11y、approval R6 i18n、config R6 `connectors.*` 形状、approval 非目标中的通知/keyed view 等），未发现与评审条目无关的行为范围扩张。
- 结论：改动内容与首评 §6/§7 条目 + Lead 决策（T1–T6/C-1–C-6/M-1–M-6/预算例外）一一对应；新增内容为要求项的实现级展开（事件表/turn_id/两级校验/来源策略等）。

### 12.2 首评 P0/P1/P2 逐条关闭状态

| 首评条目 | 状态 | 关闭依据（复评实测） |
| --- | --- | --- |
| P0-1 config 安全不变式矛盾 | **RESOLVED** | `config/spec.md:25-28` 弱不变式「requires_approval==true 必询问」；`config/design.md:87-97` 真值表 + 4 个免审批 mutating 工具清单（`spec.md:27-35`）；C-1=A 写入 `proposal.md:112-113`；负向测试 `spec.md:166-170` |
| P0-2 approval↔config 事件链/双实现 | **RESOLVED** | `approval/design.md:40-58`《审批事件序列表》S1–S5（唯一真源）+ 求值顺序；`approval/spec.md:30-38` 单 owner；`config/spec.md:45-49` 明确引用同一张表、不复制；broker 不再持有允许集合（`approval/design.md:22,51-53`） |
| P0-3 工具行重放方向/状态 | **RESOLVED（规格层；实现补丁见新 P1-R3）** | `approval/spec.md:40-53` 按 `Message.turn_id` 归并（非"前一条"）、legacy 挂后一条 assistant、denied/cancelled 终态；`design.md:151-169` SQLite v5/写入路径/golden；任务 `tasks.md:17,22,23,25` |
| P0-4 settings-ia revision 不闭环 | **RESOLVED** | `settings-ia/spec.md:69-73` 新增 `SettingsGetResult/SettingsSetResult` + `expected_revision`；`spec.md:41-42` 旁挂 `settings.meta.json`；`design.md:34-40` 存储/增量；`tasks.md:21-23` 增加 `store-settings-json` 任务与写作用域；单一底部保存 `spec.md:29` |
| P1-1 写作用域/顺序冲突 | **RESOLVED** | settings-ia `tasks.md:5-14` 全局顺序+文件 owner；approval `tasks.md:64-72`、config `tasks.md:6-40`、mcp `tasks.md:6-34` 同一顺序（settings-ia→approval→config→mcp）与 owner 表 |
| P1-2 C12 文档先行/决策矛盾 | **RESOLVED** | 四份均有 docs-first 前置任务（settings tasks:20、approval tasks:10、config tasks:46、mcp tasks:40）；config `spec.md:189-198` 与 proposal §7 同一份（无矛盾）；settings design:15「不新增 ADR（T6）」 |
| P1-3 config 预算/拆分 | **RESOLVED（Lead 批准例外）** | 已拆出新 spec（`client-mcp-config`，5 文件）；父 spec `proposal.md:124-134` 记录拆分后 ~16 文件 + Lead 2026-11-01 例外；mcp `proposal.md:129-139` ~27 文件 + 例外 |
| P1-4 capability 登记错误 | **RESOLVED** | settings `design.md:29`/acceptance:64 明确 slot 非 capability、不登记；approval `spec.md:76-77` 复用 `contract.agent`/`contract.tool`；`zk capability --check` 45 entries 不变（复跑 exit 0） |
| P1-5 生成物门禁缺失 | **RESOLVED** | settings `spec.md:81,84`、tasks:30、acceptance:21-22；approval `tasks.md:18,30`、acceptance:25-27；config/mcp 亦有；复跑 `schema-check` exit 0 |
| P1-6 pane 排序/内建 pane | **RESOLVED（残留 P2-N1）** | `settings-ia/spec.md:11-22` 全部 pane 走贡献 + 固化 order 表 + 空列表 `[]` + 两套快照；design:41-46；残留 design 与 order 表细节不一致见 P2-N1 |
| P1-7 未知枚举降级 | **RESOLVED** | config `spec.md:77-82` R5；`design.md:63-71` 自定义 Deserialize（未知→Ask）；`spec.md:173` 4 形态+未知值测试 |
| P1-8 catalog 两级校验 | **RESOLVED（移入 mcp spec）** | mcp `spec.md:30-39` R2 两级校验（现有规则原样提升 + 本地上限仅 `McpSettings::validate`）+ catalog 回归测试；`design.md:78-79` 关键实现约束 |
| P1-9 mcp.test 缓存/阻塞 | **RESOLVED（移入 mcp spec）** | mcp `spec.md:59-71` R5 两段式 `mcp.test`/`mcp.test.status` + 无缓存 transport + 硬超时 + `SESSIONS` 数量断言；`design.md:114-148` |
| P1-10 企业 env/catalog 口径 | **RESOLVED（移入 mcp spec，T4）** | mcp `spec.md:41-51` R3 enterprise catalog-only、env 不生效、空列表权威；`proposal.md:109-110` T4 记录 |
| P1-11 审批 wire 漂移 | **RESOLVED** | approval `spec.md:75-83` R8 typed 4 值 + `approved/rejected` alias + 非法值错误；`design.md:125-134` 宿主 typed 解析/前端同步/单测 |
| P1-12 §4.1 100% 不实 | **RESOLVED（T5 选项②）** | approval `proposal.md:69-70` 7/9 完整 + 1/9 部分 + 非目标；`proposal.md:43-46` 与 `acceptance.md:64-79` 剩余风险；`spec.md:28` 明确非目标 |
| P1-13 敏感参数脱敏 | **RESOLVED** | approval `spec.md:67-73` R7 单一 redaction + `sensitive_arg_keys` + 负向测试 + `KNOWN-LIMITATIONS` 边界；`design.md:171-179` |
| P1-14 e2e 运行器错误 | **RESOLVED（new mcp 有 P2-N4）** | approval `tasks.md:28`/`acceptance.md:31-32` 改 `scripts/agent-e2e.sh`+`e2e/agent-rig.spec.ts`；config `tasks.md:55` 同一真实运行器；mcp 新 spec 的 e2e 门控问题见 P2-N4 |
| P1-15 personal-only 清单 | **RESOLVED（移入 mcp spec）** | mcp `spec.md:91-98` R7 + `design.md:184-198` 明确 `editions/personal-only.toml:3` 是 **packages 数据列表**；tasks:48 验收 |
| P1-16 元数据由 agent-rig 猜/读文件 | **RESOLVED** | approval `spec.md:8-18` R1 工具侧 `Tool::approval_metadata`；`design.md:136-149` 工具侧声明/关键工具表；agent-rig 只聚合 |
| P2-1 行号漂移 | **RESOLVED（抽查 2 处仍有 2 行级漂移）** | 主要引用已按实测（events.rs:60-79、ipc.rs:1776-1783/1842-1857、store:651 等）；`approval/proposal.md:15` 引 `agent/src/lib.rs:165-172`，实际枚举 165-174（差 2 行，见 P2-N10） |
| P2-2 acceptance 双 `## 6` | **RESOLVED** | settings acceptance §6/§7、approval acceptance §6/§7（结构检查通过） |
| P2-3 config design 自相矛盾 | **RESOLVED** | config design 已按 `apps/desktop/src/{mcp.rs,ipc.rs,main.rs}` 统一（design:213-214、tasks:45） |
| P2-4 五态 vs 六态 | **RESOLVED** | config `spec.md:119-129` 六态表；mcp `spec.md:78-88` 六态表；approval/ settings 亦有六态/键盘验收 |
| P2-5 verify 不可执行 | **RESOLVED** | config tasks:56/57 改为实际命令；四份任务表均给命令 + 证据路径 |
| P2-6 strict UX | **RESOLVED** | config `proposal.md:118-119` C-5 + `spec.md:37-38`；acceptance:52 已知 UX 验收 |
| P2-7 `edition` 运行期分支 | **RESOLVED** | settings `design.md:47-48` 无 edition props；mcp design:178 同 |
| P2-8 契约依赖登记 | **RESOLVED** | mcp `spec.md:100-108` R8 / `tasks.md:40`：docs/24 §1 + docs/04 §1.5 登记 config→connector 依赖（docs-first） |
| P2-9 证据路径未定义 | **RESOLVED** | 四份 acceptance 均预填 `target/evidence/2026-11-01-client-*/`（settings:7, approval:7, config:9, mcp:9） |
| P2-10 C14 回归项 | **RESOLVED** | settings `spec.md:83` + `tasks.md:28` + `acceptance.md:31` 含渠道锁定只读/改地址需重登 |

> 另：首评 §9 的待拍板 T1–T6 均已在 spec 内落定（T1 `config/proposal.md:104-106`；T2 `config/proposal.md:107`+ mcp proposal:108；T3 `settings-ia/spec.md:41-42`；T4 `mcp/proposal.md:109-110`；T5 `approval/proposal.md:99`；T6 `settings-ia/design.md:15`/`config/proposal.md:120`/`mcp/proposal.md:124`）。C-1–C-6（config/spec.md:189-198）与 M-1–M-6（mcp/spec.md:168-177）已记录；预算例外（settings proposal:84-85、approval proposal:127、config proposal:126-128、mcp proposal:126-127）已记录为 Lead 批准（2026-11-01）。

### 12.3 复评新发现 · P1（OPEN，实现前必须修）

#### P1-R1 · settings-ia：edition 专属 pane 与"全部内建 pane 由共享 `ui-settings` 贡献"冲突
- 位置：`settings-ia/spec.md:11-13`（全部 pane 含「企业」由 `ui-settings` 声明；personal/enterprise 集合由 editions 选包决定）、`spec.md:20`（order 表含「嵌入与媒体 60」「企业 80」）、`design.md:44-48`（`ui-settings` 声明 general/models/agent/tools/search/embedding/media/about；pane props 无 edition 分支）；`tasks.md:26-28`（tasks 6/7/8 把 embedding/media/企业 pane 放进共享 `ui-settings`）。
- 问题：`ui-settings` 的 `[role].editions = ["personal","enterprise"]`（`packages/ui/ui-settings/zk.toml:9`），贡献声明的字段仅 `slot/component/order`（`tools/zk/src/main.rs:726-731` UiContributionDecl 无 edition），因此 **personal 与 enterprise 会得到完全相同的 pane 集合**；但 R3 要求 `embedding/media` 只在 personal、`server_url/登出/网关模型` 只在 enterprise，成功指标又要求 personal ≥6 / enterprise ≥7。现有代码用运行期 props 做 edition gating（`apps/desktop/web/src/App.tsx:642-644` `showEmbedding/showMedia/showServerUrl`），而本设计和 P2-7 要求"无 edition 分支、差异由贡献包是否被组合表达"——两者不能同时成立。
- 修法（三选一，推荐①）：
  ① 把 edition 专属 pane 拆成 edition 专属包（`ui-settings-provider` 已 personal；新增 `ui-settings-enterprise`（enterprise）与 personal-only 的 embedding/media pane 包——可复用 M-5 的 personal-only 包先例），`ui-settings` 只保留两版共享 pane；相应更新 `editions/*.toml`、`personal-only.toml`、order 表与预算；
  ② 扩展 manifest/generator：`[[ui.contributions]]` 增加 `editions`/`when` 条件，`zk validate`+`zk compose`+`docs/24`/`docs/16` 同步（机制变更更大，需 docs/24 评审）；
  ③ 保留共享 pane + 运行期 props gating，但必须在 spec/design 明确"内建 pane 可按 shell props 隐藏"并删除"无 edition 分支"的绝对表述，接受与 C2 精神的张力。
- 状态：**OPEN**。

#### P1-R2 · approval/mcp：跨包字段新增的编译面未列入写作用域（`ToolSpec`/`Message`/`AgentEvent`/`AppSettings`）
- 位置：approval `spec.md:110-117`（`ToolSpec.sensitive_arg_keys`、`Message.turn_id`、`ToolCallFinished.args/status`）、`tasks.md:17-30`（写作用域只含 6 个工具包）、design:11；mcp `spec.md:137-140`（`AppSettings.mcp`）、`tasks.md:42-49`。
- 问题（Rust 结构体/变体字面量必须列出全部字段，`#[serde(default)]` 不适用于 Rust 构造）：
  - `ToolSpec {` 在 **43 个文件**出现；approval 写作用域内仅 13 个，**30 个在作用域外**（如 `packages/client/tool-fs/src/lib.rs`、`tool-glob`、`tool-grep`、`tool-todo`、`tool-web-fetch` 等 29 个工具/测试文件 + `packages/client/agent-loop/tests/conformance.rs`），新增字段会使 workspace 编译失败；
  - `ToolCallFinished` 构造在 `packages/client/agent-loop/src/lib.rs:101,117`（mock agent loop，不在 approval 写作用域）也同样失败；
  - `Message {` 构造在 `packages/client/tool-session-query/tests/tool.rs`（不在 approval 写作用域）；
  - mcp 的 `AppSettings.mcp` 会使 `packages/client/store-settings-json/tests/conformance.rs` 的 `AppSettings {` 字面量失败（不在 mcp 写作用域）。
  - 预算：approval 38 文件 + 上述 ~30 文件 = 实际 ~68；mcp 27 文件另加 1-2 个测试文件。当前 acceptance 也没有 `cargo clippy --workspace --all-targets`/`cargo test --workspace` 级门禁，CI 会先红。
- 修法（推荐①，最小面）：
  ① 把 `sensitive_arg_keys` 从 `ToolSpec` 改为 `Tool` trait 默认方法（`fn sensitive_arg_keys(&self) -> &'static [&'static str] { &[] }`，仅需声明的工具包覆写）——避免 30 文件扫改；`Message.turn_id` 与 `ToolCallFinished` 字段仍需把 `agent-loop`（src+tests）与 `tool-session-query/tests` 加入写作用域；mcp 把 `store-settings-json/tests/*` 加入写作用域；
  ② 或保留 `ToolSpec` 字段，把全部 30 个作用域外工具包/测试列入 tasks 与写作用域、更新预算例外，并补 workspace 级门禁；
  - 同时把 `cargo check --workspace`（或 clippy/test workspace）加入 approval 与 mcp 的 acceptance（P1-5 的补充）。
- 状态：**OPEN**。

#### P1-R3 · approval：turn 取消路径被 `BridgeSink` 抑制，cancelled 终态行仍不会落库（且 approval-cancel 的 turn 取消 owner 未定）
- 位置：approval `spec.md:31-39`（S3/R4）、`design.md:51-53`（S3）、`design.md:155-163`（写入路径/回归修复）、`proposal.md:72-74`（成功指标 3）；现有代码：`apps/desktop/src/events.rs:59-63`（`cancelled` 时除 `TurnCancelled` 外全部丢弃）、`apps/desktop/src/ipc.rs:1745-1752`（`BridgeSink.cancelled = job.cancelled`）、`apps/desktop/src/ipc.rs:1859-1875`（`chat.cancel` 置 job 标志并 `cancel_agent()`）、`packages/client/agent-rig/src/lib.rs:357-359`（取消提前 return）。
- 问题：
  1. 用户取消 turn（`chat.cancel`）时 `job.cancelled=true`，`BridgeSink.emit` 会在落库前直接 return；approval 设计新增的 `ToolCallFinished{status:cancelled}` 因此**不会写入会话 DB**（bus 事件也被丢弃），R4/acceptance「取消后刷新仍显示已取消」不可达——与首评 P0-3 的验收目标冲突；
  2. S3 要求 approval 卡「取消」后 turn 进入 `turn_cancelled`，但**未定义谁把 turn 置为取消**（`approval.decide{cancel}` 只解除 broker 等待；`job.cancelled` 由 `chat.cancel` 设置）；`approval.decide{cancel}` 与 `chat.cancel` 两条路径的终态/事件/assistant 消息 status 未统一。
- 修法：在 design §2.5.2 明确：① `BridgeSink` 对终态 `ToolCallFinished`（含 `status=cancelled/failed/denied`）**先落库再判断 cancelled**（或把 cancelled 短路移到 bus emit，仅抑制 bus），并保证 `TurnCancelled` 仍可发；② approval-cancel 单一路径：宿主收到 `approval.decide{cancel}` 时同时置 `job.cancelled`/调用 `cancel_agent`（或由 agent-rig 统一发 `TurnCancelled` 并让 `ipc` 用 cancelled 状态写 assistant 行）；③ e2e 增加「工具执行中取消 → 刷新后工具卡为 cancelled 且不白屏」（现有验收只有拒绝/取消手工项，补真实 store 断言）。
- 状态：**OPEN**。

### 12.4 复评新发现 · P2（OPEN，建议修，不阻塞）

1. **P2-N1 settings-ia order 表与 design 细节不一致**：`spec.md:20` 固化「模型与 Provider 20 / 嵌入与媒体 60 / 企业 80」，但 `design.md:44-46` 拆成 `embedding(60)/media(65)`、provider「20 同 pane 合并/或独立 21」，且 design 的内建列表**没有企业 pane（80）**；`tasks.md:27` 仍把 embedding/media 放共享包。修法：冻结 provider 与 media 的 order/合并规则、把企业 pane 的 80 写入 design 列表（或按 P1-R1 方案拆分），去掉"或"。
2. **P2-N2 approval cancel/reject 措辞不一**：`spec.md:26` 说「cancel 由显式取消或断线/窗口关闭触发（fallback reject）」，`spec.md:90-91` 边界表写「窗口关闭 → resolved{reject}/tool denied」，`design.md:52` 写「resolved{cancel} 或 {reject,timed_out}」。修法：按 S3 冻结一条映射（建议：超时 → reject+timed_out；显式取消 → cancel；断线/窗口关闭 → reject）并在表/设计中只写一份。
3. **P2-N3 approval docs/28 口径混用**：`design.md:231` 让 docs/28 记录「状态六态（pending/running/success/failed/denied/cancelled）」，这是工具终态六值，与 AGENTS §7 的 UI 六态（空/加载/错误/离线/流式中/已取消）不是同一概念。修法：分开写「工具终态枚举」与「组件六态」，避免 docs/28 歧义。
4. **P2-N4 mcp e2e 在 mock 段会误跑**：mcp `tasks.md:50` verify 为 `bash scripts/m1-e2e.sh --grep "MCP"`；`scripts/m1-e2e.sh:8-14` 的第一段先 compose `--profile mock` 且 `ZEOKEEL_E2E_SKIP_SERVE=1`，而新 spec 需真实 serve/IPC（`mcp.test` 在 mock transport 无实现）。仅"加入第二段显式清单"不能阻止第一段执行它。修法：在 spec 内用 `test.skip(process.env.ZEOKEEL_E2E_SKIP_SERVE === "1")`、或第一段 `--grep-invert` 增加文件名/标题、或用独立 config；并把命令写清。
5. **P2-N5 mcp design 的 enterprise registry 描述与 settings-ia list slot 不一致**：`mcp/design.md:198` 称 enterprise 对该 slot 生成 `null` stub；settings-ia 的 list slot 设计是 `SettingsPanes = []`/不生成（`settings-ia/spec.md:21`、design:43）。修法：按 list slot 语义改写（无 single-component stub）。
6. **P2-N6 mcp 文档面遗漏/占位符**：mcp 新增 `mcp.test`/`mcp.test.status` 两条 IPC，但 tasks 1/12 与 design §7 的 docs 清单**未含 `docs/26 §5`（IPC 命令表）**（父 spec 明确把 connectors.* 写进 docs/26 §5）；`mcp/proposal.md:54-56` 还留有「`docs/39`? 视实际」占位。修法：docs/26 §5 加入两条命令 + 删除占位符并确定文档目标。
7. **P2-N7 mcp 状态 DTO 与错误码未封闭**：`mcp/design.md:139` `McpTestStatusResult.state: String`（spec R5 为封闭 `pending/ok/error`），C10 风格应用 closed enum；`mcp/spec.md:144` `test_id` 并发错误写「`mcp/unsupported` 或独立码」二选一未定。修法：`state` 用枚举、冻结并发错误码（建议独立 `mcp/test-in-flight`）并进错误码表。
8. **P2-N8 settings-ia 双文件保存失败语义未定义**：`settings.json` 与 `settings.meta.json` 两次写（design:39-40），若 settings 写成功而 meta 失败，`settings.set` 返回错误但 `settings.json` 已改——与"失败不落盘"（spec.md:15）冲突。修法：冻结失败语义（meta 写失败时回滚/补偿、或先写 meta 再写 settings 且失败可重放），并加故障注入测试。
9. **P2-N9 config 弱不变式 vs always-allow**：`config/spec.md:25-28` 说"不存在自动批准 requires_approval==true 工具的路径"，但 S4「始终允许本会话」在用户先前显式选择后不再询问（`approval/design.md:53`），字面上构成自动批准。修法：在 R1 注明"策略求值范围内；S4 为用户显式授权例外"，避免文档口径被逐字反驳。
10. **P2-N10 approval 行号残余漂移**：`approval/proposal.md:15` 引 `packages/contracts/agent/src/lib.rs:165-172` 指 `ApprovalDecision`，实际枚举到 `:174`（首评 P2-1 同类问题，差 2 行）。修法：改为 `:165-174` 或去掉行号。

### 12.5 新拆 `client-mcp-config` 自洽检查

| 检查项 | 结果 |
| --- | --- |
| 五件套齐全（proposal/spec/design/tasks/acceptance） | ✅ 5 文件 |
| 跨 spec 顺序/共享文件 owner 与父 spec 一致 | ✅ `tasks.md:6-34` 与 config `tasks.md:6-40` 同一顺序（settings-ia→approval→config→mcp-config）且本 spec 为最后阶段最终 owner |
| 目标单一、与父 spec 边界清晰 | ✅ 父 `spec.md:4-6` 与 mcp `spec.md:4-6` 互指；父非目标 `proposal.md:54-59` 已移出 MCP；mcp 不写审批/卡片/IA |
| 无第二份审批事件表 | ✅ mcp 目录 `grep approval_requested / 事件序列表` 0 命中；审批引用只出现在"见 approval spec" |
| C10/C11 生成物门禁 | ✅ `tasks.md:49` + `acceptance.md:24-26`（export_schema/schema-check/--check） |
| personal-only 清单是数据列表 | ✅ `spec.md:91-98`、`design.md:184-198`、`tasks.md:48`（不是注释同步） |
| 决策 M-1…M-6 已记录 | ✅ `proposal.md:113-125` 与 `spec.md:168-177` 同一份 |
| 依赖登记（config→connector） | ✅ `spec.md:105-106`、`tasks.md:40`（docs/24 §1 + docs/04 §1.5，docs-first） |
| 与父 spec 共享文件冲突 | ✅ 串行顺序 + 父 spec 写作用域明确"不写 connector/mcp"（`config/tasks.md:67-71`） |
| 复评新发现 | ⚠ P2-N4/N5/N6/N7（见 §12.4）：e2e 门控、null stub 措辞、docs/26 遗漏、状态枚举 |

### 12.6 复评门禁复跑（2026-11-01，独立执行）

| 命令 | exit | 结果 |
| --- | --- | --- |
| `python3 scripts/spec-check.py` | **0** | 0 errors / **4 warnings**（四份 spec 均 `draft`，预期） |
| `python3 scripts/docs-check.py` | **0** | 0 errors / 0 warnings / 41 files |
| `python3 scripts/review-check.py` | **0** | ok（29 records；本记录字段完整，四 change id 均在本节出现） |
| `./target/debug/zk validate` | **0** | enterprise=87/personal=56 packages，roles=90；115 packages scanned |
| `./target/debug/zk compose --edition personal --profile default --channel official --check` | **0** | up to date（17 files） |
| `./target/debug/zk capability --check` | **0** | up to date（45 entries；无新增 capability） |
| `bash scripts/schema-check.sh` | **0** | schema-ts up to date；schema-check ok |
| `bash scripts/i18n-check.sh` | **1** | 仍红：`apps/desktop/web/src/main.tsx:49`（预期由 approval spec 任务 11 修复；`main.tsx:47` 已修，剩 `:49` 一条；实现前不作为 spec 缺陷） |

- 只读证据：复评未修改任何 spec/代码/生成物；`git status --porcelain` 仅显示 15 个 spec 文档修改 + 1 个新 spec 目录 + 本评审记录；`git diff --stat` 只含 spec 文档。
- 未验证（不冒充）：真实 Tauri/e2e（`cargo test`/`pnpm e2e`/`agent-e2e` 都属于实现阶段；本轮未运行），企业组合构建、真实 PG 登录链路。

### 12.7 复评结论

- **首评 P0（4/4）、P1（16/16）、P2（10/10）全部 RESOLVED 或已记录**（P0-3 的规格语义已闭合，其实现细节缺口升级为复评新 P1-R3）。
- **复评新发现 P1 3 条（P1-R1/R2/R3，OPEN）**：均为"实现前必须修"的规格/设计缺口（edition 专属 pane 模型、跨包字段编译面、取消路径落库+事件链）；P2 10 条（N1–N10，含首评 P2-1 残余行号）建议同批修。
- 结论：**CHANGES-REQUESTED**（P0=0 / P1=3 / 新 P2=10）。**不可置 `approved`**：需作者先修 P1-R1/R2/R3（均为 spec 层改动，不涉及已开工代码），再交同一评审员复评；若 Lead 对某条 P1 给出书面豁免（写入对应 proposal §8 或统一 decision 记录），复评时按豁免处理并在本节更新。
- 复评后再核对：§12.1 的 20 文件任一 hash 变化 → 本复评失效，需按新 hash 重新核对差异。
- 四个 change id 供 `review-check` 使用：`2026-11-01-client-settings-ia`、`2026-11-01-client-approval-tool-rendering`、`2026-11-01-client-config-approval-mcp`、`2026-11-01-client-mcp-config`。

## 13. 第三轮复评（2026-11-01）

- 复评者：`spec-reviewer`（同一评审员；独立性同 §头：未参与四份 spec 写作/实现；唯一写入仍为本记录）。
- 复评日期：2026-11-01（第三轮）。
- 复评对象：`HEAD 6a6d24a` + 20 个 spec 文件 SHA256（15 个 tracked spec 文件全部变更 + mcp 5 文件；其中 `client-config-approval-mcp/tasks.md` 与 §12.1 相同，未改）；hash 见 §13.1。
- 本轮输入：§12 的 P1-R1/R2/R3 与 P2-N1…N10 修复声明、Lead 第三轮决策、四份 spec 全文、当前代码（只读）。**未实现任何代码**（`git status --porcelain` 无代码/生成物改动，C16 保持）。
- 结论：**CHANGES-REQUESTED（P0=1 / P1=2 / 新 P2=6）**。§12 全部 P0/P1/P2 已关闭；本轮新发现 1 个 P0（跨契约依赖环，规格不可实现）与 2 个 P1，修复后可再复评。

### 13.1 第三轮对象 SHA256（20 文件）与差异范围

```text
9959a7d15c1169c5bf8c4acdbc77f61848ac9ae4fa546af57beeae521a08f38b  specs/changes/2026-11-01-client-approval-tool-rendering/acceptance.md
e512d4b4fd7d6839aa51f581ad61046ae5b77916167088788b26619034ba374b  specs/changes/2026-11-01-client-approval-tool-rendering/design.md
2e8203198b40eefdad65fbbf2c3abd45b025b093566531db352c8960ae66ceb6  specs/changes/2026-11-01-client-approval-tool-rendering/proposal.md
a35ee4435363ad7ca608300ef878fc7e389e472f5ed6171f906227fbe008c648  specs/changes/2026-11-01-client-approval-tool-rendering/spec.md
1d410c3e19b1613339655af4461098da247e71d0b567be966bc437d848b0a4a8  specs/changes/2026-11-01-client-approval-tool-rendering/tasks.md
b8419402f22caadf16c50cc27a5f4c96677402c309e0ca4d6966ec20aaf932c5  specs/changes/2026-11-01-client-config-approval-mcp/acceptance.md
74adacfefaefd0474040e141a8522ee9332b9ed23c687c50e2d9abd2d0fde00f  specs/changes/2026-11-01-client-config-approval-mcp/design.md
b58f8ac3f743fea62859d2c8818081c5edc12c3fe4f88af1d5d380c53bd3c7b8  specs/changes/2026-11-01-client-config-approval-mcp/proposal.md
aebdc1844f9b33bf693866be3f4dca0121a323bc9d76cd24abb0c02aa3732493  specs/changes/2026-11-01-client-config-approval-mcp/spec.md
04e635a49d49bb4d2d3762b4cc5e14d0e9f7d4e74a2c8019803d1852243fa590  specs/changes/2026-11-01-client-config-approval-mcp/tasks.md
80997bcb1ed9c82645ce2985383f65bc109b8cf4702d5062523fcf79e8560d8c  specs/changes/2026-11-01-client-mcp-config/acceptance.md
8926681911bed5d37eba891b8f2b583533d6e1fdafde6ebce169796c321d86b6  specs/changes/2026-11-01-client-mcp-config/design.md
5e5956a76fc8f20969113718bba25de98dbde9645d7c3dd98a0d8473c59a1c1e  specs/changes/2026-11-01-client-mcp-config/proposal.md
6ecc986b239c94d47158c329bdf7d1892c7265e6ce23048735978b506cb14869  specs/changes/2026-11-01-client-mcp-config/spec.md
2ea576fd7e6d257ba8e7473e6d8c0db117393330dc9c55cae7435f9425c52f95  specs/changes/2026-11-01-client-mcp-config/tasks.md
2d9d50972b68c0046b8d9490c73df185df9aac1577003cf4a984333a09002834  specs/changes/2026-11-01-client-settings-ia/acceptance.md
f9d3d78ac16dae2ab7902efc3f44dac11e0345544b618c3aed9df0465348f93e  specs/changes/2026-11-01-client-settings-ia/design.md
fb6ddc2c3faee5fc1c1d7db25382f8d7e0304626cb949d3078d04d6eda04f356  specs/changes/2026-11-01-client-settings-ia/proposal.md
70160baaf44977a244277b4e65a077f2548ef57a0a70e4964220237bc25a8b81  specs/changes/2026-11-01-client-settings-ia/spec.md
a4176e3c354c863c210b19ec6116e771aaba4d861f19cf3eb73707152f54d788  specs/changes/2026-11-01-client-settings-ia/tasks.md
```

差异范围抽查（"只改要求项"）：
- 与 §12.1 的 20 hash 对照：19 个变更 + `client-config-approval-mcp/tasks.md` 未变；`git diff --stat` 仍只含 15 个 tracked spec 文件（+1480/−466），新增 `client-mcp-config/` 目录；`git status --porcelain` 排除 spec/本记录后为空 ⇒ 无代码/生成物改动。
- 变更落点与 task-13 修复清单一一对应：settings（R1/R2 的 `editions` 贡献过滤、order 表 `provider 21/embedding 60/media 65/enterprise 80`、R4 事务+`settings_hash`、生成器 `UiContributionDecl.editions`、docs/16+24 先行、acceptance 故障注入/workspace check）；approval（`Tool::sensitive_arg_keys` 默认方法、agent-loop/tool-session-query 编译面+workspace check、BridgeSink 终态先落库+`turn_cancelled` 标志+assistant status、cancel 单一路径、S3a–S3c、docs/28 拆分、行号 165–174）；config（R1 弱不变式限定策略求值 + S4 例外）；mcp（`test.skip` mock 段门控、list slot `[]` 语义、docs/26 §5 + 删除占位符、`McpTestState` 枚举 + `mcp/test-in-flight`）。
- 抽查未受影响章节保持原范围：settings R5 a11y、approval R1/R2 卡字段与 R6 i18n、config R3/R5/R6、mcp R1/R2/R3/R6 等均只做评审要求项相关增量，未发现范围扩张。

### 13.2 §12 条目第三轮关闭状态

| 条目 | 状态 | 第三轮依据 |
| --- | --- | --- |
| 首评 P0-1…P0-4（§12.2） | 保持 **RESOLVED** | 本轮改动未回退相关语义（config/spec:25-28、approval/design §2.0、settings spec:69-73、approval R4） |
| P1-R1 edition 专属 pane | **RESOLVED** | `[[ui.contributions]]` 可选 `editions` + `zk compose` 构建期过滤（settings spec:13-15,22-25；design:14,42-51；tasks:24,27；docs/16+24 任务 0） |
| P1-R2 跨包编译面 | **RESOLVED** | `Tool::sensitive_arg_keys()` 为 trait 默认方法、`ToolSpec` 不动（approval spec:121、design:106-113、tasks:17-18）；agent-loop(src+tests)/tool-session-query tests 进写作用域（tasks:18,35；design:160-162）；`cargo check --workspace --all-targets` 进 tasks:31/58、acceptance:28 |
| P1-R3 取消路径 | **RESOLVED（本轮回退见新 P1-13.2）** | BridgeSink 终态先落库再判 cancelled、`turn_cancelled` 标志 + assistant status、approval-cancel 单一路径、chat.cancel 释放 waiter/删除手发事件、取消 e2e（approval design:164-203；spec:54-56；tasks:24,29） |
| P2-N1 order 不一致 | **RESOLVED** | settings spec:22-23 冻结 provider 21/embedding 60/media 65/enterprise 80；design:48-49 与之一致 |
| P2-N2 cancel/reject 措辞 | **RESOLVED** | 唯一映射 S3a–S3c（approval design:52-58；spec:26,93-95；R8:87） |
| P2-N3 docs/28 概念混用 | **RESOLVED** | design:265、tasks:30、acceptance:55 明确拆「工具终态枚举」/「组件六态」 |
| P2-N4 mcp mock 段误跑 | **RESOLVED** | mcp spec:166、tasks:50、design:233、acceptance:34 用 `test.skip(ZEOKEEL_E2E_SKIP_SERVE==="1")`；另用 Playwright 1.64.1 临时探针验证 describe 内 `test.skip(boolean, description)` 会 skip 全 suite（本评审只读执行，未写仓库） |
| P2-N5 list slot null stub 措辞 | **RESOLVED** | mcp design:207-208 改为 `SettingsPanes = []`、无 single-component stub；settings 空列表规则一致 |
| P2-N6 docs/26 + 占位符 | **RESOLVED** | mcp tasks:40,51、design:330、acceptance:54 补 docs/26 §5；proposal 中 `docs/39? 视实际` 占位已删（grep exit=1） |
| P2-N7 状态枚举/错误码 | **RESOLVED** | mcp design:139-147 `McpTestState`（pending/ok/error）+ `mcp/test-in-flight`；spec:66-67,120；tasks:44；acceptance:44 |
| P2-N8 双文件事务 | **RESOLVED** | settings spec:44-48、design:39-41、tasks:22、acceptance:34（备份→settings→meta→失败回滚 + settings_hash + 故障注入） |
| P2-N9 弱不变式 vs S4 | **RESOLVED** | config spec:25-28、design:95-97,214-215、acceptance:40 限定「策略求值范围内」+ S4 显式授权例外 |
| P2-N10 行号漂移 | **RESOLVED** | approval proposal:14 引 `agent/src/lib.rs:165-174`（实测枚举 165-174） |

### 13.3 第三轮新发现 · P0（OPEN，阻塞）

#### P0-13.1 · 跨契约类型引用造成 `zk-contract-tool ↔ zk-contract-agent` 依赖环（按现设计不可编译）
- 位置：`approval/design.md:11`（`zk-contract-agent` 新增 `ApprovalMetadata`/`ApprovalRisk`/`ApprovalDecision`）、`:87-104`（`ApprovalMetadata`/`ApprovalRisk` 定义在 `// zk-contract-agent` 代码块；`ApprovalDecision` 亦在 agent）、`:106-119`（`zk-contract-tool` 的 `Tool::approval_metadata(...) -> Option<ApprovalMetadata>`、`ApprovalGate::decide_scoped(...) -> ApprovalDecision`）；`approval/spec.md:114-121`、`tasks.md:17`、`proposal.md:118-123`。
- 问题：`packages/contracts/agent/Cargo.toml:10` 已依赖 `zk-contract-tool`；若 `ApprovalMetadata`/`ApprovalRisk`/`ApprovalDecision` 定义在 agent contract，而 tool contract 的 trait 方法签名又引用它们，则 tool 必须依赖 agent ⇒ **Cargo 依赖环**（agent→tool 与 tool→agent 同时存在）。这不是实现选择问题，按现设计无法编译（`cargo check --workspace` 必红），也违反 C4 契约单向缝的登记要求。
- 修法（推荐①，改动最小）：把 `ApprovalMetadata`/`ApprovalRisk`/`ApprovalDecision` 定义移到 `zk-contract-tool`（工具/门控是更底层，agent 已依赖 tool），`zk-contract-agent` 用 `pub use zk_contract_tool::{ApprovalDecision, ApprovalMetadata, ApprovalRisk};` 保持 `zk_contract_agent::*` 路径兼容；`ApprovalRequested.metadata` 使用 tool 类型；`ToolCallStatus` 可留 agent。同步更新 `approval/design §1/§2.2`、`spec §4`、`tasks #1`、`docs/24` 契约依赖表，并在 acceptance 记录 `cargo public-api diff`（或按 docs/04 §10 记录 UNVERIFIED）。备选：`ApprovalGate` 的 typed 决策放到 agent 层的扩展 trait（agent-rig 由 tool gate + agent trait 双持），但比移动类型更重。
- 状态：**OPEN**。

### 13.4 第三轮新发现 · P1（OPEN，实现前必须修）

#### P1-13.1 · mcp：`zk-contract-ipc` 引用 `zk-contract-connector::McpServerConfig`，但缺 Cargo 依赖/写作用域/依赖登记
- 位置：`mcp/design.md:12,134-147`（`McpTestPayload { server: zk_contract_connector::McpServerConfig }`）、`tasks.md:44`（任务 5 写作用域仅 `packages/contracts/ipc/{src/lib.rs,tests/schema.rs}`，未含 `Cargo.toml`）、`design.md:210-235` 逐文件清单同样未含 ipc Cargo；`design.md:325-326`/`tasks.md:40` 只登记 `zk-contract-config → zk-contract-connector`。
- 问题：`packages/contracts/ipc/Cargo.toml` 当前仅依赖 `zk-contract-agent`/`zk-contract-config`；加入 connector DTO 后必须新增 `zk-contract-connector` path 依赖（无环，connector 契约无依赖），否则 `cargo test -p zk-contract-ipc`/workspace 编译失败。现有 tasks/acceptance 未列该 Cargo 变更，也未在 docs/24/04 登记 `ipc → connector` 依赖（P2-8 只覆盖 config→connector）。
- 修法：任务 5/write scope 增加 `packages/contracts/ipc/Cargo.toml`；design §1 契约行写明新依赖；docs/24 §1 / docs/04 §1.5 登记 `zk-contract-ipc → zk-contract-connector`（与 config→connector 同批）；acceptance 加 `cargo check --workspace --all-targets`（或至少 `cargo test -p zk-contract-ipc`）证据。
- 状态：**OPEN**。

#### P1-13.2 · approval：'工具执行中取消' 的终态语义未设计（e2e 期望 cancelled，但现有执行路径会写 success）
- 位置：`approval/spec.md:43-56`（R4：结束即发终态；取消路径必须发、落库）、`:129`（e2e 断言工具执行中取消→cancelled）、`acceptance.md:43-44`、`design.md:199-203`（§2.5.5 e2e）、`design.md:176-189`（§2.5.3 只定义 gate 前拒绝/取消路径；成功/失败路径补 success/failed）、`tasks.md:29`（#12）。
- 问题：当前 agent-rig 工具执行是 `spawn_blocking` + `tokio::time::timeout`，取消标志只在**进入工具执行前**检查（`packages/client/agent-rig/src/lib.rs:357-359`）；工具执行中 `chat.cancel` 后，工具会跑完并按 `Ok(text)` 分支发 `ToolCallFinished{success}`（`:397`），不会发 `cancelled`。design §2.5.3 也未定义「outcome 返回后检查 cancelled 并改判 status」；而 e2e/验收要求刷新后卡为 `cancelled`。二者按现文冲突（若实现照 design 的成功路径写，e2e 必失败；若照 e2e 改判，design 未写结果保留/丢弃与安全语义）。
- 修法（二选一并写清）：① 在 design §2.5.3 冻结「工具返回后若 cancelled 已置位 → 发 `ToolCallFinished{status:cancelled}`（并明确 result 保留只读展示/不当作成功副作用），否则按 success/failed」；补单测+慢工具 e2e；② 若产品语义是「执行中取消不改变已完成工具状态」，则把 acceptance/e2e 改为「审批等待中取消 → cancelled/denied」并删除/降级「执行中取消→cancelled」断言。**推荐①并明确 side-effect 诚实展示**（避免把已完成副作用伪装为取消）。
- 状态：**OPEN**。

### 13.5 第三轮新发现 · P2（OPEN，建议修，不阻塞）

1. **P2-N11 mcp spec §4 数据契约漏 `McpTestState`/`McpTestStatusResult`**：`mcp/spec.md:141` 只列 `McpTestPayload`/`McpTestResult`，而 design:12,139-147、tasks:44 有 `McpTestStatusResult`/`McpTestState`；修法：§4 补两个 DTO 与状态枚举（OPEN）。
2. **P2-N12 settings proposal 头部预算陈旧**：`settings-ia/proposal.md:8` 仍写「约 15 个文件…不拆分」，§8:84-88 已是 16–20 + 预算例外；修法：头部改与 §8 一致（OPEN）。
3. **P2-N13 `SettingsPaneContribution` 的 `editions?` 字段与生成数组不一致**：`settings-ia/design.md:72` 类型含 `editions?: string[]`，但 §2.3(b) 生成 `{ id, order, Component }` 不含 editions；运行期无影响，但类型/实现应对齐（要么生成物带 editions，要么类型去掉）（OPEN）。
4. **P2-N14 双文件事务的备份/回滚失败边界未定义**：`settings-ia/spec.md:44-48`/design:39-41 定义「备份→写→回滚」，但备份写失败、回滚失败时的返回/日志/下一状态未写；修法：补故障分支与测试（OPEN）。
5. **P2-N15 config R2 的事件表引用粒度**：`config/spec.md:47-49`、`design.md:101-103` 仍写「S1–S5 / S3（取消/超时/断线）」，approval 权威表已拆 S3a–S3c；语义兼容但不够精确，建议引用 `S1/S2/S3a–S3c/S4/S5`（OPEN）。
6. **P2-N16 契约变更的 public-api diff 证据缺位**：docs/04 §10 要求契约 PR 附 `cargo public-api diff` 与反序列化对拍样本；四份 spec 的 acceptance 均未列该项，且本机 `cargo public-api` 未安装（AGENTS §6.2 UNVERIFIED）。修法：acceptance 增加「public-api diff（或记录工具不可用 UNVERIFIED）」+ 契约对拍样本（OPEN）。

### 13.6 本轮要求的三项跨 spec 检查

| 检查 | 结果 |
| --- | --- |
| settings edition 贡献过滤 vs config/mcp pane 依赖 | **无新矛盾**：mcp pane 由 personal-only 包（`[role].editions=["personal"]`）过滤，无需在 contribution 上重复 editions；config 的审批控件位于共享「工具与审批」pane（order 40，两版都可用，符合 docs/04 §1.5「客户端本地审批由用户决定」）；settings 内建 personal/enterprise pane 由 contribution `editions` 过滤（settings spec:13-15,22-25）。 |
| approval 取消路径 vs config R2 S3c | **语义一致、引用粒度待对齐**：approval 权威表 S3a（显式 cancel）/S3b（超时 reject+timed_out）/S3c（断线/窗口关闭 reject）；config R2 以「S1–S5 / S3 组」引用，未逐列 S3a–S3c（P2-N15）。 |
| mcp 状态枚举 vs 生成 schema | **可实现、spec §4 漏记**：`McpTestState` 为 closed enum（design:139-147），task 5/schema 测试按 C10 生成；spec §4 未列 `McpTestStatusResult`/`McpTestState`（P2-N11）。 |

### 13.7 第三轮门禁复跑（2026-11-01，独立执行）

| 命令 | exit | 结果 |
| --- | --- | --- |
| `python3 scripts/spec-check.py` | **0** | 0 errors / **4 warnings**（四份 spec 均 `draft`，预期） |
| `python3 scripts/docs-check.py` | **0** | 0 errors / 0 warnings / 41 files |
| `python3 scripts/review-check.py` | **0** | ok（29 records） |
| `./target/debug/zk validate` | **0** | enterprise=87/personal=56 packages，roles=90；115 scanned |
| `./target/debug/zk compose --edition personal --profile default --channel official --check` | **0** | up to date（17 files） |
| `./target/debug/zk capability --check` | **0** | up to date（45 entries；无新增） |
| `bash scripts/schema-check.sh` | **0** | schema-ts up to date；schema-check ok |
| `bash scripts/i18n-check.sh` | **1** | `apps/desktop/web/src/main.tsx:47,49` 仍红（由 approval spec 任务 #11 在实现阶段修复；本复评不改代码，预期） |

- 只读证据：本轮未修改任何 spec/代码/生成物；`git status --porcelain` 仅 15 个 tracked spec 修改 + `client-mcp-config/` + 本评审记录；`git diff --stat` 只含 spec 文档。
- 未验证（不冒充）：真实 `cargo test`/`agent-e2e`/Tauri、企业组合构建、真实 PG；契约 public-api diff 工具未安装（见 P2-N16）。

### 13.8 第三轮结论

- **§12 的 P0（4/4）、P1-R1/R2/R3、P2-N1…N10 全部 RESOLVED**（第三轮已逐条核对并记录依据；P1-R3 的取消落库链路已闭合，但其「执行中取消」终态语义是本轮新 P1-13.2）。
- **第三轮新发现**：P0=**1**（P0-13.1 跨契约依赖环，规格不可实现/不可编译）、P1=**2**（P1-13.1 mcp ipc 依赖缺口；P1-13.2 执行中取消终态语义冲突）、P2=**6**（N11–N16）。
- 结论：**CHANGES-REQUESTED（P0=1 / P1=2 / 新 P2=6）**。**不可置 `approved`**：P0-13.1 必须先将 `ApprovalMetadata`/`ApprovalRisk`/`ApprovalDecision` 移到 `zk-contract-tool`（或在 agent 层重构 gate trait）并更新四份相关 spec/tasks；P1-13.1 补 ipc Cargo/依赖登记；P1-13.2 冻结执行中取消语义。修完交同一评审员第四轮复评；若 Lead 对任一新 P0/P1 给出书面豁免（写入对应 proposal §8 或统一 decision 记录），复评按豁免处理并在本节更新。
- 四个 change id 供 `review-check` 使用（复评通过后）：`2026-11-01-client-settings-ia`、`2026-11-01-client-approval-tool-rendering`、`2026-11-01-client-config-approval-mcp`、`2026-11-01-client-mcp-config`。
- 失效条件：§13.1 的 20 文件任一 hash 变化 → 本第三轮复评失效，需按新 hash 重新核对。

## 14. 第四轮复评（2026-11-01）

- 复评者：`spec-reviewer`（同一评审员；独立性同 §头：未参与四份 spec 写作/实现；唯一写入仍为本记录）。
- 复评日期：2026-11-01（第四轮）。
- 复评对象：`HEAD 6a6d24a` + 20 个 spec 文件 SHA256（18 个变更；`client-config-approval-mcp/proposal.md` 与 `client-settings-ia/tasks.md` 未变）；hash 见 §14.1。
- 本轮输入：§13 的 P0-13.1、P1-13.1、P1-13.2 与 P2-N11…N16 修复声明、Lead 第四轮决策、四份 spec 全文、当前代码/生成物（只读）。**未实现任何代码**（`git status` 无代码/生成物改动，C16）。
- 结论：**APPROVED（P0=0 / P1=0；新 P2=3 条，非阻塞）**。

### 14.1 第四轮对象 SHA256（20 文件）与差异范围

```text
6499fbaa11a8f9227a0816f6e3c3d7639b813ac8893fb9317b1ee6d077245893  specs/changes/2026-11-01-client-approval-tool-rendering/acceptance.md
94115facbc34e487ed1d1cc0f8b2ece730b6a1e2489498e9516772ba4e149193  specs/changes/2026-11-01-client-approval-tool-rendering/design.md
4d0bd81834b98422321168b319073cf70b590de67bb519cfe2086307f2a6c732  specs/changes/2026-11-01-client-approval-tool-rendering/proposal.md
8a10d48a3cdc12845b246735f5eb43ee3b3a786c45502e041d64e3891d7cad10  specs/changes/2026-11-01-client-approval-tool-rendering/spec.md
68bccba03135506a7164b066b087808f5d8c816087fcb82727039f4949d9173d  specs/changes/2026-11-01-client-approval-tool-rendering/tasks.md
68ec9c2af3fbef9d0126a8ec7b8f3cc61ed102e084075a57f2ebeaad7b48108a  specs/changes/2026-11-01-client-config-approval-mcp/acceptance.md
a9cd247e8700b4bc3966283ed33aad31de0c432227036773234b0a8f45947c02  specs/changes/2026-11-01-client-config-approval-mcp/design.md
b58f8ac3f743fea62859d2c8818081c5edc12c3fe4f88af1d5d380c53bd3c7b8  specs/changes/2026-11-01-client-config-approval-mcp/proposal.md
650dd716a817399f1512ed555ee768b01b757c769495a91232b24fb04d379dd3  specs/changes/2026-11-01-client-config-approval-mcp/spec.md
6b162344e9f57347ae15b4cc39cd5f82955e062807df52d811f02b8020890934  specs/changes/2026-11-01-client-config-approval-mcp/tasks.md
e7de34e5c8a094183ba1d27017eb0c3159943635741eb775287c8912db4c0db9  specs/changes/2026-11-01-client-mcp-config/acceptance.md
b7ae0780f5ceb7ce632e836ed063be35f8da754207aa2fa6e0fa91defc13fdbe  specs/changes/2026-11-01-client-mcp-config/design.md
a0fa6e21e15b7552200e90554de38dce7356555747089ab470cbb59c1e2ad161  specs/changes/2026-11-01-client-mcp-config/proposal.md
b310cb3a01debbeff7f528361d29f83b3d7092654814d707a04a8c737199665d  specs/changes/2026-11-01-client-mcp-config/spec.md
5c26df882cc21cc7f817cb517dccfbefb1a86bbfc573aae58175fbdbd4447af8  specs/changes/2026-11-01-client-mcp-config/tasks.md
e2911287c0e27b2baf8747ad72499122e2c3e4f927b311d36ef25030f9d9fa7d  specs/changes/2026-11-01-client-settings-ia/acceptance.md
5814fa0a76f8c27cc50c3784c8b359ebdfede90166776d8cb7a65937e8464b87  specs/changes/2026-11-01-client-settings-ia/design.md
63742c911e1b856a35c80b3326f3108312cff79672be372b4465ae2244e5580f  specs/changes/2026-11-01-client-settings-ia/proposal.md
868cd0cd2042c99329c72b1cc91c7c4a6822336beef8954de81ba85795849620  specs/changes/2026-11-01-client-settings-ia/spec.md
a4176e3c354c863c210b19ec6116e771aaba4d861f19cf3eb73707152f54d788  specs/changes/2026-11-01-client-settings-ia/tasks.md
```

差异范围抽查（"只改要求项"）：
- 与 §13.1 的 20 hash 对照：**18 个变更**；`client-config-approval-mcp/proposal.md` 与 `client-settings-ia/tasks.md` 未变。`git diff --stat` 仍只含 15 个 tracked spec 文件（+1543/−468），新增 `client-mcp-config/` 目录；`git status --porcelain` 排除 spec/本记录后为空 ⇒ 无代码/生成物改动。
- 改动落点与 task-16 修复清单一一对应：approval（§1/§2.2/§4/§8 的类型归属与 re-export、§2.5.3 0b 执行中取消、§2.5.4 副作用提示、tasks #1/#5/#9/#11/#12/#14、acceptance §1/§2/§8、proposal §7 第四轮决策/§8 ~42 文件）；mcp（design:12,221、tasks:23,40,44,92、spec:139-147、acceptance:27-30/81 —— ipc Cargo + 依赖登记 + public-api；spec §4 补 `McpTestStatusResult`/`McpTestState`）；config（spec:47-54、design:101-104、acceptance:31/83 S3a–S3c 与 public-api）；settings（proposal:8 预算 16–20、spec:44-48 回滚失败边界、design:39-41/72 生成数组不含 editions、acceptance:26 public-api）。
- 抽查未受影响章节保持原范围（settings R1/R2/R5、approval R1/R2/R3/R6、config R1/R3-R7、mcp R1-R4/R6-R8），未发现范围扩张。

### 14.2 §13 条目第四轮关闭状态

| 条目 | 状态 | 第四轮依据 |
| --- | --- | --- |
| P0-13.1 跨契约依赖环 | **RESOLVED** | `ApprovalMetadata`/`ApprovalRisk`/`ApprovalDecision` 定义移入 `zk-contract-tool`（approval/design:11,87-110；spec:16,117-122；tasks:17）；`zk-contract-agent` `pub use` 保持旧路径（design:112-113）；依赖仅 agent→tool（design:130-131；docs/24 任务 0/13）；`cargo tree -p zk-contract-tool` 当前输出无 `zk-contract-agent`（实测 grep=0/exit 1）；tool Cargo 未新增反向依赖 ⇒ 无环 |
| P1-13.1 mcp ipc→connector 依赖 | **RESOLVED** | `packages/contracts/ipc/Cargo.toml` 加入 `zk-contract-connector`（mcp/design:12,221；tasks:23,44；spec:139-147；acceptance:27）；`docs/24` §1 / `docs/04` §1.5 同时登记 config→connector 与 ipc→connector（tasks:40、design:326-327） |
| P1-13.2 执行中取消终态 | **RESOLVED** | design §2.5.3 0b（outcome 返回后检查 cancelled → `ToolCallFinished{cancelled, result=<已产生文本原样>}`；execute 前取消 result 空；返回模型文本保持原结果；:185-187）；spec R4:56-58；§2 边界表:98；UI 规则 design:206-207；tasks #5/#9/#11/#12；acceptance §2:45 |
| P2-N11 mcp spec §4 漏 DTO/枚举 | **RESOLVED** | mcp spec:140-143 补 `McpTestStatusResult`/`McpTestState {Pending,Ok,Error}` |
| P2-N12 settings 头部预算 | **RESOLVED** | settings proposal:8 改为「约 16–20 个文件…预算例外见 §8」 |
| P2-N13 `SettingsPaneContribution.editions?` 不一致 | **RESOLVED** | settings design:72 生成数组只含 `{id,order,Component}`，并注「editions 只存在于 zk.toml contribution 声明，用于 compose 过滤」 |
| P2-N14 回滚失败边界 | **RESOLVED** | settings spec:48 + design:41：恢复也失败 → 删除 meta、返回错误、下次加载 revision=0 重建并提示；故障注入四类场景（spec:48） |
| P2-N15 config R2 S3 粒度 | **RESOLVED** | config spec:47-54、design:101-104 逐列 S1/S2/S3a–S3c/S4/S5（S3a 显式取消/S3b 超时/S3c 断线） |
| P2-N16 public-api diff 证据 | **RESOLVED（UNVERIFIED 记录）** | approval acceptance:30/92、tasks:31/60/92；mcp acceptance:30/81、tasks:92；config acceptance:31/83；settings acceptance:26 —— 均记「本机未安装 → UNVERIFIED，CI/有工具时必跑」，未冒充已跑 |

### 14.3 第四轮残留 P2（OPEN，非阻塞、建议实现阶段顺手修）

1. **P2-14.1 approval acceptance 两条取消入口的措辞**：`acceptance.md:45` 写「工具执行中取消（审批卡 cancel 与 `chat.cancel` 两条入口各验一次）」，但审批卡 cancel 发生在**审批等待中**（gate 前、result 空），而 `chat.cancel` 才可能发生在工具执行中（result 非空）；建议拆成两条手工检查并分别对应 S3a 与 design §2.5.3 0b，避免测试者误解（OPEN，非阻塞）。
2. **P2-14.2 approval 的「生成 schema 不变」需在实现时核验**：`design.md:131` 称类型移到 tool 后「生成 schema 不变」；实际 `export_schema` 产物归属可能随定义 crate 变化（`agent/tests/schema.rs` 仍可通过 re-export 导出 `agent-approval-decision.json`，tool tests 若不新增同名文件则产物不变）。要求 task #2/acceptance 记录 `shared/contracts/schema` 与 `SCHEMAS`/TS 的 diff，确认无意外 key/文件名变化（OPEN，非阻塞；task #2 的 `--check` 已覆盖）。
3. **P2-14.3 settings tasks 未列 public-api**：settings acceptance:26 已列 `cargo public-api diff`（UNVERIFIED），但 `tasks.md`（本轮未变）的收尾任务/验收前检查未列该证据；建议在 task #13/#14 同步补一行（OPEN，非阻塞）。

### 14.4 本轮要求的三项跨 spec 检查

| 检查 | 结果 |
| --- | --- |
| tool/agent 类型归属对 schema/TS 的影响 | **无破坏性影响（待实现核验）**：类型移到 `zk-contract-tool`、agent `pub use`；`AgentEvent`/`ApprovalDecidePayload` 的 schemars 定义语义不变（re-export 不改变 derive 输出）；`agent/tests/schema.rs` 仍可用 `zk_contract_agent::ApprovalDecision` 导出同名 schema，tool tests 若新增文件则由 `export_schema`/`schema-ts.py` 生成并进 `SCHEMAS`；残留见 P2-14.2。 |
| 取消终态与 config S3 一致性 | **一致**：approval S3a（显式 cancel→turn_cancelled）、S3b（超时 reject+timed_out）、S3c（断线/窗口关闭 reject）；config R2 逐列引用 S1/S2/S3a–S3c/S4/S5（config spec:47-54、design:101-104）；「执行中取消」不是 `approval_*` 事件，而是工具终态 `cancelled`（approval R4），二者不冲突。 |
| ipc Cargo 依赖方向 | **无环**：`zk-contract-ipc → zk-contract-connector`（connector 无契约依赖）；同时存在 `zk-contract-config → zk-contract-connector`；`zk-contract-agent → zk-contract-tool` 保持单向；无 tool→agent、无 connector→ipc/config。docs/24 双依赖登记已进 tasks（mcp tasks:40）。 |

### 14.5 第四轮门禁复跑（2026-11-01，独立执行）

| 命令 | exit | 结果 |
| --- | --- | --- |
| `python3 scripts/spec-check.py` | **0** | 0 errors / **4 warnings**（四份 spec 均 `draft`，预期） |
| `python3 scripts/docs-check.py` | **0** | 0 errors / 0 warnings / 41 files |
| `python3 scripts/review-check.py` | **0** | ok（29 records；四份 spec 仍 draft，approved 后本记录满足其字段/change id 要求） |
| `./target/debug/zk validate` | **0** | enterprise=87/personal=56 packages，roles=90；115 scanned |
| `./target/debug/zk compose --edition personal --profile default --channel official --check` | **0** | up to date（17 files） |
| `./target/debug/zk capability --check` | **0** | up to date（45 entries；无新增） |
| `bash scripts/schema-check.sh` | **0** | schema-ts up to date；schema-check ok |
| `bash scripts/i18n-check.sh` | **1** | `apps/desktop/web/src/main.tsx:47,49` 仍红（归 approval spec task #11 实现阶段修复；本复评不改代码，预期） |
| `cargo tree -p zk-contract-tool`（只读 fact check） | **0**（grep 无 agent，grep exit 1） | 当前 tool crate 无 `zk-contract-agent`；设计不新增反向依赖，支持「无环」结论 |

- 只读证据：本轮未修改任何 spec/代码/生成物；`git status --porcelain` 仅 15 个 tracked spec 修改 + `client-mcp-config/` + 本评审记录；`git diff --stat` 只含 spec 文档。
- 未验证（不冒充）：真实 `cargo test`/`agent-e2e`/Tauri/企业组合/PG 属实现阶段；`cargo public-api` 工具本机未安装（四份 acceptance 均记 UNVERIFIED）。

### 14.6 第四轮结论

- **§13 的 P0（1/1）、P1（2/2）、P2（6/6）全部 RESOLVED**（P2-N16 以「UNVERIFIED 记录」方式关闭，符合 `docs/11` 不冒充原则）。
- 本轮未发现新 P0/P1；残留 3 条 P2（§14.3，非阻塞，建议实现阶段顺手修）。
- **第四轮复评结论：APPROVED**（P0=0 / P1=0；新 P2=3，非阻塞）。
- 生效条件与后续：① 四份 `proposal.md` 状态由 `draft` 改为 `approved`，并把「评审：」指向本记录；② 四份 `acceptance.md` 引用本复评结论与 §14.1 hash；③ `review-check` 在 status=approved 后将校验本记录（已包含四个 change id：`2026-11-01-client-settings-ia`、`2026-11-01-client-approval-tool-rendering`、`2026-11-01-client-config-approval-mcp`、`2026-11-01-client-mcp-config`）与最新结论 `APPROVED`；④ 本记录对「批准后行政性变更」（proposal 状态/评审链接、acceptance 回填）不视为对象变更；除此之外 §14.1 的 20 文件 hash 任一变化仍使本复评失效，需重新核对。
- 实现阶段请按 §14.3 的 3 条 P2 顺手处理，并按各 spec 的 acceptance/tasks 补齐证据（含 `cargo public-api` UNVERIFIED 记录、`cargo tree`/workspace check）。



