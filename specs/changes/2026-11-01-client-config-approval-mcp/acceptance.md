# 2026-11-01-client-config-approval-mcp · 验收与证据

- 验收人：待填（实现完成后由非实现者验收，C19）
- 日期：待填
- PR/commit：待填
- 评审记录：`docs/reviews/2026-11-01-client-ui-parity-specs.md`（CHANGES-REQUESTED；复评通过后在此追加结论与日期）

> 状态：`draft`（未开工；C-1…C-6 拍板 + 复评通过前不改产品代码）。
> 证据根目录（预填，P2-9）：`target/evidence/2026-11-01-client-config-approval-mcp/`
> （`logs/`、`screenshots/`、`axe.json`、`e2e-agent-rig.log`、`e2e-login.log`、`schema-diff.txt`）。
> 本文件只写验收口径，不代替可执行证据（`docs/11`）。

## 1. 验收命令与结果

| 命令 | 结果 | 证据路径 |
| --- | --- | --- |
| `python3 scripts/spec-check.py` | 待跑 | `target/evidence/2026-11-01-client-config-approval-mcp/logs/spec-check.log` |
| `python3 scripts/docs-check.py` | 待跑 | `.../logs/docs-check.log` |
| `python3 scripts/review-check.py` | 待跑 | `.../logs/review-check.log` |
| `python3 scripts/docs-command-check.py` | 待跑 | `.../logs/docs-command-check.log` |
| `zk validate` | 待跑 | `.../logs/zk-validate.log` |
| `zk compose --edition personal --profile default --channel official --check` | 待跑 | `.../logs/compose-check.log` |
| `zk capability --check` | 待跑 | `.../logs/capability-check.log` |
| `bash scripts/schema-check.sh` | 待跑 | `.../logs/schema-check.log` |
| `cargo test --workspace export_schema && git diff --exit-code` | 待跑 | `.../schema-diff.txt` |
| `cargo test -p zk-contract-config` | 待跑 | `.../logs/cargo-contract-config.log` |
| `cargo test -p zk-contract-ipc` | 待跑 | `.../logs/cargo-contract-ipc.log` |
| `cargo test -p zk-agent-rig approval_policy` | 待跑 | `.../logs/cargo-agent-rig.log` |
| `cargo test -p zk-desktop connectors_dispatch` | 待跑 | `.../logs/cargo-desktop-connectors.log` |
| `cargo test -p zk-desktop settings_normalize` | 待跑 | `.../logs/cargo-desktop-normalize.log` |
| `cargo public-api diff`（契约 API 破坏检查；**本机未安装 `cargo-public-api` → 记 UNVERIFIED**，CI/有该工具时必跑） | UNVERIFIED（本机） | `.../logs/public-api-diff.log` |
| 契约反序列化对拍样本（旧 JSON → 新 `AppSettings`/`ApprovalPolicy` 的 conformance golden，docs/04 §10） | 待跑 | `.../contract-golden.json` |
| `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build` | 待跑 | `.../logs/pnpm.log` |
| `pnpm --dir apps/desktop/web test:unit` | 待跑 | `.../logs/pnpm-desktop-web.log` |
| `bash scripts/i18n-check.sh` | 待跑 | `.../logs/i18n-check.log` |
| `bash scripts/agent-e2e.sh`（真实 personal rig；审批策略） | 待跑 | `.../e2e-agent-rig.log` + 截图 |
| `bash scripts/login-e2e.sh`（企业连接器链路；**需本机 PG**，无则 UNVERIFIED） | 待跑 | `.../e2e-login.log` |
| `zk compose --edition enterprise --profile default --channel official` + `cargo check --workspace`（企业回归） | 待跑 | `.../logs/enterprise-check.log` |

## 2. 手工检查

- [ ] **弱不变式**（策略求值范围内）：`cargo test` 真值表 3 策略 × 4 组合；`requires_approval==true` 全部「询问」；S4 always-allow 为用户显式授权例外（approval spec）；
      `ask` 下 4 个免审批 mutating 工具（`schedule`/`job_kill`/`goal`/`mcp_call`）不询问有断言。
- [ ] **策略行为（真实 rig）**：`strict` 下对 `fs.read` 出现审批卡；`ask` 下不出现；`auto_read_only` 下 mutating 工具询问、
      只读不询问；切换策略后**下一轮**生效（不重启）。
- [ ] **事件链边界**：策略短路路径无 `ApprovalRequested`/`ApprovalResolved`；询问路径序列由 approval spec 定义且不回归。
- [ ] **迁移/降级**：`auto_approve_read_only=true→ask`、`false→strict`；保存后写回规范化形态且幂等；
      未知值（`"bogus"`）与类型错不阻止启动、降级 `ask`。
- [ ] **connectors 契约/接线**：登录/刷新后 `connectors.configure`；连接器启停后无需重登生效；「重新加载」→ `connectors.reload`；
      登出 → `connectors.clear`；重复点击不产生重复会话/token；非法 catalog 保留旧配置并显示错误。
- [ ] **六态**（审批控件/连接器接线）：空（默认 `ask`/空列空态）、加载（`settings.get` 中）、错误（`settings.get`/`connectors.*` 失败）、
      离线（personal 本地保存可用；enterprise 连接器失败可见不假装成功）、流式中（保存中禁用重复提交）、已取消（关闭丢弃草稿）。
- [ ] **a11y/i18n**：审批控件键盘可达、Esc 关闭、焦点回位；axe 0 serious/critical；zh/en 无硬编码新增（`i18n-check` 绿）。
- [ ] **C-5 已知 UX**：`strict` 下 `ask_user_question` 先出现审批卡、批准后出现提问卡（双重交互），行为与 spec §2 记录一致、不崩溃。
- [ ] **兼容**：旧设置文件可直接加载；旧前端不调用 3 条新命令正常工作；回滚旧版本读新文件不崩。
- [ ] **文档口径**：`rg "web 侧接线" docs KNOWN-LIMITATIONS.md` 无命中；`docs/38` §2 / `docs/26` §5 / `docs/07` §6.1 与实现一致。

## 3. 文档与决策

- [ ] SSOT 文档已更新：`docs/38` §2、`docs/26` §5、`docs/07` §6.1、`docs/19`（审批策略段）、`KNOWN-LIMITATIONS.md` §3.5
- [ ] 决策已记录：C-1…C-6 已定（Lead 2026-11-01；`docs/15` §C D 编号 + `docs/07` §6.1 + `docs/38` §2；**C-6 已定不新增 ADR**）
- [ ] `docs/README.md` 已登记新主题：不适用（沿用既有主题）
- [ ] `CHANGELOG.md`：`auto_approve_read_only` deprecated + 映射说明；`connectors.*` 新命令说明
- [ ] `docs/reviews/2026-11-01-client-ui-parity-specs.md` 已追加复评结论（评审记录生命周期见 `docs/reviews/README.md`）
- [ ] 变更 spec 已归档到 `specs/archive/`（或在 `specs/changes/README` 状态表更新）

## 4. 回滚

- 回滚步骤：回退 commit（无 server/DB 迁移、无镜像变更）。
- 数据影响：设置 JSON additive；回滚后旧版忽略 `approval_policy`，按 `auto_approve_read_only` 镜像值工作（语义不变）。
- 已验证回滚：待填（需一次「新版写设置 → 旧版启动」兼容验证）。

## 5. 剩余风险 / 后续

| 项 | 影响 | 后续 spec |
| --- | --- | --- |
| `auto_read_only` 语义已定 C-1=A（只读自动 + mutating 全询问）；若用户后续改选需小改 R1 一行 | 低 | 本 spec（R1 一行） |
| `strict` 对交互类工具的双重交互（C-5 已定：Phase 1 不豁免，记为已知 UX） | 低 | Phase 2 评估豁免白名单 |
| 审批卡安全字段/工具行持久化/`usage` | 高 | `2026-11-01-client-approval-tool-rendering` |
| `connectors.*` 远端生命周期（守护/健康检查/重连） | 中 | KNOWN-LIMITATIONS 既有残余，不属本 spec |
| 企业 e2e 依赖本机 PG（login-e2e） | 中 | 环境项，无 PG 记 UNVERIFIED |
| MCP 本地配置/`mcp.test`/企业 MCP 来源 | 高 | `2026-11-01-client-mcp-config` |
| `cargo public-api diff` 工具本机未安装（docs/18 §7.2 UNVERIFIED）；契约破坏检查依赖 CI/后续接线 | 中 | 实现阶段 CI 门禁 |

## 6. 结论

- [ ] `done`：证据齐全，可合并
- [ ] `abandoned`：原因：待填

## 7. 复用证据（C17）

- 搜索记录：`./target/debug/zk find approval` → `kit.approval`；`find connector` → `contract.connector`；
  `find settings` → `store.settings.json`；`find config` → `contract.config`（各 1 命中）。
- 复用清单：
  - `kit.approval`（`crates/zk-kit-approval/src/lib.rs`）→ 审批 gate 语义不变；
  - `contract.tool::ApprovalGate`（`packages/contracts/tool/src/lib.rs:55-75`）→ 不改；
  - `AgentRuntimeConfig`/`default_agent_config`（`packages/client/agent-rig/src/config.rs:455-475`）→ 复用热读管线；
  - `apps/desktop/src/ipc.rs` 的 `apply_connector_catalog`/`sync_connector_catalog`/`restore_connector_oauth_tokens`（`:338-410`）→ 复用；
  - `settings.set` 管线与 `masked_settings` → 复用；
  - settings-ia 的 `ui.settings.pane`/`SettingsPaneProps` → 复用（不新建 pane 机制）。
- 新增能力登记：`ApprovalPolicy::should_ask`（`contract.config` 既有条目）；`connectors.*` IPC DTO（`contract.ipc` 既有条目）。
  未新增 capability id / 未新增包。
