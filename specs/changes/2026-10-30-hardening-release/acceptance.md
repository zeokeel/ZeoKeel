# 2026-10-30-hardening-release · 验收（M6）

- 状态：`blocked`（环境/实现阻塞项 UNVERIFIED；本地可执行项本轮已实跑，见下表，未 false-green）
- 本轮证据：2026-10-31，HEAD `d0a1c95` + `1.0.0-rc.1` 发布工程未提交改动（worktree）
- 验收人：pending（独立评审 `docs/reviews/2026-10-30-m6-implementation.md` 结论仍 CHANGES-REQUESTED，待复评）
- 证据目录：`target/evidence/m6/`、`target/evidence/release-1.0-rc1/security/`
- 13 项安全门禁唯一真源：`docs/21` §10；可执行实现：`scripts/security-audit.sh`

## 1. 自动验证（本轮实跑；exit 语义见 docs/11 §5）

| # | 命令 | 本轮结果（exit） | 证据 |
| --- | --- | --- | --- |
| 1 | `bash scripts/security-audit.sh` | **exit 2 unverified**：13 项 7 pass / 0 fail / 6 unverified（overall 不为 pass） | `target/evidence/m6/security/{security.json,security-audit.md,logs/}` |
| 1b | `ZEOKEEL_SECURITY_STRICT=1 bash scripts/security-audit.sh` | **exit 1 fail**（strict：unverified 视为 fail） | `target/evidence/m6/security-strict/security.json` |
| 2 | `bash scripts/backup-restore.sh` | **exit 2 unverified**：12 checks 9 pass / 3 unverified | `target/evidence/m6/backup-restore.json`、`m6/backup/` |
| 3 | `bash scripts/upgrade-drill.sh`（dry-run） | **exit 2 unverified**：manifest/schema/asset/backup-layout pass；Docker drill + 占位资产 unverified | `target/evidence/m6/upgrade-drill.json` |
| 3b | `bash scripts/upgrade-drill.sh --docker --dist <dist>` | 未执行：本机无 docker/dist（CI 可执行路径） | — |
| 4 | `bash scripts/fault-injection.sh` | **exit 2 unverified**：7 pass / 5 unverified | `target/evidence/m6/fault-injection.json` |
| 5 | `bash scripts/perf-budget.sh` | **exit 2 unverified**：本机预算项全过；image/500-user unverified | `target/evidence/m6/perf.json` |
| 6 | `bash scripts/reproducible-build.sh` | **exit 2 unverified**：4 artifacts 两次构建一致 / 2 unverified（image tar、严格 flag 集） | `target/evidence/m6/reproducible.json` |
| 7 | `bash scripts/admin-e2e.sh`（m6-a11y 矩阵） | 上轮 e2e：11 passed（含 admin a11y/portal axe）；client 全页矩阵未全量 | `target/evidence/release-check/e2e/admin-e2e.log` |
| 8 | `bash scripts/sbom.sh <out>` | 源码 SBOM pass；personal/enterprise CycloneDX 已产出；产物级 SBOM unverified | `target/evidence/release-1.0-rc1/sbom/`、`m6/security/sbom.json` |
| 9 | `python3 scripts/docs-check.py` / `spec-check.py` / `review-check.py` | 0 errors / 0 warnings；review-check ok（27 records） | 本地运行 |
| 10 | 独立评审 | M6 实现评审 CHANGES-REQUESTED（2026-10-30）；威胁模型评审记录待产出（A7） | `docs/reviews/2026-10-30-m6-implementation.md` |

## 1.1 本轮关键实测值（2026-10-31）

- perf：server ready 993ms、health p95 0.61ms、admin initial gzip 112419B、desktop initial gzip 345968B、mock first token 83.47ms、gateway p50 87.46ms / p95 122.66ms。
- backup/restore：source ready 1048ms、restore ready 470ms；users=2/audit=1/ledger=0/tables=32 回灌一致；audit hash-chain 0 broken；truncated dump 被 pg_restore 拒绝。
- fault injection：DB down→`/readyz` 503 且可恢复；audit 写失败→用户创建 HTTP 500 且 0 行写入（fail-closed）；kill -9 后 users/audit 行数不变、chain 完整；SIGTERM 10s 内退出。
- reproducible：zk `10953b6a…`、zk-server `6c77dc57…`、zk-desktop `a9659bc9…` 两次 clean build sha 一致；admin dist 3 文件两次构建一致。
- security 13 项：pass = 2（auth/JWT/CSRF/RBAC）、3（SSRF+rebinding）、4（zip/WASM）、5（command/input）、11（隐私口径）、12（日志/错误 secrets）、13（弱管控声明）；unverified = 1（威胁模型评审）、6（密钥生命周期）、7（WORM 锚定）、8（依赖审计工具）、9（产物 SBOM）、10（镜像扫描）。

## 2. UNVERIFIED（环境/实现阻塞，不得写 complete）

- 安全：威胁模型独立评审（A7）；`cargo deny/audit`、`pnpm audit --prod`（本机未安装/无网络，CI `security.yml`）；镜像扫描（无 docker/trivy/镜像）；产物级 SBOM。
- 密钥：master.key 文件/加密/轮换未实现（上游 key 明文 `api_key_enc`，`docs/07`“未决”）；JWT/更新签名轮换无本地演练；WORM 每日锚定未实现。
- 备份：WAL/PITR（`archive_mode=off`）、age/KMS 加密 + 异地、master.key 文件恢复。
- 升级：Docker 镜像 load/upgrade/rollback/restore（本机无 Docker/dist）；`deploy/updates/` 仍为 18B 占位客户端资产（build-dist 真实 dist 后才有意义）。
- 性能/容量：镜像体积、500 用户压测（无 harness/key）。
- 交付：三平台安装包、macOS 公证、Windows 真机、真实 provider/媒体在线（唯一真源 `KNOWN-LIMITATIONS.md` §2）。
- 评审：M6 实现复评、威胁模型评审未出（release-reviewer/A7）。

## 3. 说明

- 失败即红；环境项保持 UNVERIFIED；13 项门禁真源见 `docs/21` §10。
- 本 acceptance 只按本轮证据更新，不回写历史结论；`blocked` 保留到 13 项 + Docker/PITR/评审全部转 pass。
