# ZeoKeel 1.0.0-rc.1 已知限制（Known Limitations）

> **本文件是已知限制/UNVERIFIED 的唯一真源（SSOT）**；`docs/40-known-limitations.md` 只做摘要并链接本文件。
> 版本：`1.0.0-rc.1`（2026-10-31，本地验证）。`1.0.0` 正式版需 rc 验收 + 远端 CI/release 实跑通过后由用户确认，
> 不得据本文件声称 1.0.0 已发布。证据见 `target/evidence/release-check/`、`target/evidence/release-1.0-rc1/`、
> `docs/37-release-1.0-checklist.md`、`docs/12-roadmap.md`。

## 1. 版本与发布状态

| 项 | 状态 | 说明 |
| --- | --- | --- |
| 产品版本 | `1.0.0-rc.1` | `Cargo.toml` workspace、根 `package.json`、`apps/desktop/web`、`apps/admin/web`、`apps/desktop/tauri.conf.json` 一致 |
| 内部 workspace 包版本 | `0.1.0` | `packages/**/web/package.json` 为私有 `@zeokeel/*` 工作区包（不单独发布），版本不随产品版本；仅产品表面升到 rc |
| git tag / GitHub Release | 无 | 尚未创建 `v1.0.0-rc.1` tag，也无 Release/远端 CI 运行记录 |
| GitHub Actions（hosted runner） | **UNVERIFIED** | 远端从未运行；本地只验证了 workflow 语法/步骤（见 `.github/workflows/`、`target/evidence/release-1.0-rc1/ci/`） |
| 1.0.0 正式版 | 未发布 | 需 rc 验收 + 远端 CI/release 实跑通过后由用户确认 |

## 2. 交付物与环境项（UNVERIFIED）

本机无 Docker/证书/真机/真实 provider key 与网络，以下项没有本地产出或验证证据；**不得当作已验证**：

| 项 | 状态 | 原因/复现命令 |
| --- | --- | --- |
| Docker 镜像构建/导入/`compose up` | UNVERIFIED | 本机无 `docker`（`docker --version`），`deploy/` 只有源文件与 compose；`bash scripts/security-audit.sh` 同类环境项为 unverified |
| 服务端镜像归档 / 镜像内 SBOM | UNVERIFIED | 依赖 Docker 构建；`zk sbom` 只覆盖工作区，不证明镜像内容（见 §5） |
| 三平台客户端安装包（Linux/macOS/Windows） | UNVERIFIED | `apps/desktop/tauri.conf.json` `bundle.active=false`，本机无 Tauri CLI/打包依赖；本地未产出安装包 |
| macOS 公证 + staple | UNVERIFIED | 证书未配（ADR-037）；CI 步骤就绪后由发布环境执行 |
| Windows 签名 | 按设计不做 | Windows 安装包**不签名**（ADR-033）；发布说明必须声明 |
| PITR / WAL 归档备份 | UNVERIFIED | `scripts/backup-restore.sh` 只有 `pg_dump`；WAL/PITR 未配置 |
| 升级/回滚 full drill（Docker） | UNVERIFIED | `scripts/upgrade-drill.sh --docker` 需 Docker；本地仅非 Docker 部分 |
| 可复现构建（server/desktop/admin/镜像） | 本地 4 项 PASS；镜像 UNVERIFIED | `scripts/reproducible-build.sh`（M6 复跑）：`zk` / `zk-server` / `zk-desktop` / `admin-dist` 两次构建一致；镜像 tar 与严格 flag 集（`--remap-path-prefix`/`SOURCE_DATE_EPOCH`）需 Docker/CI 冷构建 → UNVERIFIED。证据 `target/evidence/m6/reproducible.json` |
| 性能预算 | 部分 UNVERIFIED | `scripts/perf-budget.sh` 的网关/镜像等字段为 null/unverified；本机基线见 `target/evidence/m6/perf.json` |
| 500 用户负载/压测 | **本地 smoke PASS（修复网关串行化后）；生产容量仍 UNVERIFIED** | harness `bash scripts/load-smoke.sh --local --users 500 --concurrency 50 --ramp-ms 5000`（2026-10-31 修复后）：0 错误，health p95 1.72ms，login p95 2.84s（local Argon2 stampede，仅参考），**gateway p95 46.5ms / p50 38.2ms / max 131ms（预算 5s），稳定吞吐 98.4 settled req/s**。根因：网关每请求重建 `reqwest::Client`（TLS/连接器初始化）；已改为进程级连接池客户端 + 每请求 provider timeout（`packages/server/gateway/src/lib.rs`）。修复前同 harness 约 11.5–12.3 req/s、gateway p95 4.86–6.17s（b12b14 证据保留）。本机为单机 PG + 假上游、`synchronous_commit=off`，非生产容量；真实部署需用 `ZEOKEEL_LOAD_BASE_URL`+`ZEOKEEL_LOAD_USERNAME/PASSWORD` 复测。证据 `target/evidence/release-1.0-rc1/phase-b/b14-perf/{load-smoke-500.json,load-smoke-500.log}` + 修复前 `.../b12b14/` |
| 真实 provider/媒体/embedding 在线调用 | UNVERIFIED（本机无 key/网络） | harness `bash scripts/real-provider-smoke.sh`（`--help` 列全部 env）：无 env 时明确 UNVERIFIED 且 exit 2；有 key 时真实调用 chat（`ZEOKEEL_REAL_PROVIDER_*`/`ZEOKEEL_OPENAI_*`）、embedding/image/audio/transcription/rerank（各自 `ZEOKEEL_*_BASE_URL/_API_KEY/_MODEL`）与 gateway upstream（`ZEOKEEL_REAL_RESPONSES_MODEL`），逐项 pass/fail/UNVERIFIED。证据 `target/evidence/release-1.0-rc1/phase-b/b12b14/real-provider-smoke.json`（当前 unverified）；企业版媒体/embeddings 按 C7 不提供 |
| SBOM 打包进交付 tar/镜像 | UNVERIFIED | `zk sbom` 已实现工作区级 CycloneDX；SBOM 进 tar/镜像由 release 流程处理，本机未验证（`docs/09` §8） |
| `zk channel verify` 对真实 dist | UNVERIFIED | 只在 dry-run/占位 dist 上验证过；真实 dist 未产出 |
| 三平台真机安装/升级冒烟 | UNVERIFIED | 无真机；本地仅 `client-upgrade-e2e.sh` 模拟链路 |

## 3. 功能残余与设计边界（B15 后最终口径）

### 3.1 Agent 运行时与 rig 接线（B15 后最终口径；详见 `docs/38` §5.4）

- **B7 已实现**：`AgentEvent::Usage{usage}` 为独立 additive 事件（terminal 事件前一次；契约 schema/TS 已由生成器更新）；
  usage 同时随 `TurnCompleted.reply` 携带并持久化到会话，重启加载后 UI 恢复展示。无残余。
- **B8 已实现**：桌面把 `SessionStore::messages_in` 作为多轮历史权威并注入 prompt（`run_stream_in_shared_with_history`
  + rig `AgentRunner::history`），有历史时不再同时使用 rig-memory 目录日志（避免双份上下文）。**残余**：tool-call
  历史不回放（纯文本行缺 tool-call id）；跨 workspace 隔离由 `messages_in` 校验承担。
- **B9 已实现（子集）**：可选 JSONL effect log（`packages/client/agent-rig/src/effect_log.rs`，rig `Recorder` +
  `RecordBuffer`，每条记录经 `rig_core::redact::json_text` 脱敏后追加；关闭时不落盘，写失败只降级日志）。**不可达
  （已确证）**：live high-level run checkpoint/resume——rig 0.45 `AgentRun`/`build_run` 为 `pub(crate)`，宿主拿不到
  可恢复 run state（证据 `target/evidence/release-1.0-rc1/spikes/rig-0.45.md`）；telemetry 仅 tracing spans（无自定义
  sink），`observe` 全量语义未接。
- **B10 已实现（窗口子集）**：`AgentRuntimeConfig.context.window_tokens`（env `ZEOKEEL_AGENT_CONTEXT_WINDOW_TOKENS`；
  缺省 128k 窗口 − 4k 输出预留）在每轮前预算；超预算时**确定性丢弃最旧 history**，仍超则 `CONTEXT_WINDOW_EXCEEDED`
  诚实报错。**残余（设计待定）**：rig-memory `CompactionPolicy`/summarizer 全量压缩未接（需 summarizer 模型/成本/
  阈值决策）。
- `pdf`/`epub` 文档 loader 未启用（ADR-067：licence/MSRV/lockfile 约束）；`read_document` 为独立实现（PDF/EPUB
  正反测试已有）。

### 3.2 工具能力（B1–B5 后最终口径）

无 `partial`/`unsupported`/`experimental` 行；完整状态以生成物 `docs/generated/AGENT-TOOL-MATRIX.md` 为准
（`scripts/agent-tool-matrix.py --check` 绿）。

| 工具 | 状态 | 说明 |
| --- | --- | --- |
| `terminal` | implemented | 每会话多会话（`open/status/send/close/list`；session id 校验、每会话上限 8） |
| `pwsh-persistent` | implemented（平台门控） | 真实长驻 pwsh 会话；非 Windows/缺 pwsh → `TOOL_UNSUPPORTED_PLATFORM`（诚实不可用） |
| `goal` | implemented | 会话级目标 + 有界 round driver（`set/status/run/cancel`；rounds/max_rounds 持久化，默认 10、clamp 1..100；重启安全） |
| `ralph` | implemented | 每轮 `run_fresh` 独立 child context（B4；不再共享 memory） |
| `agent-team` | implemented | coordinator + 有界 async worker 扇出（cap 8/depth 3；ADR-070；单 worker 失败→`partial`） |
| `tool-cordis` | disabled（设计） | C2 禁止运行期插件加载/检查，保持禁用 |

### 3.3 配置与能力边界

- agent hooks/memory 已进 `AppSettings.agent`（设置页“本地 Agent”），web-search 端点/超时已进 `AppSettings.web_search`（设置页“网络搜索”）；优先级 **显式设置 > env fallback > 内置默认**（env：`ZEOKEEL_AGENT_*`、`ZEOKEEL_SEARCH_URL`），保存即生效（web-search 下一次调用解析，无需重启）。**剩余**：`ZEOKEEL_AGENT_*` 仍是 fresh-install/默认值时的生效路径，UI 不展示 env fallback 的 effective 状态（与调度设置同口径）；web-search 超时无 env fallback。
- web-search 端点仅校验 https，不限制私网/自定义主机的出口策略（本地配置、P2 已知项；弱管控下由客户终端网络边界负责）。
- 企业版媒体/embeddings **按设计不可用**（C7/ADR-031）：UI 隐藏、`settings.set` 拒绝；客户端不得持有上游 key。web-search 是工具出口而非 AI provider，C7 不适用，两版本均可配置。
- `cap-vector` 仅本地 JSON 持久化：无跨进程写锁、无 SQLite/FTS、无跨设备同步（最后一写者胜），enterprise 无 embeddings 路由。

### 3.4 可访问性/i18n

- axe 矩阵（B12）已覆盖：client（`serve`：空态/聊天/设置/命令面板/检查器 + Tab/焦点回位/Esc/⌘K/Ctrl+B、
  reduced-motion，`e2e/m1-a11y.spec.ts`）、admin（登录页 + 10 个页面 + 键盘焦点，`e2e/m6-a11y.spec.ts`）、portal
  （runtime zh `/portal` + 静态 `deploy/portal/index.html` 双语，`e2e/m5-portal.spec.ts`/`m6-a11y.spec.ts`）；
  门禁 0 serious/critical。证据 `target/evidence/release-1.0-rc1/phase-b/b12b14/`。
- 残余：真实 Tauri WebView/三平台与主题矩阵未跑（M6 平台项）；zh/en 全字符串审计由 B13 `scripts/i18n-check.sh` 覆盖，
  本矩阵只做 `lang` 属性与页面级检查。

### 3.5 连接器（MCP）客户端残余（B16/R9.6）

- 客户端 MCP transport 已实现：stdio JSON-RPC（initialize/tools.list/tools.call）、HTTP(S) POST（rustls TLS）、
  catalog 配置映射/校验、每 connector 一次 initialize 并复用会话、配置变更/force-update 重启；测试
  `cargo test -p zk-connector-mcp`（16 tests：stdio/HTTP/TLS + OAuth token 注入）。
- catalog 配置 passthrough 已打通：服务端 `GET /api/client/v1/connectors` 现下发 sanitize 后的非敏感执行
  `config`（command/args/env allowlist/url/timeout_ms；`authorize_url`/`token_url` 仅在 `oauth` 元数据中，
  secret 键被剔除；`scripts/srv-store-test.sh` 断言）；desktop `auth.configure` 拉取 catalog 并
  `zk_connector_mcp::update_from_catalog`，logout 清理；更新事件可经 IPC
  `connectors.configure/reload/clear` 触发（web 侧接线）。
- OAuth 授权码/PKCE 客户端流程已实现（desktop Rust：127.0.0.1 临时端口 loopback redirect + 系统浏览器 + PKCE S256
  + token 交换/刷新，token 经 keychain IPC 本地保存；MCP HTTP transport 经 `set_oauth_token` 注入
  `Authorization: Bearer`，无 token 时 `MCP_OAUTH_REQUIRED`）。**剩余 UNVERIFIED**：真实 IdP/provider 联调未做
  （本机无外部网络/真实 OAuth MCP server）；OAuth 元数据 discovery（`.well-known`）未实现（endpoint 由 catalog 显式下发）。
- **剩余/UNVERIFIED**：远端 connector 生命周期管理（长驻进程守护/健康检查/崩溃重连）未实现（当前按需会话 +
  配置变更重启）。详见 `docs/07` §6.1/§6.2。

## 4. 安全/合规残余

- 安全门禁（`docs/21` §10；审计 JSON id 1–14，其中 id 14 为拆分自 gate 5 的 `mcp-stdio-transport`）由
  `scripts/security-audit.sh` 逐项输出 `pass/fail/unverified`/`unverified_external`（`env_only` 标记）；
  整体只有在全部 `pass` 时才为 `pass`，否则为 `unverified`/`fail` —— **发布说明不得写“安全门禁通过”**（`docs/11` §5）。
- 仅 gate 7（审计 WORM 日锚定，部署方控制，`docs/20` §3）、gate 9 产物级 SBOM（仅 release dist 生成）、
  gate 10 无 trivy report 时属 `unverified_external`（`env_only: true`）：CI strict 不因它们失败，也从不记为 pass。
  gate 6 密钥生命周期演练已由 master.key 测试 + `scripts/master-key-rotate.sh` 变为 CI 可满足项。
- gate 5b `mcp-stdio-transport`（审计 JSON id 14）原为 rc.1 已知产品缺口，B16 落地后已实现并在审计中 `pass`
  （`cargo test -p zk-connector-mcp`；本地最终证据 `target/evidence/release-1.0-rc1/security2/security.json`）。
  CI `ZEOKEEL_SECURITY_KNOWN_GAPS` 已清空，如 gate 14 回归即 strict 红。
- 镜像扫描（gate 10）在 CI security job 真实执行（pinned trivy 0.76.1 + 真实 server image）；PR/本地无
  `ZEOKEEL_SECURITY_TRIVY_REPORT` 时为 `unverified_external`/environment-only，不记 pass。依赖审计（gate 8：
  `cargo deny`/`cargo audit`/`pnpm audit`）在 CI 执行，本机无工具/网络时为 unverified。
- 威胁模型独立评审 **APPROVED**（精确行 `- 威胁模型评审结论：APPROVED`，`docs/reviews/2026-10-31-threat-model.md:3`，2026-10-31 复评）；M6 acceptance 仍 `blocked`
  （仅剩 Docker/证书/真机/CI 等环境项 + M6 实现复评）：`specs/changes/2026-10-30-hardening-release/acceptance.md`。
- **依赖审计策略（RC.1）**：`cargo-deny` 在 CI 执行；known vulnerability 一律 deny，未使用 API 的
  advisory 以窄理由 ignore：zip 4.6 的 `extract()` 系列（本项目只用 `by_index`+`enclosed_name` 并拒绝
  hostile entries）、zstd 0.13（仅 wasmtime 自产模块缓存；wasmtime 50 锁定 0.13）。`unmaintained`/`unsound`
  对传递依赖降为 warn（wasmtime 50 / Tauri 2.7 / keyring 4 的依赖钉死，无安全升级路径）；post-RC 升级
  wasmtime/tauri/keyring/zip 后应恢复为 deny。详见 `deny.toml` 注释。
- 第三方许可证清单（`THIRD-PARTY.md`）未生成；license 审计本机 UNVERIFIED。
- **2026-10-31 最终本机快照**（HEAD `6d89107` + A18 worktree）：非 strict exit 2、overall `unverified`、**pass=10 / fail=0 / unverified=1（gate 8 本机无 cargo-deny/audit/网络）/ unverified_external=3（gate 7/9/10）**；gate 1（威胁模型 APPROVED）与 gate 14（B16 MCP）pass。`ZEOKEEL_SECURITY_STRICT=1` 本机 exit 1 仅因 gate 8/trivy 的 CI 前置；CI `security.yml` 提供 PG/工具/镜像/trivy 后 strict 预期 exit 0（overall 仍不为 pass，env-only 7/9/10 保留）。
- CI 安全门禁证据：`target/evidence/release-1.0-rc1/security2/{security.json,security-audit.md,logs/}`、`target/evidence/release-1.0-rc1/security2-strict/security.json`；
  口径见 `docs/21` §10、`.github/workflows/security.yml`、`docs/18` §7。

## 5. 组合与生成物

- **SBOM 覆盖**：`zk sbom` 输出 CycloneDX 1.5，覆盖 `Cargo.lock` 全部 crate + pnpm workspace 直接包（`pnpm-workspace.yaml` globs）；
  pnpm transitive 依赖未纳入；`--edition/--profile/--channel` 只做组合校验，**不按组合过滤 components**；
  `--check` 只证明生成确定性，不证明镜像/交付物内容（`docs/09` §8）。
- **personal 组合的 apps/server 产物裁剪：done（A18，R8.2/F6，无 ADR 豁免）**。`zk compose` 按 `edition.targets`
  过滤：personal（`["desktop"]`）生成的 `apps/server/Cargo.toml` 无 `zk-srv-*` 依赖、`apps/server/src/main.rs` 为 stub、
  `apps/server/src/generated/packages.rs` 的 `PACKAGES` 为空；enterprise（`server,desktop`）由 compose 写完整
  server manifest/main（CI 构建路径）。测试断言 plan 闭包（无 `srv.`/`admin.`）、生成物（无 srv deps/空 glue）、
  发布 gating（`release.yml` server build/镜像仅 enterprise）三处，证据
  `target/evidence/release-1.0-rc1/phase-a/a18/`。
- `zk api`、`zk lock`、`zk verify-lock`、`zk metadata snapshot` 未实现（backlog，`docs/24` §3）；
  当前 CI 不包含这些命令，`zk api --check` 不得作为验收门禁。
- `cargo nextest`/`cargo public-api` 本机未安装，当前不在 CI 执行；`cargo deny`/`cargo machete` 仅在
  `.github/workflows/security.yml`（hosted runner 未实跑）。

## 6. 许可

- 本产品 MIT：根 `LICENSE` 全文；`Cargo.toml` `license = "MIT"`。
- OEM 产物按客户交付说明；参考仓库不作为依赖/构建输入（C15）。

## 7. 升级到 1.0.0 的条件

1. 本 `1.0.0-rc.1` 通过全量本地门禁与 rc 验收（含 A18 目标裁剪、A6 安全门禁、A8 M6 证据）。
2. 远端 CI（hosted runner）全绿；tag `v1.0.0-rc.1` 触发 release 产出真实三平台包/镜像/SBOM/SHA256SUMS。
3. 上述通过后由用户确认版本号从 `1.0.0-rc.1` 转为 `1.0.0`。
