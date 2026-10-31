# 2026-10-31-release-1-0-readiness · 任务清单

> 每个任务必须能独立提交、独立验证；没有 `verify` 的任务不算任务。
> 阶段 A = 发布工程（先恢复"可发布"），阶段 B = 功能缺口（D2），阶段 C = 发布执行（push/tag/release）。
> 当前唯一可写目标：`1.0.0-rc.1`。

## 1. 任务表

| # | 任务 | 阶段 | 写作用域（文件/目录） | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- | --- |
| A1 | A | 修 rustdoc 私有链接错误 | `packages/server/jobs/src/lib.rs` | — | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | ☐ |
| A2 | A | ci.yml 补 rust/TS/a11y/e2e 门禁并删除 false claim | `.github/workflows/ci.yml` | A1 | 本地逐命令复跑 + YAML 审阅 | ☐ |
| A3 | A | security.yml：geiger 真跑、a11y 真跑、strict 模式 | `.github/workflows/security.yml` | A6 | YAML 审阅 + 本地 strict 复跑 | ☐ |
| A4 | A | release.yml：tag/version 校验、GH Release、真实产物、SBOM/known-limitations 入 dist | `.github/workflows/release.yml` | A10,A17 | YAML 审阅 + `workflow_dispatch` 逻辑走查 | ☐ |
| A5 | A | docs/18 + docs/03 门禁真源同步（CI 执行 vs UNVERIFIED；AGENTS §6 真值由 ci-release 提供给 docs-version） | `docs/18-strictness.md`、`docs/03-composition-and-ci.md` | A2,A3 | `python3 scripts/docs-check.py` | ☐ |
| A6 | A | security-audit.sh 13 项可执行化（逐项 rule/status/命令） | `scripts/security-audit.sh`、`docs/21-security-privacy.md` | — | `bash scripts/security-audit.sh`（exit 0/2 语义）、`ZEOKEEL_SECURITY_STRICT=1` | ☐ |
| A7 | A | 威胁模型独立评审记录 | `docs/reviews/2026-10-31-threat-model.md` | A6 | 评审者非实现者 + `scripts/review-check.py` | ☐ |
| A8 | A | M6 脚本补齐：backup/restore（master.key+data）、upgrade drill、perf 真实值、reproducible 覆盖、fault injection | `scripts/{backup-restore,upgrade-drill,perf-budget,reproducible-build,fault-injection}.sh` | — | 各脚本本机运行 + JSON 证据 | ☐ |
| A9 | A | M6 acceptance 与证据对齐 | `specs/changes/2026-10-30-hardening-release/acceptance.md` | A8 | `python3 scripts/spec-check.py` + 证据存在性 | ☐ |
| A10 | A | 版本统一 1.0.0-rc.1（Cargo/package/tauri） | `Cargo.toml`、`Cargo.lock`、`package.json`、`apps/*/web/package.json`、`packages/*/*/web/package.json`（仅需者）、`apps/desktop/tauri.conf.json` | — | `cargo check`/`pnpm typecheck`/版本一致性脚本 | ☐ |
| A11 | A | CHANGELOG 1.0.0-rc.1（只写已验证事实 + UNVERIFIED） | `CHANGELOG.md` | A10 | `python3 scripts/docs-check.py` | ☐ |
| A12 | A | README/AGENTS/docs/12 当前状态与事实一致 | `README.md`、`AGENTS.md`、`docs/12-roadmap.md` | A10 | `docs-check` + 人工 diff | ☐ |
| A13 | A | known limitations + LICENSE + docs/37 清单 + docs/README 登记 | `KNOWN-LIMITATIONS.md`、`LICENSE`、`docs/37-release-1.0-checklist.md`、`docs/README.md`、`docs/40-known-limitations.md` | A10 | `docs-check` | ☐ |
| A14 | A | docs/20 + docs/10 死链脚本/能力真源修复（含 AGENTS §12 陈旧待做由 docs-version 同步） | `docs/20-operations.md`、`docs/10-repo-and-build.md` | — | `docs-check` + `ls scripts/` 对照 | ☐ |
| A15 | A | 删除 2 处 `as T`（admin 响应运行时校验） | `packages/admin/ui-kit/web/src/`（+tests） | — | `pnpm -r typecheck/lint/test:unit` + `rg " as "` | ☐ |
| A16 | A | build-dist/deploy 占位治理（拒绝假包、真实 dist 契约） | `scripts/build-dist.sh`、`deploy/updates/` | A4 | `bash scripts/build-dist.sh --help` + 负向测试 | ☐ |
| A18 | A | R8.2 personal 裁剪：`zk compose` 按 `targets` 过滤服务端 glue + plan/生成物/产物断言测试 | `tools/zk/src/main.rs`、`tools/zk/templates/`、`crates/zk-composed`（生成）、测试 | A1 | `cargo test -p zk` + `zk compose --check` + personal 生成物断言 | ☐ |
| A17 | A | 本地 RC 证据：Linux 安装包（可行时）/SBOM/checksums，否则 UNVERIFIED 说明 | `target/evidence/release-1.0-rc1/`（不入库）、`scripts/sbom.sh` | A10,A16 | `zk sbom` + `sha256sum` + 构建日志 | ☐ |
| B1 | B | terminal 多会话 | `packages/client/tool-terminal/` | A1 | `cargo test -p zk-tool-terminal` + 新 e2e 断言 | ☐ |
| B2 | B | pwsh-persistent 实现（平台诚实不可用） | `packages/client/tool-pwsh-persistent/` | A1 | `cargo test -p zk-tool-pwsh-persistent` | ☐ |
| B3 | B | goal round driver | `packages/client/tool-goal/`、必要时 `agent-rig` | A1 | 单测 + agent e2e | ☐ |
| B4 | B | ralph fresh-context 隔离 | `packages/client/tool-ralph/`、`tool-subagent-fork` | A1 | 单测（上下文隔离断言） | ☐ |
| B5 | B | agent-team 从 experimental 到 implemented（或 ADR 不做，需用户确认） | `packages/client/tool-agent-team/`、`docs/13` | A1 | 单测 + 任务编排 e2e | ☐ |
| B6 | B | rig 0.45 能力 spike：run-state/Recorder/observe/telemetry/effect/ModelCatalog 压缩 | 只读（报告 `target/evidence/...`） | A1 | spike 报告 + API 证据 | ☐ |
| B7 | B | AgentEvent 独立 usage 事件 | `zk-contract-agent`、`agent-rig`、desktop | B6 | 契约 conformance + 单测 | ☐ |
| B8 | B | SessionStore::messages_in 回放 | `zk-contract-session`、`store-session-sqlite`、desktop | A1 | 单测 + 重启回放 e2e | ☐ |
| B9 | B | rig run-state/telemetry/effect 按实际 API 接入子集 | `agent-rig`、`crates/zk-kit-*`（如需新 kit，先 ADR） | B6,B7 | 单测 + hook 事件断言 | ☐ |
| B10 | B | ModelCatalog 上下文窗口/压缩 | `agent-rig` | B6 | 单测（窗口/压缩行为） | ☐ |
| B11 | B | hooks/memory/web-search 进 AppSettings + 设置 UI（env 仅 fallback） | `zk-contract-config`、`ui-settings`、生成 glue 输入 | A10 | schema-check + 单测 + e2e 设置保存/重启 | ☐ |
| B12 | B | a11y 矩阵：client/admin/portal | `e2e/`、必要时 `packages/*/web/src/` | A2 | `pnpm e2e` + axe 0 serious/critical | ☐ |
| B13 | B | i18n zh/en 完整性 | `packages/ui/*/web/src/`、`apps/desktop/web/src/` | — | 单测 + `rg` 硬编码检查 | ☐ |
| B14 | B | 真实 provider/媒体在线 + 500 用户压测 harness（env-gated） | `scripts/`、`e2e/` | B6 | 有 key 时 pass；无 key 时 UNVERIFIED 报告 | ☐ |
| B15 | B | 矩阵/docs/38 收敛更新（status 与实际一致） | `docs/generated/AGENT-TOOL-MATRIX.md`（生成器输入）、`docs/38-agent-runtime.md` | B1–B13 | `python3 scripts/agent-tool-matrix.py --check` | ☐ |
| C1 | C | 全量门禁 + 独立评审 + archive spec | `specs/archive/2026-10-31-release-1-0-readiness/`、`docs/reviews/` | A*,B* | 全部 verify 绿 + `review-check` | ☐ |
| C2 | C | push 分支触发 CI，修复 runner 差异 | 远端 `release/1.0.0-rc.1` | C1 | `gh run watch` 全绿 | ☐ |
| C3 | C | tag `v1.0.0-rc.1` 触发 release，产出真实 dist 并验证 | 远端 tag/Release | C2 | `gh release view` + `zk channel verify` | ☐ |

## 2. 写作用域

- 本变更只写：上表列出的文件/目录；其余（`target/` 证据除外）不写。
- 明确不写：`@generated` 生成物手改；历史 migration；`docs/14`、`docs/17`；参考仓库。
- 并行任务是否重叠：A2/A3/A4 同属 `.github/workflows/` → 由同一 owner 串行；A10–A14 同属版本/文档 → 同一 owner 串行；A8/A16 同属 `scripts/` 不同文件 → 可并行但需不同 owner；B1–B5 各包独立；B7–B10 可能同写 `agent-rig` → 串行或同 owner。

## 3. 提交顺序

1. A1 rustdoc（最小红门禁）。
2. A6 安全脚本 + A8 M6 脚本（独立证据）。
3. A10 版本（后续文档/CI/产物都引用同一版本）→ A2/A3/A5 CI 与门禁真源（A4 依赖 A10/A17，排在其后）。
4. A11–A14 文档真源（A11/A12/A13/A14 依赖 A10）。
5. A15 严格性 → A16 交付契约 → A18 personal 裁剪 → A17 本地 RC 证据。
6. B6 spike（已产出结论）→ B7→B8→B9→B10a→B4（`agent-rig` 串行）；B1+B2（同 owner）、B11、B12、B13 并行；B5 在 ADR 冻结后实施；B14；B15 最后。
7. C1 全量验收/评审 → C2 push CI → C3 tag/release。

## 4. 验收前检查

- [ ] `python3 scripts/spec-check.py`
- [ ] `python3 scripts/docs-check.py`
- [ ] 变更涉及的 T0/T1 命令全绿（见 design §6）
- [ ] 无手改 `@generated`
- [ ] 复用检查已做（`zk find`/能力索引；新增能力已登记 `@capability`）
- [ ] PR 只包含本 spec 范围
- [ ] `acceptance.md` 已填证据路径
- [ ] 发布说明列出全部 UNVERIFIED（不得 false-green）
