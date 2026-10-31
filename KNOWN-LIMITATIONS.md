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
| 可复现构建（server/desktop/admin/镜像） | UNVERIFIED | `scripts/reproducible-build.sh` 本地只比较 `zk` debug 二进制；其余标 unverified |
| 性能预算 | 部分 UNVERIFIED | `scripts/perf-budget.sh` 的网关/镜像等字段为 null/unverified；本机基线见 `target/evidence/m6/perf.json` |
| 500 用户负载/压测 | UNVERIFIED | 无压测环境；仅 100 用户级本地基线设计（`docs/20` §6） |
| 真实 provider/媒体/embedding 在线调用 | UNVERIFIED | 无真实 key/网络；本地为 mock/fixture；企业版媒体/embeddings 按 C7 不提供 |
| SBOM 打包进交付 tar/镜像 | UNVERIFIED | `zk sbom` 已实现工作区级 CycloneDX；SBOM 进 tar/镜像由 release 流程处理，本机未验证（`docs/09` §8） |
| `zk channel verify` 对真实 dist | UNVERIFIED | 只在 dry-run/占位 dist 上验证过；真实 dist 未产出 |
| 三平台真机安装/升级冒烟 | UNVERIFIED | 无真机；本地仅 `client-upgrade-e2e.sh` 模拟链路 |

## 3. 功能残余（partial / 平台限制 / 设计边界）

### 3.1 Agent 运行时与 rig 接线（详见 `docs/38` §5.4）

- `AgentEvent` 无独立 usage 事件：usage 随 `TurnCompleted.reply` 携带；usage 已持久化并在加载后恢复展示。
- `SessionStore::messages_in` 历史回放未实现：多轮依赖 rig-memory 目录日志，未统一由会话 DB 回放。
- rig 的 run-state 序列化/`Recorder`/`observe`/`telemetry`/`effect`、`ModelCatalog` 上下文窗口/压缩**未接**；
  rig 0.45 实际 API 可行性以 spike 结论为准（`target/evidence/release-1.0-rc1/spikes/`）。
- `pdf`/`epub` 文档 loader 未启用（ADR-067）；`read_document` 为独立实现。

### 3.2 工具能力

| 工具 | 状态 | 说明 |
| --- | --- | --- |
| `terminal` | partial | 每会话单会话；多会话未实现 |
| `pwsh-persistent` | unsupported | 平台门控；非 Windows 诚实不可用 |
| `goal` | partial | 无完整 round driver |
| `ralph` | partial | fresh-context 隔离未实现 |
| `agent-team` | experimental | 未达到 implemented 口径 |
| `tool-cordis` | disabled | 未启用 |

### 3.3 配置与能力边界

- agent hooks/memory/web-search 等仍以 env 为主（`AppSettings` 覆盖为部分实现），设置 UI 覆盖不完整。
- 企业版媒体/embeddings **按设计不可用**（C7/ADR-031）：UI 隐藏、`settings.set` 拒绝；客户端不得持有上游 key。
- `cap-vector` 仅本地 JSON 持久化：无跨进程写锁、无 SQLite/FTS、无跨设备同步（最后一写者胜），enterprise 无 embeddings 路由。

### 3.4 可访问性/i18n

- axe 矩阵仅覆盖现有 e2e（client serve、admin/login）用例；client/admin/portal 全页矩阵未全量（`docs/19` §6 为准）。
- zh/en 文案完整性只按现有页面检查，未做全字符串审计。

## 4. 安全/合规残余

- 安全门禁（`docs/21` §10，13 项）由 `scripts/security-audit.sh` 逐项输出；**外部工具/环境项**
  （`cargo-deny`/`cargo-audit`/`trivy`/`pnpm audit`、真实 provider）在本机为 `unverified`，
  整体状态不得为 `pass`；`ZEOKEEL_SECURITY_STRICT=1` 时 unverified 视为 fail。
- 威胁模型独立评审记录见 `docs/reviews/`（评审任务产出前为 UNVERIFIED）。
- M6 acceptance 仍为 `blocked`（Docker/证书/真机等环境项未完成）：`specs/changes/2026-10-30-hardening-release/acceptance.md`。
- 第三方许可证清单（`THIRD-PARTY.md`）未生成；`cargo-deny`/`pnpm` license 审计在本机 UNVERIFIED。

## 5. 组合与生成物

- **SBOM 覆盖**：`zk sbom` 输出 CycloneDX 1.5，覆盖 `Cargo.lock` 全部 crate + pnpm workspace 直接包（`pnpm-workspace.yaml` globs）；
  pnpm transitive 依赖未纳入；`--edition/--profile/--channel` 只做组合校验，**不按组合过滤 components**；
  `--check` 只证明生成确定性，不证明镜像/交付物内容（`docs/09` §8）。
- **personal 组合的 apps/server 产物裁剪**：目标裁剪由 A18 生成器任务实现；**在 A18 验收通过前，personal 组合仍生成/携带
  `apps/server` 目标**，不得声称已裁剪；A18 完成后以生成物与 `zk compose --check` 实际结果为准（R8.2）。
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
