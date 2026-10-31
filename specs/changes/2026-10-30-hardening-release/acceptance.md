# 2026-10-30-hardening-release · 验收（M6）

- 状态：`blocked`（环境项 UNVERIFIED；CI/发布/安全可执行项已实跑，见下表，未 false-green）
- 本轮证据（final refresh）：2026-11-01，HEAD `c1aebf9`；发布 tag `v1.0.0-rc.1`（artifact commit `5c0f29d`）；rc.1 GitHub Release 已发布
- 验收人：lead + 独立评审 `docs/reviews/2026-10-31-release-1-0-readiness.md`；M6 实现复评 **APPROVED**（`docs/reviews/2026-10-30-m6-implementation.md` §10）
- 证据目录：`target/evidence/m6/`、`target/evidence/release-1.0-rc1/security/`
- 13 项安全门禁唯一真源：`docs/21` §10；可执行实现：`scripts/security-audit.sh`

## 1. 自动验证（本轮实跑；exit 语义见 docs/11 §5）

| # | 命令 | 本轮结果（exit） | 证据 |
| --- | --- | --- | --- |
| 1 | `bash scripts/security-audit.sh`（CI strict） | **exit 0、overall unverified_external、pass=12 / fail=0 / unverified=0 / unverified_external=2**（gate 7 WORM 部署控制 ADR-068、gate 9 产物级 SBOM release-only）；gate 1 威胁模型 **APPROVED**、gate 10 trivy（0 fixable HIGH/CRITICAL；68 项无上游修复登记 `KNOWN-LIMITATIONS.md` §2）、gate 14（B16 MCP）pass | GitHub Actions security run `41225234844` artifact `security-evidence`；本机非 strict 见 `target/evidence/release-1.0-rc1/ci/final/security.json`（pass=10 / fail=0 / unverified=1 gate 8 / external=3）。旧 `security2/` 为 superseded 失败快照，非权威 |
| 1b | `ZEOKEEL_SECURITY_STRICT=1 bash scripts/security-audit.sh` | CI strict 已实跑 exit 0（见行 1）；本机 strict exit 1 仅因 gate 8 缺少 CI 工具（预期，不计 pass） | security run `41225234844`；语义 `docs/18` §7、`docs/21` §10 |
| 2 | `bash scripts/backup-restore.sh` | **exit 2 unverified**：12 checks 9 pass / 3 unverified | `target/evidence/m6/backup-restore.json`、`m6/backup/` |
| 3 | `bash scripts/upgrade-drill.sh`（dry-run） | **exit 2 unverified**：manifest/schema/asset/backup-layout pass；Docker drill + 占位资产 unverified | `target/evidence/m6/upgrade-drill.json` |
| 3b | `bash scripts/upgrade-drill.sh --docker --dist <dist>` | **CI 已执行、结果 UNVERIFIED（环境失败）**：release workflow `upgrade-drill` job 在 hosted runner 实跑，compose postgres unhealthy 导致失败；job `continue-on-error`，证据 artifact `upgrade-drill-evidence`，release body 标 UNVERIFIED（不记 pass） | release run `41222029806` 的 `upgrade-drill-evidence`；`KNOWN-LIMITATIONS.md` §2 |
| 4 | `bash scripts/fault-injection.sh` | **exit 2 unverified**：7 pass / 5 unverified | `target/evidence/m6/fault-injection.json` |
| 5 | `bash scripts/perf-budget.sh` + `scripts/load-smoke.sh --local --users 500` | perf-budget exit 2（image/容量环境项）；**500 用户本地 smoke 修复后 exit 0**：0 错误、gateway p50 38.2ms / p95 46.5ms、98.41 settled req/s（生产容量仍 UNVERIFIED） | `target/evidence/m6/perf.json`；`target/evidence/release-1.0-rc1/phase-b/b14-perf/load-smoke-500.json` |
| 6 | `bash scripts/reproducible-build.sh` | **exit 2 unverified**：**zk / zk-server / zk-desktop / admin-dist 4 项两次构建一致**；2 unverified（image tar、严格 flag 集；CI 冷构建覆盖） | `target/evidence/m6/reproducible.json` |
| 7 | CI `e2e` + CI `a11y` jobs（真实 serve carrier） | CI 实跑绿：`e2e` **14 passed**（含 m1 a11y 7 项 + serve/security/m1-serve）；`a11y` **15 passed**（client m2/m3/m4 + admin 10 页 + portal，0 serious/critical）；本地 m1 复跑 14/14 | CI run `41219684321` / `41225234859` 日志；`target/evidence/release-1.0-rc1/phase-b/b12b14/` |
| 8 | `bash scripts/sbom.sh <out>` / release dist SBOM | 源码 SBOM pass；personal/enterprise CycloneDX 已产出；**release dist 的 `SBOM/cyclonedx.json` 已发布并以 SHA256SUMS 校验**；镜像层内 SBOM 仍 UNVERIFIED | `target/evidence/release-1.0-rc1/sbom/`；GitHub Release `v1.0.0-rc.1` 资产；`KNOWN-LIMITATIONS.md` §5 |
| 9 | `python3 scripts/docs-check.py` / `spec-check.py` / `review-check.py` | docs-check 0/0（41 files）；spec-check 0/0；review-check ok（28 records） | CI `docs` job；本地运行 |
| 10 | 独立评审 | 威胁模型评审 **APPROVED**（`docs/reviews/2026-10-31-threat-model.md:3`）；rc.1 终评 **APPROVED**（`docs/reviews/2026-10-31-release-1-0-readiness.md:154` + hotfix addendum）；M6 实现复评 **APPROVED**（`docs/reviews/2026-10-30-m6-implementation.md` §10，`- 复评结论：APPROVED`） | 上述评审记录；review-check 28 records |

## 1.1 本轮关键实测值（2026-10-31）

- perf：server ready 993ms、health p95 0.61ms、admin initial gzip 112419B、desktop initial gzip 345968B、mock first token 83.47ms、gateway p50 87.46ms / p95 122.66ms。
- backup/restore：source ready 1048ms、restore ready 470ms；users=2/audit=1/ledger=0/tables=32 回灌一致；audit hash-chain 0 broken；truncated dump 被 pg_restore 拒绝。
- fault injection：DB down→`/readyz` 503 且可恢复；audit 写失败→用户创建 HTTP 500 且 0 行写入（fail-closed）；kill -9 后 users/audit 行数不变、chain 完整；SIGTERM 10s 内退出。
- reproducible：zk `10953b6a…`、zk-server `6c77dc57…`、zk-desktop `a9659bc9…` 两次 clean build sha 一致；admin dist 3 文件两次构建一致。
- security 最终快照（HEAD `6d89107` + A18 worktree，非 strict exit 2）：pass = gate 1（威胁模型 APPROVED）、2、3、4、5a、5b/14（B16 MCP）、6（master.key ring + rotate drill）、11、12、13；unverified = 8（本机无 cargo-deny/audit、pnpm 无网络 → CI）；unverified_external = 7（WORM 部署控制，ADR-068）、9（产物级 SBOM 仅 release）、10（本机无 trivy 报告）。
- perf（B14 修复后，本地 500 用户 smoke）：exit 0、0 错误、gateway p50 38.2ms / p95 46.5ms / max 131.4ms、98.41 settled req/s、health p95 1.72ms（单机 PG + 假上游；生产容量 UNVERIFIED）。

## 2. UNVERIFIED（环境/实现阻塞，不得写 complete）

- 安全：CI strict 已实跑通过（pass=12 / fail=0 / external=2），无未验证工具项；WORM 外部锚定为部署方控制（ADR-068），产物级 SBOM 仅 release dist（gate 9）——均 env-only，不记 pass。
- 密钥：master.key 文件/加密/轮换已实现（AES-256-GCM versioned ring + `scripts/master-key-rotate.sh`）；JWT/更新签名轮换仍是离线/HSM 部署演练（无本地可执行 drill，`unverified_external`）。
- 备份：WAL/PITR（`archive_mode=off`）、age/KMS 加密 + 异地、master.key 文件恢复。
- 升级：Docker 镜像 load/upgrade/rollback/restore（本机无 Docker/dist）；`deploy/updates/` 仍为 18B 占位客户端资产（build-dist 真实 dist 后才有意义）。
- 性能/容量：生产 500 用户容量对真实部署 UNVERIFIED（本地 smoke 已 pass）；镜像体积未测。
- 交付：三平台安装包已由 CI release 产出并校验；**macOS 公证、Windows 真机安装、真实 provider/媒体在线、500 用户生产容量、镜像层内 SBOM、Docker load/upgrade/rollback drill** 仍 UNVERIFIED（唯一真源 `KNOWN-LIMITATIONS.md` §2）。personal×official 安装器需单独 workflow_dispatch。
- 评审：威胁模型与 rc.1 终评已 APPROVED；M6 实现复评状态以 `docs/reviews/2026-10-30-m6-implementation.md` §9 为准。

## 2.1 rc.1 发布证据（2026-11-01）

- GitHub Release `v1.0.0-rc.1`（prerelease）：真实 AppImage/deb/dmg/nsis、server/postgres/caddy 镜像 tar、CycloneDX SBOM、`SHA256SUMS`、完整 dist 归档（`zeokeel-dist-1.0.0-rc.1-official.tar.gz`）；Windows 安装器 sha256 `feaf270f…`（hotfix 后）。
- 发布流水线：release run `41222029806` success（三平台安装器 + dist + GitHub Release）；release job 解包 dist 归档后 `sha256sum -c SHA256SUMS` 25/25 OK，13 个 loose asset digest 与 SHA256SUMS 一致。
- CI/security：`41225234859`（HEAD `c1aebf9`，9/9 jobs）与 `41225234844`（strict 安全审计）success。
- 遗留：upgrade-drill 环境失败已记录为 UNVERIFIED（证据 artifact），不得当 pass。

## 3. 说明

- 失败即红；环境项保持 UNVERIFIED；13 项门禁真源见 `docs/21` §10。
- 本 acceptance 只按本轮证据更新，不回写历史结论；`blocked` 保留到环境项（Docker drill/PITR/公证/真机/真实 provider/生产容量）实际验收；评审与环境 UNVERIFIED 口径见 `KNOWN-LIMITATIONS.md` §2。
