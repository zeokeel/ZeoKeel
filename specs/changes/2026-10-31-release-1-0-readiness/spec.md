# 2026-10-31-release-1-0-readiness · 行为规格

> 只写"系统必须怎样"，不写实现选型；实现见 `design.md`。
> 发布目标：`1.0.0-rc.1`（用户 2026-10-31T13:08 拍板：D1=rc.1、D2=补齐全部功能缺口、D3=授权推送、D4=环境项 CI 路径+UNVERIFIED）。
> 行为决策已于本 spec 冻结（§6），实现不得再留未决行为。

## 1. 需求

### R1 · 门禁全绿（本地可复现）

- **WHEN** 在干净工作树执行 `AGENTS.md` §6 的"构建/测试"命令
- **THEN** 全部 exit 0，包括当前红的 `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`
- **AND** `cargo test --workspace export_schema` 后 `git diff --exit-code` 为 0，工作树不含未提交生成物差异

### R2 · CI 是门禁真源，不得 false-green

- **WHEN** `AGENTS.md` §6、`docs/03`、`docs/18` 声明某命令为"当前门禁"
- **THEN** `ci.yml`/`security.yml`/`release.yml` 至少有一个 workflow 实际执行该命令；否则该命令必须从"当前门禁"清单移除并标 UNVERIFIED（含文件:行、原因、替代验证）
- **AND** `pnpm -r typecheck/lint/test:unit`、`cargo doc -D warnings`、a11y 矩阵、apps/updates/oem/client-upgrade e2e 在 CI 有真实执行步骤
- **AND** `cargo geiger`/`cargo nextest`/`cargo public-api` 不得"安装但从不调用"；不能执行则删除声明并标 UNVERIFIED

### R3 · 发布流水线可产出且可验证真实交付物

- **WHEN** 推送 tag `v<version>`（必须等于 workspace/tauri/package 版本）或手动 `workflow_dispatch`
- **THEN** release workflow（tag push 触发、`permissions: contents: write`）产出三平台客户端安装包、服务端镜像归档、`dist/`（含 `SHA256SUMS`、`CLIENT-RELEASE.json`、SBOM、KNOWN-LIMITATIONS）并创建 GitHub Release
- **AND** `scripts/build-dist.sh` 拒绝占位/伪造输入（exit 2），`zk channel verify --dist <真实dist>` 通过
- **AND** release 说明明确列出 UNVERIFIED 环境项；Windows 未签名、macOS 公证状态按实际记录（不得 `|| true` 吞掉失败）

### R4 · 安全门禁 13 项可执行

- **WHEN** 执行 `bash scripts/security-audit.sh`
- **THEN** `docs/21` §10 逐项列出 13 项门禁（从 `specs/changes/2026-10-30-hardening-release/spec.md` 迁移枚举，SSOT 在 docs/21），每项输出 `pass/fail/unverified`（含 rule/severity/owner/命令/证据）
- **AND** 本机可验证项 `pass`，外部工具/环境项 `unverified` 且整体状态不得为 `pass`
- **AND** 威胁模型（`docs/33`）有独立评审记录（`docs/reviews/2026-10-31-threat-model.md`）；认证/CSRF/SSRF/zip/WASM/命令/密钥/审计链/隐私用例有可执行测试或明确 UNVERIFIED
- **AND** `ZEOKEEL_SECURITY_STRICT=1` 时 unverified 视为 fail（CI 用）

### R5 · M6 演练证据

- **WHEN** 执行 M6 脚本
- **THEN** backup→restore 覆盖 master.key 与 `/data` 回灌、升级/回滚 drill 在 Docker 环境可执行（本机无 Docker 时标 UNVERIFIED）、perf 预算输出真实数值（不得 null 仍 pass）、可复现构建覆盖 `zk`/`zk-server`/`zk-desktop`/admin dist（或明确 UNVERIFIED 原因）、故障注入覆盖项与断言齐全
- **AND** `specs/changes/2026-10-30-hardening-release/acceptance.md` 状态与证据一致（done 或 blocked，不得 false pass）

### R6 · 版本与发行文档一致

- **WHEN** 查看 `Cargo.toml`、根/应用 `package.json`、`apps/desktop/tauri.conf.json`、`CHANGELOG.md`、`docs/37`
- **THEN** 产品版本统一为 `1.0.0-rc.1`，CHANGELOG 有 `1.0.0-rc.1` 条目且只写已验证事实，`docs/37` 每项标注 已勾/UNVERIFIED 及证据指针
- **AND** 新增 `KNOWN-LIMITATIONS.md`（SSOT；`docs/40-known-limitations.md` 只做摘要链接或在 `docs/README` 登记后作为唯一正文，二者不得双份规范）
- **AND** 根 `LICENSE` 存在（MIT 全文）

### R7 · 入口文档与事实一致

- **WHEN** 阅读 `README.md`、`AGENTS.md`（§1 当前阶段、§12 已知未决）、`docs/10`、`docs/12`、`docs/20`
- **THEN** 描述与当前 HEAD 实际状态一致（已实现项不得再写"未接/M0 骨架/待做"）
- **AND** `docs/10`/`docs/20` 引用的脚本全部存在；不存在的能力明确标 UNVERIFIED/不适用
- **AND** `python3 scripts/docs-check.py` 0 errors / 0 warnings

### R8 · 严格性与组合债务归零（无豁免）

- **WHEN** 检查 `packages/admin/ui-kit/web/src/index.ts` 等边界
- **THEN** 无 `as`/`any` 类型断言（含 `eslint-disable` 压制）；外部 JSON 走运行时校验（Ajv/契约 schema），越界值拒绝并记日志
- **AND** **personal 组合的裁剪可验证**：`zk plan --edition personal --profile default --channel official` 闭包不含任何 `srv-*`/企业包；`zk compose --edition personal` 生成的 `apps/server/src/generated/packages.rs` 为空（不含服务端域包）；个人版发布产物不含服务端二进制/镜像。**不接受 ADR 豁免或"文档声称已裁剪、实现未裁剪"。**

### R9 · 功能缺口补齐（用户 D2 = 全部可行项）

按 `docs/generated/AGENT-TOOL-MATRIX.md` 与 `docs/38` §5.4 的**明确缺口**逐项收敛；by-design 非目标见 proposal §3。

- **R9.1 工具**：`terminal` 每会话多会话；`pwsh-persistent` 从 unsupported 变为 implemented（平台不支持时 `UNSUPPORTED_PLATFORM` 诚实不可用）；`goal` 有 round driver；`ralph` 有 fresh-context 隔离；`agent-team` 从 experimental 变为 implemented（模型见 §6）。
- **R9.2 rig**：`AgentEvent` 独立 usage 事件；`SessionStore::messages_in` 回放；按 rig 0.45 公开 API 接入可验证子集——`serve::Recorder`/`RecordBuffer` → JSONL effect 记录（`AgentBuilder::record_to`）、`observe`/`telemetry`/`effect` 结构化事件、`AgentRun` 序列化/`Agent::resume` 用于可恢复的 run state；`catalog::Catalog` + `ModelSpec.context_window` 的上下文窗口/预算与 compaction（§6）。**已知不可实现项**：rig-agent 无公开的 live high-level runner 中途 checkpoint（`RunOrigin`/`build_run` 为 `pub(crate)`，源码证据在 `target/evidence/release-1.0-rc1/spikes/rig-0.45.md`）→ 写入 known limitations，最终验收前向用户确认口径；不得伪造实现。
- **R9.3 配置**：`docs/38` 记录为 env-only fallback 的 agent hooks/memory/web-search 等进入 `AppSettings` + 设置 UI（env 仅 fallback）；enterprise 按 C7 不适用的项在 UI 隐藏并拒绝。
- **R9.4 可访问性/i18n**：client/admin/portal 关键页面 axe 矩阵（0 serious/critical）、Tab/Esc/⌘K/focus/reduced-motion、zh/en 文案完整（portal 至少 zh+en）。
- **R9.5 真实环境**：真实 provider/媒体/embedding 在线调用与 500 用户压测提供可执行 harness（需要 key/环境的项标 UNVERIFIED，禁止 mock 冒充真实）。

## 2. 边界与错误

| 场景 | 期望行为 | 错误码/提示 |
| --- | --- | --- |
| 缺 Docker/证书/真实 key/真机 | 脚本/CI 明确 `UNVERIFIED`、整体 not pass；文档记录原因与后续命令 | `status:"unverified"`，exit 2（strict 下 exit 1） |
| 伪造成交物（占位包/假 SHA） | `build-dist.sh`/`verify` 拒绝 | exit 1/2 + 明确错误行 |
| tag 与版本不一致 | release workflow 失败 | `version mismatch` |
| 外部 JSON 形状非法 | 拒绝并记日志，不崩溃 | 契约错误码 / `CAPABILITY_UNAVAILABLE` |
| personal 组合含服务端包 | `zk validate`/测试失败 | 裁剪断言失败 |
| 平台不支持（pwsh on Linux） | 工具诚实不可用 + 原因 | `UNSUPPORTED_PLATFORM` |

## 3. 权限与可见性

- 角色：超管/审计员/员工能力不变；本变更不新增权限点、不改弱管控边界（C6）。
- 企业版 AI 仍只走网关（C7）；媒体/embeddings 在企业版保持不可用并在 UI 隐藏。
- 内容可见性不变（C8）。

## 4. 数据与兼容

- 新增/修改表/字段：R9.2 的 run-state/effect 记录为客户端本地 JSONL/JSON（不入库或仅 session 附加列 `run_state_json`，`#[serde(default)]`）；R9.3 settings 新 section additive；迁移只追加。
- 兼容：客户端 `min_supported` 保持同版本；不破坏既有 IPC/配置；破坏性变更需 v2/ADR。
- 幂等：沿用现有 idempotency/结算语义，不新增无幂等写路径。

## 5. 验收断言（可执行）

- [ ] `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets --all-features -- -D warnings`、`cargo test --workspace`、`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`、`cargo test --workspace export_schema && git diff --exit-code` 全 0。
- [ ] `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build` 全 0。
- [ ] `zk validate && zk compose --edition personal --profile default --channel official --check && zk capability --check && zk docs --check && zk db lock --check && zk lint --duplicates && zk sbom --edition personal --profile default --channel official --check` 全 0。
- [ ] `python3 scripts/docs-check.py && python3 scripts/spec-check.py && python3 scripts/review-check.py && bash scripts/schema-check.sh` 全 0。
- [ ] 10 套 e2e（m1/schedule/agent/login/admin/client-upgrade/apps/updates/oem/srv-auth）全 0；R9 各项有对应测试（名称 + 命令写入 acceptance）。
- [ ] `bash scripts/security-audit.sh` 13 项逐项状态；本机可验证项 pass、外部项 unverified；`ZEOKEEL_SECURITY_STRICT=1` 为 fail。
- [ ] release workflow 在远端实跑（tag `v1.0.0-rc.1`）产出真实三平台包/镜像/SBOM/SHA256SUMS，`zk channel verify` 通过并生成 GitHub Release。
- [ ] `docs-check` 0 warning；README/AGENTS/CHANGELOG/docs/37/known limitations/LICENSE 与事实一致。
- [ ] R8.2 personal 裁剪断言测试通过（plan 闭包 + 生成物 + 发布产物）。
- [ ] `docs/generated/AGENT-TOOL-MATRIX.md` 中明确 gap 全部 implemented 或 §6 已列 by-design/不可实现项（含证据），不得保留未说明的 partial/experimental。

## 6. 已冻结决策（实现以此为准）

| # | 决策 | 内容 |
| --- | --- | --- |
| F1 | B5 agent-team 模型 | 父 agent 作为 coordinator，经现有 `zk-kit-subagent` 注册表派发 N 个 worker（沿用 async runner、cap 8、depth 3、取消/超时）；结果聚合回父；不引入运行期插件（C2），不做动态团队编排 UI；新增 ADR 记录编排语义与上限。 |
| F2 | B8 历史权威 | `store-session-sqlite` 为唯一权威；rig-memory 仅作可选的模型侧缓存。历史回放由桌面把 SQLite messages 注入 prompt（`messages_in`），rig-memory 关闭时行为一致。 |
| F3 | B3 goal round 语义 | goal = 会话级目标 + 轮次计数器（持久化于会话 meta）；driver 每轮执行一次 agent turn，达到条件/取消/`max_rounds`（默认 10，clamp 1..100）停止；每轮记录 hook/usage。 |
| F4 | B10 上下文压缩 | 预算取 `catalog::Catalog` 的 `ModelSpec.context_window`（缺省 128k，可配）；超预算时先确定性截断最旧非系统消息，配置了 summarizer 模型时再用 rig-memory `plan_summary/apply_summary` 生成摘要；不得静默丢系统消息/工具结果对；压缩事件写 hook + 审计式日志。 |
| F5 | B6/rig 不可实现项 | 仅 `live runner 中途 checkpoint` 不可实现（`pub(crate)`，源码证据）；其余 R9.2 项必须实现。known limitations 记录 + 最终验收前用户确认。 |
| F6 | R8.2 personal 裁剪 | 由 `zk` 生成器按 `targets` 过滤：personal 不生成服务端域包 glue；发布流水线按 edition 只构建对应产物；测试断言 plan 闭包/生成物/产物三处。 |
| F7 | 版本文案 | 本轮统一 `1.0.0-rc.1`；`1.0.0` 正式版仅在 rc 验收/远端 CI+release 通过后由用户确认再打 tag。 |

## 7. 未决问题（仅外部环境，不影响实现）

- [ ] GitHub hosted runner 的 docker/notarize/多平台安装包首次实跑结果（D4：允许 CI 路径 + UNVERIFIED；实跑由 C2/C3 完成）。
- [ ] 真实 provider/媒体 key 与 500 用户压测环境（D4：harness 可执行 + UNVERIFIED）。
