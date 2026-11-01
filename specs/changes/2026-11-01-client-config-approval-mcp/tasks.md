# 2026-11-01-client-config-approval-mcp · 任务清单

> 每个任务必须能独立提交、独立验证；没有 `verify` 的任务不算任务。
> 状态：`draft`（C-1…C-6 拍板 + 独立评审复评通过前不得改产品代码，C16/C19）。

## 0. 跨 spec 合并顺序与共享文件 owner（P1-1）

**全局串行顺序（不可并行跨阶段）**：

```text
2026-11-01-client-settings-ia
  → 2026-11-01-client-approval-tool-rendering
  → 2026-11-01-client-config-approval-mcp   （本 spec）
  → 2026-11-01-client-mcp-config
```

理由：这四份 spec 共享 `packages/contracts/ipc`、`packages/client/agent-rig`、`packages/ui/ui-kit`、
`apps/desktop/src/ipc.rs`、`apps/desktop/web/src/i18n.tsx` 与 `docs/`；同一文件在任一阶段**只允许一个 spec 在写**，
后序 spec 必须在前序 spec 合并并跑完其门禁后再改该文件，避免评审 §3 的竞写与 P0-2 的审批链双实现。

**共享文件唯一 owner（最终合并责任）**：

| 共享文件/目录 | 写它的 spec（按阶段） | 唯一合并 owner |
| --- | --- | --- |
| `packages/contracts/ipc/src/lib.rs` | settings-ia（revision DTO）→ 本 spec（`connectors.*`）→ mcp-config（`mcp.test` DTO） | `mcp-config`（最后阶段负责最终一致性 + schema 复核） |
| `packages/client/agent-rig/src/{lib.rs,config.rs}` | approval（always-allow/事件/决策 wire）→ 本 spec（策略求值） | 本 spec（config-approval-mcp） |
| `apps/desktop/src/ipc.rs` | settings-ia（revision 命令）→ approval（决策/事件桥）→ 本 spec（connectors dispatch）→ mcp-config（mcp.test + MCP 应用） | `mcp-config` |
| `packages/ui/ui-kit/web/src/types.ts` | settings-ia（pane 类型/表单类型）→ approval（审批类型）→ 本 spec（策略类型） | 本 spec |
| `apps/desktop/web/src/i18n.tsx` | settings-ia → approval → 本 spec → mcp-config | 本 spec |
| `apps/desktop/web/src/{App.tsx,store.ts}` | settings-ia（保存/revision）→ 本 spec（连接器接线）→ mcp-config（MCP pane 无 App 改动则不动） | 本 spec |
| `packages/ui/ui-settings/web/src/**` | settings-ia（外壳/pane）→ 本 spec（tools pane 审批控件） | 本 spec |
| `editions/personal.toml`、`editions/enterprise.toml`、`editions/personal-only.toml` | settings-ia（如涉及）→ mcp-config（新 personal-only 包） | `mcp-config` |
| `docs/07`、`docs/16`、`docs/19`、`docs/26`、`docs/38`、`KNOWN-LIMITATIONS.md` | 各 spec 写各自段落（本 spec：`docs/07`/`docs/26`/`docs/38`/`docs/19` 审批段/`KNOWN-LIMITATIONS` §3.5） | `mcp-config`（最终合并 + `docs-check`） |
| `e2e/*.spec.ts`、`e2e/serve-m1.sh`、`scripts/agent-e2e.sh`、`scripts/login-e2e.sh` | settings-ia → approval → 本 spec → mcp-config | `mcp-config` |
| 生成物（`shared/contracts/**`、`apps/desktop/web/src/generated/**`、`docs/generated/**`） | 由各阶段最后一步运行生成器 | 最后一个 spec（`mcp-config`）负责最终 `--check` |

**审批链边界（P0-2，评审要求）**：`should_ask`（策略求值）归本 spec 的 agent-rig 改动；
`always-allow（会话允许）`、`approval.decide` wire、`ApprovalRequested/Resolved` 事件对序列归
`2026-11-01-client-approval-tool-rendering`（其 `specs/changes/2026-11-01-client-approval-tool-rendering/design.md §2.0`《审批事件序列表》**S1/S2/S3a–S3c/S4/S5** 为**唯一真源**；本 spec 只负责 **S5 判定**，不复制表；P2-N15）。审批链整体 golden 测试由 approval spec 提供，
本 spec 只增加「策略短路时不发事件」的负向测试；合并在 approval spec 之后，避免两份实现。

## 1. 任务表

| # | 任务 | 写作用域（文件/目录） | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | **决策落文档（docs-first，C12/P1-2）**：审批策略封闭枚举 + 弱不变式 + 4 个免审批 mutating 工具清单写入 `docs/38` §2；`connectors.*` 契约/触发时机写入 `docs/26` §5 与 `docs/07` §6.1；`docs/19` 设置段补审批策略；`docs/15` §C 登记 D 编号（**C-6 已定：不新增 ADR**；同时写 `docs/07` §6.1 + `docs/38` §2） | `docs/38-agent-runtime.md`、`docs/26-workspace-and-conversations.md`、`docs/07-server-domains.md`、`docs/19-client-ux-flows.md`、`docs/15-open-questions.md` | — | `python3 scripts/docs-check.py`、`python3 scripts/docs-command-check.py` | ☑ |
| 2 | **契约 config**：`ApprovalPolicy`（自定义 `Deserialize` 未知→Ask）、`ToolSettings.approval_policy: Option<...>`、`effective_approval_policy()`、`AppSettings::normalize_approval_policy()`；conformance/schema 测试（真值表/弱不变式/迁移 4 形态/未知值/幂等） | `packages/contracts/config/{src/lib.rs,tests/conformance.rs,tests/schema.rs}` | 1 | `cargo test -p zk-contract-config`（含 `approval_policy`/`approval_migration`/`approval_unknown`） | ☑ |
| 3 | **agent-rig 策略消费**：`AgentRuntimeConfig.approval_policy`、`from_contract`/`default_agent_config` 映射（含 `ZEOKEEL_APPROVAL_POLICY` 测试 fallback）、`ToolRunContext.approval_policy`、`dynamic_tools` 用 `should_ask` 替换 `if requires_approval`；事件顺序不变 + auto 路径无审批事件单测 | `packages/client/agent-rig/{src/lib.rs,src/config.rs,tests/*}` | 2（需 approval spec 已合并 agent-rig） | `cargo test -p zk-agent-rig approval_policy` | ☑ |
| 4 | **契约 ipc**：`CONNECTORS_CONFIGURE/RELOAD/CLEAR` 常量 + `ConnectorsConfigurePayload`/`ConnectorsConfigureResult`；schema 导出 | `packages/contracts/ipc/{src/lib.rs,tests/schema.rs}` | 2（settings-ia 的 ipc 改动已合并） | `cargo test -p zk-contract-ipc && cargo test -p zk-contract-ipc export_schema` | ☑ |
| 5 | **desktop**：boot/`settings.set` 调用 `normalize_approval_policy`（文件存在才回写）；`connectors.*` 改用契约常量 + 封闭错误码（行为不变）；dispatch 单测（含非法 catalog 保留旧配置/幂等） | `apps/desktop/src/ipc.rs`（含内联 tests） | 2,3,4 | `cargo test -p zk-desktop connectors_dispatch && cargo test -p zk-desktop settings_normalize` | ☑ |
| 6 | **desktop web 接线**：`store.ts` 返回 `{items,catalog}`；`App.tsx` 登录/启停后 `connectors.configure`、登出 `connectors.clear`、新增「重新加载」→ `connectors.reload`；错误可见 | `apps/desktop/web/src/{store.ts,App.tsx}` | 4,5 | `pnpm --dir apps/desktop/web test:unit` | ☑ |
| 7 | **i18n**：新增错误/成功文案键（zh-CN/en-US），无硬编码 | `apps/desktop/web/src/i18n.tsx` | 6 | `bash scripts/i18n-check.sh` | ☑ |
| 8 | **UI 审批控件**（C-4 已定：放 settings-ia「工具与审批」pane order 40；C-5 已定：strict 不豁免交互工具）：settings-ia pane 内加三选一 + 说明文案，替换旧「自动批准只读」；`ui-kit` 手写镜像加 `ApprovalPolicy`/`approval_policy`；六态/a11y/i18n | `packages/ui/ui-settings/web/src/panes/tools.tsx`、`packages/ui/ui-kit/web/src/types.ts`、`packages/ui/ui-settings/web/tests/*` | 1,2 + **settings-ia（pane slot/外壳/共享表单类型）已合并** | `pnpm --dir packages/ui/ui-settings test:unit && pnpm --dir packages/ui/ui-kit test:unit && pnpm -r typecheck` | ☑ |
| 9 | **生成物重生成**：`cargo test --workspace export_schema` + `python3 scripts/schema-ts.py` + `zk compose`；逐字节 `--check` | 生成物（`shared/contracts/**`、`apps/desktop/web/src/generated/**`、`docs/generated/**`）仅由生成器写 | 2,3,4,5 | `cargo test --workspace export_schema && git diff --exit-code && bash scripts/schema-check.sh && zk compose --edition personal --profile default --channel official --check && zk capability --check` | ☑ |
| 10 | **e2e（真实运行器，P1-14）**：扩展 `e2e/agent-rig.spec.ts`（strict/ask/auto_read_only 策略 + 热切换）；扩展 `e2e/m3-client-content.spec.ts`（toggle→configure、重新加载→reload、登出→clear） | `e2e/agent-rig.spec.ts`、`e2e/m3-client-content.spec.ts`、必要时 `scripts/agent-e2e.sh` | 5,6,8 | `bash scripts/agent-e2e.sh`；`bash scripts/login-e2e.sh`（企业链路，需本机 PG；无 PG 记 UNVERIFIED） | ☑ |
| 11 | **文档回写 + CHANGELOG**：`docs/07` §6.1、`KNOWN-LIMITATIONS.md` §3.5 口径修正；`CHANGELOG.md`（`auto_approve_read_only` deprecated + 映射）；`docs/19`/`docs/38`/`docs/26` 与实现对齐 | `docs/07-server-domains.md`、`docs/19-client-ux-flows.md`、`docs/26-workspace-and-conversations.md`、`docs/38-agent-runtime.md`、`KNOWN-LIMITATIONS.md`、`CHANGELOG.md` | 1,5,6,8 | `python3 scripts/docs-check.py`、`! rg -q "web 侧接线" docs KNOWN-LIMITATIONS.md` | ☑ |
| 12 | **收尾**：`acceptance.md` 填证据路径（`target/evidence/2026-11-01-client-config-approval-mcp/`）；全量门禁复跑；回写 `research/client-dsh-parity-2026-11-01.md` §7 状态 | 本 spec 目录、`research/client-dsh-parity-2026-11-01.md`（仅状态标注） | 1–11 | `python3 scripts/spec-check.py && python3 scripts/docs-check.py && python3 scripts/review-check.py` + §2 全绿 | ☑ |

## 2. 写作用域

- 本变更只写：
  - 契约/实现：`packages/contracts/{config,ipc}/**`、`packages/client/agent-rig/**`、`apps/desktop/src/ipc.rs`；
  - 前端：`apps/desktop/web/src/{store.ts,App.tsx,i18n.tsx}`、`apps/desktop/web/tests/**`、
    `packages/ui/ui-settings/web/src/panes/tools.tsx`、`packages/ui/ui-kit/web/src/types.ts`；
  - e2e：`e2e/agent-rig.spec.ts`、`e2e/m3-client-content.spec.ts`（必要时 `scripts/agent-e2e.sh`）；
  - 文档：本 spec 列表（`docs/07`、`docs/15`、`docs/19`、`docs/26`、`docs/38`、`KNOWN-LIMITATIONS.md`、`CHANGELOG.md`）。
- 明确不写：
  - 生成物手改（`shared/contracts/**`、`apps/desktop/web/src/generated/**`、`docs/generated/**`）——由任务 9 生成器写；
  - 审批卡/工具行/`events.rs`/`approval.rs`/`packages/contracts/agent` 的事件字段（approval spec）；
  - `packages/contracts/connector`、`packages/client/connector-mcp`、`AppSettings.mcp`、`mcp.test`、`ui-settings-mcp`（mcp spec）；
  - settings-ia 的 slot/生成器/revision、`packages/client/store-settings-json`（settings-ia）；
  - `apps/server/**`、admin SPA、`deploy/**`。
- 并行约束（见 §0）：本 spec 的 agent-rig/ipc.rs 改动必须在 approval spec 合并后串行进行；`ui-settings` 改动必须在
  settings-ia 合并后进行；同文件同一时间只有一个 spec 在写。

## 3. 提交顺序

1. 任务 1（决策落文档）→ 2（契约）→ 4（ipc 契约）
2. 任务 3（agent-rig，approval 合并后）→ 5（desktop）→ 6（web）→ 7（i18n）
3. 任务 8（UI，settings-ia 合并后）→ 9（生成物）
4. 任务 10（e2e）→ 11（文档回写）→ 12（收尾）
- 任务 2/4 可并行（不同 crate）；任务 5 依赖 2/3/4；任务 6 依赖 4/5；任务 8 依赖 settings-ia；任务 9 依赖 2–5。

## 4. 验收前检查

- [x] `python3 scripts/spec-check.py`、`python3 scripts/docs-check.py`、`python3 scripts/review-check.py`
- [x] 变更涉及的 T0/T1 命令全绿（见任务表 verify 列）：`cargo test -p zk-contract-config -p zk-contract-ipc -p zk-agent-rig -p zk-desktop`、
      `cargo test --workspace export_schema && git diff --exit-code`、`bash scripts/schema-check.sh`、
      `zk compose --edition personal --profile default --channel official --check`、`zk capability --check`
- [x] 无手改 `@generated`（生成物由 `export_schema`/`schema-ts.py`/`zk compose` 重生成，`--check` 绿）（`git diff` 只显示生成器输出）
- [x] 弱不变式有负向测试；4 个免审批 mutating 工具在 `ask` 下不询问有断言
- [x] 跨 spec 顺序已遵守（§0），共享文件 owner 已按表交接
- [x] PR 只包含本 spec 范围（不含 approval/mcp/settings-ia 文件）
- [x] `acceptance.md` 已填证据路径；C-1…C-6 已定（proposal §7），独立评审复评通过后状态改为 `approved`（C16）
- [x] 契约 public-api diff（`cargo public-api diff`；本机未安装 → 记录 **UNVERIFIED**）与反序列化对拍样本已附（docs/04 §10，P2-N16）
