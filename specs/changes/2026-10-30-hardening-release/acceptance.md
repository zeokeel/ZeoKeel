# 2026-10-30-hardening-release · 验收（M6）

- 状态：`blocked`（仅剩环境/CI/复评项 UNVERIFIED；本地可执行项本轮已实跑，见下表，未 false-green）
- 本轮证据（final refresh）：2026-10-31，HEAD `f0bb2b6` + 未提交 worktree（A18 target trimming、Phase B、安全 round-2、perf 修复）
- 验收人：pending（独立评审 `docs/reviews/2026-10-30-m6-implementation.md` 结论仍 CHANGES-REQUESTED，待复评）
- 证据目录：`target/evidence/m6/`、`target/evidence/release-1.0-rc1/security/`
- 13 项安全门禁唯一真源：`docs/21` §10；可执行实现：`scripts/security-audit.sh`

## 1. 自动验证（本轮实跑；exit 语义见 docs/11 §5）

| # | 命令 | 本轮结果（exit） | 证据 |
| --- | --- | --- | --- |
| 1 | `bash scripts/security-audit.sh`（非 strict） | **exit 2、overall unverified**：14 行（13 项 + 5b 拆分）**10 pass / 0 fail / 1 unverified（gate 8 本机无 cargo-deny/audit/网络）/ 3 unverified_external（gate 7 WORM、gate 9 产物 SBOM、gate 10 无 trivy 报告）**；gate 1（威胁模型 **APPROVED**）与 gate 14（B16 MCP transport）pass | `target/evidence/release-1.0-rc1/security2/{security.json,security-audit.md,logs/}` |
| 1b | `ZEOKEEL_SECURITY_STRICT=1 bash scripts/security-audit.sh` | **本机 exit 1**（strict：CI 可满足的 gate 8 unverified 视为 fail；env-only 7/9/10 不参与）；CI `security.yml` 提供 PG + cargo 工具 + 真实镜像/trivy 报告后 strict 预期 exit 0（仅 env-only 未验证，overall 仍不为 pass） | `target/evidence/release-1.0-rc1/security2/security.json`（非 strict 同一状态）；strict 语义见 `docs/18` §7、`docs/21` §10 |
| 2 | `bash scripts/backup-restore.sh` | **exit 2 unverified**：12 checks 9 pass / 3 unverified | `target/evidence/m6/backup-restore.json`、`m6/backup/` |
| 3 | `bash scripts/upgrade-drill.sh`（dry-run） | **exit 2 unverified**：manifest/schema/asset/backup-layout pass；Docker drill + 占位资产 unverified | `target/evidence/m6/upgrade-drill.json` |
| 3b | `bash scripts/upgrade-drill.sh --docker --dist <dist>` | 未执行：本机无 docker/dist（CI 可执行路径） | — |
| 4 | `bash scripts/fault-injection.sh` | **exit 2 unverified**：7 pass / 5 unverified | `target/evidence/m6/fault-injection.json` |
| 5 | `bash scripts/perf-budget.sh` + `scripts/load-smoke.sh --local --users 500` | perf-budget exit 2（image/容量环境项）；**500 用户本地 smoke 修复后 exit 0**：0 错误、gateway p50 38.2ms / p95 46.5ms、98.41 settled req/s（生产容量仍 UNVERIFIED） | `target/evidence/m6/perf.json`；`target/evidence/release-1.0-rc1/phase-b/b14-perf/load-smoke-500.json` |
| 6 | `bash scripts/reproducible-build.sh` | **exit 2 unverified**：**zk / zk-server / zk-desktop / admin-dist 4 项两次构建一致**；2 unverified（image tar、严格 flag 集；CI 冷构建覆盖） | `target/evidence/m6/reproducible.json` |
| 7 | `bash scripts/admin-e2e.sh` + `bash scripts/m1-e2e.sh`（a11y B12 矩阵） | admin/portal **15 passed**（0 serious/critical）；client `e2e/m1-a11y.spec.ts` **7 passed**（空态/聊天/设置/命令面板/检查器 + 键盘/焦点/reduced-motion）；证据本轮 | `target/evidence/release-1.0-rc1/phase-b/b12b14/{admin-e2e.log,m1-a11y.log,SUMMARY.md}` |
| 8 | `bash scripts/sbom.sh <out>` | 源码 SBOM pass；personal/enterprise CycloneDX 已产出；产物级 SBOM unverified | `target/evidence/release-1.0-rc1/sbom/`、`m6/security/sbom.json` |
| 9 | `python3 scripts/docs-check.py` / `spec-check.py` / `review-check.py` | 0 errors / 0 warnings；review-check ok（27 records） | 本地运行 |
| 10 | 独立评审 | 威胁模型评审 **APPROVED**（`docs/reviews/2026-10-31-threat-model.md:3`）；M6 实现评审仍 CHANGES-REQUESTED（需复评） | `docs/reviews/2026-10-31-threat-model.md`、`docs/reviews/2026-10-30-m6-implementation.md` |

## 1.1 本轮关键实测值（2026-10-31）

- perf：server ready 993ms、health p95 0.61ms、admin initial gzip 112419B、desktop initial gzip 345968B、mock first token 83.47ms、gateway p50 87.46ms / p95 122.66ms。
- backup/restore：source ready 1048ms、restore ready 470ms；users=2/audit=1/ledger=0/tables=32 回灌一致；audit hash-chain 0 broken；truncated dump 被 pg_restore 拒绝。
- fault injection：DB down→`/readyz` 503 且可恢复；audit 写失败→用户创建 HTTP 500 且 0 行写入（fail-closed）；kill -9 后 users/audit 行数不变、chain 完整；SIGTERM 10s 内退出。
- reproducible：zk `10953b6a…`、zk-server `6c77dc57…`、zk-desktop `a9659bc9…` 两次 clean build sha 一致；admin dist 3 文件两次构建一致。
- security 最终快照（HEAD `6d89107` + A18 worktree，非 strict exit 2）：pass = gate 1（威胁模型 APPROVED）、2、3、4、5a、5b/14（B16 MCP）、6（master.key ring + rotate drill）、11、12、13；unverified = 8（本机无 cargo-deny/audit、pnpm 无网络 → CI）；unverified_external = 7（WORM 部署控制，ADR-068）、9（产物级 SBOM 仅 release）、10（本机无 trivy 报告）。
- perf（B14 修复后，本地 500 用户 smoke）：exit 0、0 错误、gateway p50 38.2ms / p95 46.5ms / max 131.4ms、98.41 settled req/s、health p95 1.72ms（单机 PG + 假上游；生产容量 UNVERIFIED）。

## 2. UNVERIFIED（环境/实现阻塞，不得写 complete）

- 安全（CI-only）：`cargo deny/audit`、`pnpm audit --prod`、镜像扫描（pinned trivy）需 hosted runner / CI `security.yml`；产物级 SBOM 仅 release dist。WORM 外部锚定为部署方控制（ADR-068）。
- 密钥：master.key 文件/加密/轮换已实现（AES-256-GCM versioned ring + `scripts/master-key-rotate.sh`）；JWT/更新签名轮换仍是离线/HSM 部署演练（无本地可执行 drill，`unverified_external`）。
- 备份：WAL/PITR（`archive_mode=off`）、age/KMS 加密 + 异地、master.key 文件恢复。
- 升级：Docker 镜像 load/upgrade/rollback/restore（本机无 Docker/dist）；`deploy/updates/` 仍为 18B 占位客户端资产（build-dist 真实 dist 后才有意义）。
- 性能/容量：生产 500 用户容量对真实部署 UNVERIFIED（本地 smoke 已 pass）；镜像体积未测。
- 交付：三平台安装包、macOS 公证、Windows 真机、真实 provider/媒体在线（唯一真源 `KNOWN-LIMITATIONS.md` §2）。
- 评审：M6 实现复评仍 pending（威胁模型评审已 APPROVED）；CI strict 绿需 hosted runner 实跑确认。

## 3. 说明

- 失败即红；环境项保持 UNVERIFIED；13 项门禁真源见 `docs/21` §10。
- 本 acceptance 只按本轮证据更新，不回写历史结论；`blocked` 保留到 13 项 + Docker/PITR/评审全部转 pass。
