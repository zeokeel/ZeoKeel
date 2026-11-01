# 2026-11-01-client-mcp-config · 任务清单

> 每个任务必须能独立提交、独立验证；没有 `verify` 的任务不算任务。
> 状态：`draft`（M-1…M-6 已定（Lead 2026-11-01，见 proposal §7）；独立评审复评通过前不得改产品代码，C16/C19）。

## 0. 跨 spec 合并顺序与共享文件 owner（P1-1）

**全局串行顺序（不可并行跨阶段）**：

```text
2026-11-01-client-settings-ia
  → 2026-11-01-client-approval-tool-rendering
  → 2026-11-01-client-config-approval-mcp
  → 2026-11-01-client-mcp-config   （本 spec，最后阶段）
```

理由：四份 spec 共享 `packages/contracts/ipc`、`packages/client/agent-rig`、`packages/ui/ui-kit`、`apps/desktop/src/ipc.rs`、
`apps/desktop/web/src/i18n.tsx` 与 `docs/`；同一文件在任一阶段**只允许一个 spec 在写**，后序 spec 必须在前序 spec
合并并跑完门禁后再改该文件。本 spec 是最后阶段，对下列共享文件承担**最终合并 owner**：

| 共享文件/目录 | 本 spec 的角色 |
| --- | --- |
| `packages/contracts/ipc/src/lib.rs`、`packages/contracts/ipc/Cargo.toml` | **最终 owner**（revision/connectors/mcp.test DTO 最终一致性 + `zk-contract-connector` 依赖 + `--check`） |
| `apps/desktop/src/ipc.rs` | **最终 owner**（revision/审批/connectors/mcp.test 装配无冲突） |
| `packages/ui/ui-kit/web/src/types.ts` | 追加 MCP 镜像类型；settings-ia/approval/config 已先合入 |
| `apps/desktop/web/src/i18n.tsx` | 追加 MCP 文案；最终文案一致性由本 spec 复核（i18n-check） |
| `editions/{personal,enterprise,personal-only}.toml` | **最终 owner**（新 personal-only 包 + 清单） |
| `docs/07`、`docs/16`、`docs/19`、`docs/24`、`docs/26`、`docs/38`、`KNOWN-LIMITATIONS.md` | **最终合并 owner**（各 spec 各自段落，最终 `docs-check`） |
| `e2e/*.spec.ts`、`e2e/serve-m1.sh`、`scripts/m1-e2e.sh` | **最终 owner**（新 spec 文件加入显式清单） |
| 生成物（`shared/contracts/**`、`apps/desktop/web/src/generated/**`、`docs/generated/**`） | **最终 owner**（最后一次生成 + `--check`） |

**前置依赖**：任务 8（UI pane）依赖 settings-ia 的 `ui.settings.pane` 与 `SettingsPaneProps` 已合并；
任务 6 的 `settings.set` 改动依赖 settings-ia 的 revision 协议与父 spec 的审批规范化已合并；
任务 10 的 `/ipc` 命令依赖 settings-ia/父 spec 的 ipc 改动已合并。

## 1. 任务表

| # | 任务 | 写作用域（文件/目录） | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | **决策落文档（docs-first，C12/P1-2/M-6/P2-8）**：C1/C6 映射写 `docs/24` §1 + `docs/07` §6.1（M-6 已定：不新增 ADR）；契约依赖 `zk-contract-config → zk-contract-connector` 登记到 `docs/24` §1/`docs/04` §1.5；`docs/07` §6.1（来源优先级/enterprise catalog-only）、`docs/16`（新包/槽位）、`docs/26` §5（IPC 命令表补两条 mcp.test 命令，P2-N6）；`docs/24` §1/`docs/04` §1.5 同时登记 **`zk-contract-ipc → zk-contract-connector`**（P1-13.1）、`docs/38`（mcp.test 两段式）更新 | `docs/24-composition-and-runtime-spec.md`、`docs/04-contracts-and-api.md`、`docs/07-server-domains.md`、`docs/16-ui-packages.md`、`docs/26-workspace-and-conversations.md`、`docs/38-agent-runtime.md` | — | `python3 scripts/docs-check.py`、`python3 scripts/docs-command-check.py` | ☑ |
| 2 | **契约 connector**：`McpServerConfig::validate()`（现有 `validate_server` 规则**原样**搬移，含 url/loopback/secret-env 辅助）；`zk-connector-mcp::validate_server` 改委托；规则一致性测试 | `packages/contracts/connector/{src/lib.rs,tests/*}`、`packages/client/connector-mcp/src/lib.rs`（仅 validate 委托段） | 1 | `cargo test -p zk-contract-connector && cargo test -p zk-connector-mcp` | ☑ |
| 3 | **契约 config**：`McpSettings { servers }` + `AppSettings.mcp` + `McpSettings::validate`（上限/重复/id 字符集/`oauth_required` 拒绝）+ `Cargo.toml` 依赖 connector；conformance/schema 测试 | `packages/contracts/config/{src/lib.rs,Cargo.toml,tests/*}` | 1,2 | `cargo test -p zk-contract-config mcp` | ☑ |
| 4 | **connector-mcp 来源与探测（P1-8/P1-9）**：`LOCAL_SERVERS`/`ENV_FALLBACK`、`update_local_servers`/`clear_local_servers`/`set_env_fallback_enabled`；`configured_servers` 优先级（catalog 含空 > local 含空 > env[personal 且无 settings]）；抽取一次性 `connect` + `test_server`（无缓存、drop 回收）+ `session_count()` 测试访问器；优先级/无缓存/catalog 两级校验回归测试 | `packages/client/connector-mcp/{src/lib.rs,tests/*,README.md}` | 2,3 | `cargo test -p zk-connector-mcp`（`precedence`/`test_server_no_cache`/`catalog_limits`） | ☑ |
| 5 | **契约 ipc**：`MCP_TEST`/`MCP_TEST_STATUS` + `McpTestPayload`/`McpTestResult`/`McpTestStatusResult`（含 `McpTestState` 封闭枚举）；**`Cargo.toml` 新增 `zk-contract-connector` path 依赖**；schema 导出 | `packages/contracts/ipc/{src/lib.rs,tests/schema.rs,Cargo.toml}` | 2,3（settings-ia/父 spec 的 ipc 改动已合并） | `cargo check -p zk-contract-ipc && cargo test -p zk-contract-ipc && cargo test -p zk-contract-ipc export_schema` | ☑ |
| 6 | **desktop 装配**：新 `apps/desktop/src/mcp.rs`（`apply_settings_mcp`/boot/enterprise 清理/`test_server` 线程 + 状态表）；`ipc.rs`：`settings.set` 应用（personal）/拒绝（enterprise）、`mcp.test`+`mcp.test.status` dispatch、boot 调用；单测（boot/拒绝/超时/并发 id/无缓存） | `apps/desktop/src/{mcp.rs,ipc.rs,main.rs}` | 4,5 + settings-ia revision + 父 spec 审批规范化已合并 | `cargo test -p zk-desktop mcp` | ☑ |
| 7 | **新 UI 包四件套（脚手架）**：`packages/ui/ui-settings-mcp/**`（zk.toml/Cargo.toml/src/lib.rs/tests/conformance.rs/README.md/web/{package.json,tsconfig.json}）；contributions `ui.settings.pane` component `settings.mcp` order 70 | `packages/ui/ui-settings-mcp/**`（web/src 除外） | 1 + settings-ia `ui.settings.pane` 已合并 | `pnpm install && zk validate && cargo test -p zk-ui-settings-mcp` | ☑ |
| 8 | **MCP pane 组件**：列表/新增/编辑/删除/测试连接（`mcp.test`+轮询 `mcp.test.status`）/六态/a11y/i18n；props 用 `SettingsPaneProps`（无 edition 分支）；前端基本校验 + Rust 错误映射；单测 | `packages/ui/ui-settings-mcp/web/src/index.tsx`、`packages/ui/ui-settings-mcp/web/tests/*`、`packages/ui/ui-kit/web/src/types.ts`（MCP 镜像类型） | 6,7 | `pnpm --dir packages/ui/ui-settings-mcp test:unit && pnpm -r typecheck && bash scripts/i18n-check.sh` | ☑ |
| 9 | **组合与 personal-only 清单（P1-15）**：`editions/personal.toml` 映射、`editions/enterprise.toml` exclude、**`editions/personal-only.toml:3` packages 数据列表**加入新包；`zk validate` 断言 | `editions/{personal,enterprise,personal-only}.toml` | 7 | `zk validate && zk compose --edition personal --profile default --channel official --check` | ☑ |
| 10 | **生成物重生成 + 门禁**：`cargo test --workspace export_schema` + `python3 scripts/schema-ts.py` + `zk compose`；逐字节 `--check` | 生成物（`shared/contracts/**`、`apps/desktop/web/src/generated/**`、`docs/generated/**`）仅由生成器写 | 2,3,5,9 | `cargo test --workspace export_schema && git diff --exit-code && bash scripts/schema-check.sh && zk compose --edition personal --profile default --channel official --check && zk capability --check` | ☑ |
| 11 | **e2e（真实运行器）**：`e2e/mcp-config.spec.ts`（MCP pane 添加 fake stdio server→测试连接→保存→刷新保留）；`e2e/serve-m1.sh` 增加 fake stdio MCP 脚本与 seeded settings 变体；把新 spec 加入 `scripts/m1-e2e.sh` 第二段**显式清单**；`test.describe` 内 `test.skip(process.env["ZEOKEEL_E2E_SKIP_SERVE"] === "1", "mock stage")`（P2-N4：第一段 mock 自动 skip，第二段真实 serve 执行） | `e2e/mcp-config.spec.ts`、`e2e/serve-m1.sh`、`scripts/m1-e2e.sh` | 6,8,9 | `bash scripts/m1-e2e.sh --grep "MCP"` | ☑ |
| 12 | **文档回写 + CHANGELOG**：`docs/07`/`docs/16`/`docs/24`/`docs/04`/`docs/26`/`docs/38` 与实现对齐；`CHANGELOG.md`（`ZEOKEEL_MCP_SERVERS` 生效条件 + 企业 catalog-only）；`docs/19`/`KNOWN-LIMITATIONS` 相关段 | `docs/07-server-domains.md`、`docs/16-ui-packages.md`、`docs/24-composition-and-runtime-spec.md`、`docs/04-contracts-and-api.md`、`docs/26-workspace-and-conversations.md`、`docs/38-agent-runtime.md`、`docs/19-client-ux-flows.md`、`KNOWN-LIMITATIONS.md`、`CHANGELOG.md` | 9,11 | `python3 scripts/docs-check.py` | ☑ |
| 13 | **收尾**：`acceptance.md` 填证据路径（`target/evidence/2026-11-01-client-mcp-config/`）；全量门禁；回写 parity 报告 §7 状态 | 本 spec 目录、`research/client-dsh-parity-2026-11-01.md`（仅状态） | 1–12 | `python3 scripts/spec-check.py && python3 scripts/docs-check.py && python3 scripts/review-check.py` + §2 全绿 | ☑ |

## 2. 写作用域

- 本变更只写：
  - 契约/实现：`packages/contracts/{connector,config,ipc}/**`、`packages/client/connector-mcp/**`、
    `apps/desktop/src/{mcp.rs,ipc.rs,main.rs}`；
  - UI：`packages/ui/ui-settings-mcp/**`（新）、`packages/ui/ui-kit/web/src/types.ts`；
  - 组合：`editions/{personal,enterprise,personal-only}.toml`；
  - 锁文件：`pnpm-lock.yaml`（由 `pnpm install` 写，禁止手改）；
  - e2e：`e2e/mcp-config.spec.ts`、`e2e/serve-m1.sh`、`scripts/m1-e2e.sh`；
  - 文档：本 spec 列表（`docs/04`、`docs/07`、`docs/15`、`docs/16`、`docs/19`、`docs/24`、`docs/38`、
    `KNOWN-LIMITATIONS.md`、`CHANGELOG.md`）。
- 明确不写：
  - 生成物手改——任务 10 生成器写；
  - 审批策略/`connectors.*`（父 spec）、审批卡/工具行/`usage`（approval spec）、settings-ia 的 slot/revision；
  - `packages/contracts/agent`、`apps/desktop/src/{approval.rs,events.rs}`、`packages/ui/ui-chat/**`；
  - `apps/server/**`、admin SPA、`deploy/**`。
- 并行约束（§0）：本 spec 是最后阶段，必须在其余三份 spec 合并后开始；同文件同一时间只允许本 spec 在写。

## 3. 提交顺序

1. 任务 1（docs-first）→ 2（connector 契约）→ 3（config 契约）→ 5（ipc 契约）
2. 任务 4（connector-mcp）→ 6（desktop）
3. 任务 7（UI 包脚手架）→ 8（pane 组件）→ 9（editions）
4. 任务 10（生成物）→ 11（e2e）→ 12（文档）→ 13（收尾）
- 任务 2/3 可并行（不同 crate）；任务 4 依赖 2/3；任务 6 依赖 4/5；任务 7 依赖 settings-ia；任务 8 依赖 6/7；
  任务 9 依赖 7；任务 10 依赖 2/3/5/9；任务 11 依赖 6/8/9。

## 4. 验收前检查

- [x] `python3 scripts/spec-check.py`、`python3 scripts/docs-check.py`、`python3 scripts/review-check.py`
- [x] 变更涉及的 T0/T1 命令全绿（见任务表 verify 列）
- [x] 无手改 `@generated`（`export_schema`/`schema-ts.py`/`zk compose` 重生成，`--check` 绿）（`git diff` 只显示生成器输出）
- [x] 两级校验回归测试存在（catalog 旧合法仍可用；本地路径拒绝超上限）
- [x] `test_server` 无缓存断言（`SESSIONS` 数量不变、子进程回收）存在
- [x] `editions/personal-only.toml` 的 **packages 数据列表**已含新包（不是注释同步）
- [x] 跨 spec 顺序已遵守（§0），最终合并 owner 已复核 ipc/editions/docs/e2e 一致性
- [x] PR 只包含本 spec 范围
- [x] `acceptance.md` 已填证据路径；M-1…M-6 已定（proposal §7），独立评审复评通过后状态改为 `approved`（C16）
- [x] 契约 public-api diff（`cargo public-api diff`；本机未安装 → 记录 **UNVERIFIED**）与反序列化对拍样本已附（docs/04 §10，P2-N16）
