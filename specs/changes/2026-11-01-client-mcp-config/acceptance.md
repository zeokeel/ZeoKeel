# 2026-11-01-client-mcp-config · 验收与证据

- 验收人：待填（实现完成后由非实现者验收，C19）
- 日期：待填
- PR/commit：待填
- 评审记录：`docs/reviews/2026-11-01-client-ui-parity-specs.md`（CHANGES-REQUESTED；复评通过后在此追加结论与日期）

> 状态：`draft`（未开工；M-1…M-6 已定（Lead 2026-11-01，proposal §7）；复评通过前不改产品代码）。
> 证据根目录（预填，P2-9）：`target/evidence/2026-11-01-client-mcp-config/`
> （`logs/`、`screenshots/`、`axe.json`、`e2e-m1-mcp.log`、`schema-diff.txt`、`session-count.txt`）。
> 本文件只写验收口径，不代替可执行证据（`docs/11`）。

## 1. 验收命令与结果

| 命令 | 结果 | 证据路径 |
| --- | --- | --- |
| `python3 scripts/spec-check.py` | 待跑 | `target/evidence/2026-11-01-client-mcp-config/logs/spec-check.log` |
| `python3 scripts/docs-check.py` | 待跑 | `.../logs/docs-check.log` |
| `python3 scripts/review-check.py` | 待跑 | `.../logs/review-check.log` |
| `python3 scripts/docs-command-check.py` | 待跑 | `.../logs/docs-command-check.log` |
| `zk validate` | 待跑 | `.../logs/zk-validate.log` |
| `zk compose --edition personal --profile default --channel official --check` | 待跑 | `.../logs/compose-personal.log` |
| `zk compose --edition enterprise --profile default --channel official` + `cargo check --workspace` | 待跑 | `.../logs/compose-enterprise.log` |
| `zk capability --check` | 待跑 | `.../logs/capability-check.log` |
| `bash scripts/schema-check.sh` | 待跑 | `.../logs/schema-check.log` |
| `cargo test --workspace export_schema && git diff --exit-code` | 待跑 | `.../schema-diff.txt` |
| `cargo check -p zk-contract-ipc`（新增 `zk-contract-connector` 依赖编译通过，P1-13.1） | 待跑 | `.../logs/cargo-check-ipc.log` |
| `cargo check --workspace --all-targets`（契约依赖/全 workspace 编译） | 待跑 | `.../logs/cargo-check-workspace.log` |
| `cargo test -p zk-contract-connector -p zk-contract-config -p zk-contract-ipc` | 待跑 | `.../logs/cargo-contracts.log` |
| `cargo public-api diff`（契约 API 破坏检查；**本机未安装 `cargo-public-api` → 记 UNVERIFIED**，CI/有该工具时必跑） | UNVERIFIED（本机） | `.../logs/public-api-diff.log` |
| 契约反序列化对拍样本（旧 JSON → `AppSettings.mcp`/`McpTest` DTO 的 conformance golden，docs/04 §10） | 待跑 | `.../contract-golden.json` |
| `cargo test -p zk-connector-mcp`（`precedence`/`test_server_no_cache`/`catalog_limits`） | 待跑 | `.../logs/cargo-connector-mcp.log` + `session-count.txt` |
| `cargo test -p zk-desktop mcp` | 待跑 | `.../logs/cargo-desktop-mcp.log` |
| `cargo test -p zk-ui-settings-mcp`（四件套 conformance） | 待跑 | `.../logs/cargo-ui-pkg.log` |
| `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build` | 待跑 | `.../logs/pnpm.log` |
| `pnpm --dir packages/ui/ui-settings-mcp test:unit` | 待跑 | `.../logs/pnpm-ui-mcp.log` |
| `bash scripts/i18n-check.sh` | 待跑 | `.../logs/i18n-check.log` |
| `bash scripts/m1-e2e.sh --grep "MCP"`（personal `serve` + fake stdio MCP server；spec 的 `test.describe` 内 `test.skip(ZEOKEEL_E2E_SKIP_SERVE==="1", "mock stage")`，第一段 mock 跳过、第二段 serve 执行） | 待跑 | `.../e2e-m1-mcp.log` + 截图 + `axe.json` |

## 2. 手工检查

- [ ] **两级校验（P1-8）**：构造「catalog 旧规则合法但超过本地上限（如 args 40 条）」的 server → catalog 路径接受、`mcp_call` 可用；
      同配置走本地 settings → 保存被拒（`mcp/validation`）。
- [ ] **优先级/edition（T4）**：personal 无 settings 文件 + env → 生效；保存 settings 后 settings 权威（空列表也不回落 env）；
      enterprise 登录后 catalog 权威（空列表也不回落 env/local）；enterprise 未登录 → MCP 诚实不可用（无 env 回退）。
- [ ] **企业拒绝/清理**：enterprise `settings.set` 非空本地段被拒；boot 清理持久化本地段（设置文件回写为空）。
- [ ] **无缓存测试连接（P1-9）**：测试连接草稿 server 后，全局 MCP 会话数不变、无残留子进程；当前生效配置不变；
      超时（>15s）返回 `mcp/timeout`；`pending` 期间再次测试返回 `mcp/test-in-flight`（不覆盖在飞探测）。
- [ ] **MCP pane**：新增/编辑/删除/测试连接/保存可用；空/加载/错误/离线/流式中/已取消六态可见；键盘可达、Esc 关闭、
      焦点回位；axe 0 serious/critical；zh/en 无硬编码。
- [ ] **compat**：旧 settings（无 mcp）加载正常；旧前端不调新命令正常；旧 catalog 不受两级校验影响；回滚旧版本不崩。
- [ ] **组合**：enterprise 闭包不含 `zeokeel/ui-settings-mcp`；personal `SettingsPanes` 含 `settings.mcp` order 70；
      `editions/personal-only.toml` 的 `packages` 数据列表含新包。
- [ ] **文档**：`docs/24` §1 / `docs/04` §1.5 依赖登记可查；`docs/15` D 编号（或 ADR）存在；`docs/07`/`docs/16`/`docs/38` 与实现一致。

## 3. 文档与决策

- [ ] SSOT 文档已更新：`docs/07` §6.1、`docs/16` §2、`docs/19`（MCP 设置段）、`docs/24` §1（**config→connector 与 ipc→connector 两条依赖**）、`docs/04` §1.5、`docs/26` §5（两条 mcp.test 命令）、`docs/38` §4
- [ ] 决策已记录：M-1…M-6 已定（Lead 2026-11-01；`docs/15` §C D 编号 + `docs/24` §1 / `docs/04` §1.5 + `docs/07`/`docs/16`/`docs/38`；**M-6 已定不新增 ADR**）
- [ ] `docs/README.md` 已登记新主题：不适用
- [ ] `CHANGELOG.md`：`ZEOKEEL_MCP_SERVERS` 生效条件、企业 catalog-only、MCP 本地配置新增说明
- [ ] `docs/reviews/2026-11-01-client-ui-parity-specs.md` 已追加复评结论
- [ ] 变更 spec 已归档到 `specs/archive/`（或在 `specs/changes/README` 状态表更新）

## 4. 回滚

- 回滚步骤：回退 commit（无 server/DB 迁移、无镜像变更）；回退 `editions` 三处映射/exclude/清单。
- 数据影响：设置 JSON additive；旧版忽略 `mcp` 字段；catalog 行为未变（两级校验）。
- 已验证回滚：待填（一次「新版写设置 → 旧版启动」兼容验证 + 一次 env 用户升级前后对比）。

## 5. 剩余风险 / 后续

| 项 | 影响 | 后续 spec |
| --- | --- | --- |
| 本地 MCP 不支持 OAuth（无 authorize/token 元数据） | 中 | Phase 2（先设计本地 OAuth 元数据） |
| 无 per-server revision/原子写（M-1 已定 AppSettings） | 低 | 若出现独立 store 需求，另立 spec（M-1 备选 B） |
| `mcp.test` 状态表为进程内、重启丢失 | 低 | 无需（探测一次性） |
| 远端连接器生命周期（守护/健康检查/重连） | 中 | KNOWN-LIMITATIONS 既有残余 |
| 企业 env 关闭对既有用户的行为变化 | 中 | T4 已定；发布说明 + 诚实不可用 |
| `ui-settings-mcp` 包与 settings-ia 的 pane 排序细节 | 低 | settings-ia（order 表）与本 spec 任务 9 联调 |
| `cargo public-api diff` 工具本机未安装（docs/18 §7.2 UNVERIFIED）；跨契约依赖（config/ipc → connector）的 API 破坏检查依赖 CI/后续接线 | 中 | 实现阶段 CI 门禁 |

## 6. 结论

- [ ] `done`：证据齐全，可合并
- [ ] `abandoned`：原因：待填

## 7. 复用证据（C17）

- 搜索记录：`./target/debug/zk find mcp` → **0 命中**（不新增 capability id）；`find connector` → `contract.connector`；
  `find settings` → `store.settings.json`；`find config` → `contract.config`。
- 复用清单：
  - `zk-contract-connector::McpServerConfig`（`packages/contracts/connector/src/lib.rs:170-199`）→ 唯一 DTO；
  - `zk_connector_mcp::validate_server`（`packages/client/connector-mcp/src/lib.rs:102-148`）→ 提升到契约后委托；
  - `connector-mcp` 的 `servers_from_catalog`/`update_from_catalog`/`clear_catalog_config`/`reload_connectors`（`:186-294`）→ 复用；
  - `list_server_tools`（`:442-451`）→ 不复用于探测（缓存污染），仅 UI 展示语义参考；
  - settings 管线与 `masked_settings`、enterprise 清理先例（`apps/desktop/src/ipc.rs:269-295`）、oauth 两段式先例（`:1282-1330` / `:1340-1364`）；
  - `ui-settings-provider` personal-only 包先例（结构复制）；settings-ia 的 `ui.settings.pane`/`SettingsPaneProps`。
- 新增能力登记：`McpServerConfig::validate`（`contract.connector` 既有条目）、`McpSettings::validate`（`contract.config` 既有条目）；
  未新增 capability id、未新增 Rust 业务包（仅新增 personal-only UI 包）。
