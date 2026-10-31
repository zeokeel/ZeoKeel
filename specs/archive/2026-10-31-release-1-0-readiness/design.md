# 2026-10-31-release-1-0-readiness · 技术设计

> 规范细节链接 `docs/`，不在这里复制第二份规范。组合/manifest 以 `24` 为准，API 以 `04` 为准。

## 1. 影响面

| 类型 | 变更 |
| --- | --- |
| 包 | 修改 `zk-srv-jobs`（rustdoc）；`tool-terminal`/`tool-pwsh-persistent`/`tool-goal`/`tool-ralph`/`tool-agent-team`（R9.1）；`agent-rig`/`agent-loop`/`zk-kit-session`/`zk-contract-agent|session|config`（R9.2/R9.3）；`ui-settings`/`admin-ui-kit`（R8/R9.3）；不改包集合/角色（无新增包，除非 R9.2 需要新 kit，需 ADR） |
| 契约 | 可能新增 `AppSettings.agent.hooks/memory`、`AppSettings.web_search`、`SessionStore::messages_in`、`AgentEvent::Usage`；全部 additive + `#[serde(default)]`，不破坏 v1/v2 |
| 路由 | 无新增；server API 不变 |
| 迁移 | 仅当 run-state 持久化需要：新增 `session_run_state` 表（append-only migration，可空） |
| 生成物 | `crates/zk-composed`、`apps/*/src/generated/*`、`shared/contracts/*`、`docs/generated/*` 由 `zk compose/capability/docs/db lock` 重生成；禁止手改 |
| edition/profile/channel | personal×default×official 基线；enterprise 校验仅 compose+`cargo check`（非基线不支持 `--check`） |
| 文档 | `docs/11/12/18/20/21/24/33/37/38/40`、`README.md`、`AGENTS.md`、`CHANGELOG.md`、`KNOWN-LIMITATIONS.md`、`LICENSE`、`docs/README.md` 登记 |

## 1.5 复用检查（C17，必填）

| 要实现的点 | 现有能力（capability id / 路径） | 结论 | 归属 |
| --- | --- | --- | --- |
| terminal 多会话 | `tools/`/`packages/client/tool-terminal` 现有单会话实现；`tool-bash-persistent` 的会话表模式 | 扩展 | `tool-terminal` |
| pwsh-persistent | `tool-pwsh`（平台门控）+ `tool-bash-persistent`（持久会话模式） | 扩展/复用模式 | `tool-pwsh-persistent` |
| goal round driver | `tool-goal` + `agent-rig` 循环/`subagent` runner | 扩展 | `tool-goal` / `agent-rig` |
| ralph fresh context | `tool-ralph` + `subagent-fork` 的上下文快照 | 扩展 | `tool-ralph` |
| agent-team | `tool-agent-team`（现 experimental）+ `zk-kit-subagent` 注册表/runner | 扩展 | `tool-agent-team` |
| rig run-state/telemetry | `rig-core`/`rig-agent` 0.45（需 spike 确认公开 API）；`agent-rig` 现有 hook/usage | 复用（若 API 存在）否则记录 UNVERIFIED | `agent-rig` |
| messages_in 回放 | `store-session-sqlite`（SQLite v4，已有 messages） | 扩展 | `store-session-sqlite` / `zk-contract-session` |
| ModelCatalog 上下文窗口/压缩 | `rig-agent` ModelCatalog + `agent-rig` 配置 | 扩展 | `agent-rig` |
| hooks/memory/web-search 配置 | `AppSettings.agent`/`media`/`embedding` 模式；`ui-settings` 分 section 发布 | 扩展 | `zk-contract-config` / `ui-settings` |
| admin 响应运行时校验 | `packages/admin/ui-kit/web/src/validation.ts`（Ajv）+ `shared/contracts/schema` | 扩展 | `admin-ui-kit` |
| known limitations / release notes | `docs/37`、`deploy/README` 既有 UNVERIFIED 清单 | 扩展 | `docs/40` 或根 `KNOWN-LIMITATIONS.md` |
| a11y 矩阵 | `e2e/m6-a11y.spec.ts`（admin）、`login-e2e` | 扩展 | `e2e/` |

- 搜索证据：`docs/generated/CAPABILITY-INDEX.md`（45 条，含 `kit.media`/`vector.store`/`media.*`/`agent.hooks`）；`docs/generated/AGENT-TOOL-MATRIX.md`；`rg` 现有 `tool-*`/`cap-*`/`store-*` 实现。
- 若"新建"：仅当 rig spike 证明确需新适配层时新建 `crates/zk-kit-*`，须先补 ADR；禁止新建 `utils/helpers/common`。
- 新增可复用能力：预计无新 `@capability`；R9.2 若新增 rig 能力适配，登记 `agent.telemetry`/`agent.run_state` 并更新能力索引。

## 2. 方案

分阶段、写作用域互斥，每阶段独立提交 + verify。**A 阶段先恢复"可发布工程"**，**B 阶段补齐功能缺口**，**C 阶段发布候选**。

### 2.0 阶段与写作用域

| 阶段 | 工作流 | 写作用域（相对根） |
| --- | --- | --- |
| A | W1 rustdoc | `packages/server/jobs/` |
| A | W2 CI/发布 | `.github/workflows/` |
| A | W3 安全门禁 | `scripts/security-audit.sh`、`docs/21`、`docs/reviews/` |
| A | W4 M6 演练 | `scripts/{backup-restore,upgrade-drill,perf-budget,reproducible-build,fault-injection}.sh`、`specs/changes/2026-10-30-hardening-release/acceptance.md` |
| A | W5 版本/文档 | 根 `README/CHANGELOG/LICENSE/KNOWN-LIMITATIONS`、`AGENTS.md`、`docs/{12,20,37,40,README}`、`Cargo.toml`、`package.json`、`apps/desktop/tauri.conf.json` |
| A | W6 严格性 | `packages/admin/ui-kit/web/src/`（+tests） |
| A | W7 发布候选 | `scripts/build-dist.sh`、`deploy/`、`deploy/updates/` |
| A | W8 personal 裁剪（R8.2） | `tools/zk/src/main.rs`（compose targets 过滤）、`tools/zk/templates/`、生成物重生成、`tools/zk` 测试 |
| B | B1 工具补齐 | `packages/client/tool-{terminal,pwsh-persistent,goal,ralph,agent-team}/`、必要时 `crates/zk-kit-*` |
| B | B1 工具补齐 | `packages/client/tool-{terminal,pwsh-persistent,goal,ralph,agent-team}/`、必要时 `crates/zk-kit-*` |
| B | B2 rig 接线 | `packages/client/agent-rig/`、`crates/zk-kit-*`、契约、`store-session-sqlite` |
| B | B3 配置化 | `packages/contracts/config/`、`packages/ui/ui-settings/`、生成 glue 输入 |
| B | B4 a11y/i18n | `e2e/`、`packages/ui/*/web/src/`、`packages/admin/*/web/src/` |
| B | B5 真实环境 harness | `scripts/`、`e2e/`（env-gated，缺 key 时 UNVERIFIED） |
| C | 发布 | tag/CI/release 实跑；Lead 执行 push/tag |

### 2.1 关键流程

```text
A 阶段：A1 修门禁 → A6/A8 安全/M6 证据 → A10 版本 → A2/A3/A4/A5 CI（A4 依赖 A10/A17 的版本与产物契约）
       → A11–A14 文档 → A15 严格性 → A16/A18 交付与 personal 裁剪 → A17 本地 RC 证据
B 阶段：B6 spike(rig API，已完成) → B7 usage 事件 → B8 回放 → B9 effect/run-state 子集 → B10a 窗口/压缩 → B4 ralph
       → B1/B2 工具会话（同 owner）→ B11 配置 → B12 a11y → B13 i18n → B5 agent-team → B14 harness → B15 矩阵收敛
C 阶段：全量门禁 → push 分支触发 CI → 修复 CI 差异 → tag v1.0.0-rc.1 → release 产出 dist → 归档 spec
```

### 2.2 数据结构

- `AppSettings` 新增 additive section（`agent.hooks`/`agent.memory`/`web_search`），schema version 递增但旧设置可读（`#[serde(default)]`）。
- `SessionStore::messages_in(conversation_id, limit)` 契约方法；SQLite 查询现有 messages 表，无迁移。
- 若 run-state 持久化：新表 `session_run_state(conversation_id, state_json, updated_at)`，append-only 迁移。
- `AgentEvent` 新增 `Usage` 变体（additive；未知事件忽略并记日志）。

### 2.3 已冻结行为（实现以 spec §6 为准）

- B5 agent-team = coordinator + `zk-kit-subagent` workers（cap 8/depth 3），新增 ADR。
- B8 SQLite 为历史权威；B3 goal 轮次语义；B10 上下文预算/压缩（catalog context_window + rig-memory plan_summary）。
- R8.2 personal 裁剪：`zk` 生成器按 `targets` 过滤服务端 glue；发布按 edition 只产物对应目标；三处断言（plan 闭包/生成物/产物）。
- rig 不可实现项仅 `live runner 中途 checkpoint`（`RunOrigin`/`build_run` 为 `pub(crate)`），进 known limitations + 最终验收前用户确认。

## 3. 备选与取舍

| 方案 | 优点 | 缺点 | 结论 |
| --- | --- | --- | --- |
| A 先 A 阶段后 B 阶段 | 先恢复可发布工程，风险可控；可随时出 RC | 1.0 功能补齐晚 | 采用 |
| B 功能补齐优先 | 功能先完整 | 交付工程仍空白，无法发布 | 放弃 |
| C 只做 A 阶段（known limitations） | 最快出 RC | 不满足用户 D2="全部功能缺口" | 放弃（用户已否决） |

## 4. 安全与隐私

- 威胁/滥用：W3 按 `docs/21` §10 13 项逐项落检查；新增工具（terminal 多会话/pwsh-persistent/agent-team）必须沿用工作区根、审批、命令白名单外无额外豁免；不得引入网络放宽。
- 密钥/令牌：配置新增 section 的 secret 只写/masking，沿用 `settings.get` 掩码约定；企业版按 C7 拒绝本地媒体/embedding 配置。
- 审计：安全审计脚本输出 fail-closed；CI strict 模式 unverified=fail；发布流程不得在审计失败时继续。

## 5. 发布与回滚

- 灰度/强制更新：`1.0.0-rc.1` 为预发布，`min_supported` 保持同版本；不启用强制更新。
- 回滚：代码回滚 = revert tag/commit；迁移只追加、可空，回滚不删列；镜像/客户端回滚用上一 tag 产物。
- 客户端兼容：rc 与 0.1.0 数据兼容（workspace/session/settings schema 已 v2/v4，只加不破）。

## 6. 测试计划

| 层 | 用例 | 命令 |
| --- | --- | --- |
| T0 组合 | 基线/闭包/生成物 | `zk validate && zk compose --check && zk capability --check && zk docs --check && zk db lock --check` |
| T1 单包 | 新增工具/契约/配置单测 + conformance | `cargo test -p <pkg>`（本机无 nextest，用 cargo test） |
| T2/T3 | e2e/真 PG | `bash scripts/{m1,agent,login,admin,...}-e2e.sh`、`bash e2e/serve-schedule.sh`、`bash scripts/srv-auth-test.sh` |
| T4/T5 | 矩阵/交付物 | `pnpm -r typecheck/lint/test:unit/build`、`bash scripts/build-dist.sh`、`zk channel verify`、`bash scripts/security-audit.sh` |
| 组合裁剪 | personal plan 闭包/生成物/产物 | `zk plan --edition personal ...` + 生成物断言测试 + 发布产物检查 |
| CI | 远端实跑 | push 分支 → `gh run list/watch`；tag → release workflow |

## 7. 文档更新

- [ ] SSOT 文档：`docs/18`（门禁真源）、`docs/21`（13 项）、`docs/37`（发布清单）、`docs/12`（状态块）、`docs/20`（脚本真源）、`docs/38`（gap 收敛）、`docs/40-known-limitations.md`
- [ ] ADR/D：新增 ADR（rig 能力边界/agent-team 模型/personal target 裁剪，若涉及）；`docs/13` 更新 ADR-059 状态与 T22 收尾
- [ ] `docs/README.md` 主题登记：是（known limitations）
- [ ] 迁移/发布说明：`CHANGELOG.md` 1.0.0-rc.1、`KNOWN-LIMITATIONS.md`

## 8. 风险

| 风险 | 概率 | 影响 | 缓解 | 验证 |
| --- | --- | --- | --- | --- |
| 磁盘仅余 ~13G，构建/冷构建失败 | 高 | 高 | 先 `df` 预算；CI 承接重构建；本地只增量 | 构建日志 |
| rig 0.45 不暴露所需 API | 中 | 高 | 先 spike；无 API 则实现可验证子集 + known limitations，回问用户 | spike 报告 |
| 推送公开仓后 CI 首跑失败 | 中 | 中 | 分支先跑；按失败修；不直接推 main | `gh run watch` |
| 发布 workflow 首次运行失败（runner 缺依赖） | 中 | 高 | 本地 YAML/步骤审；CI 迭代修复；保留 UNVERIFIED | 运行日志 |
| 功能补齐引入回归 | 中 | 中 | 每任务单测 + e2e；全量门禁 | 438+ tests |
| 写入冲突 | 中 | 低 | 互斥写作用域 + Lead 最终 review | git diff |
