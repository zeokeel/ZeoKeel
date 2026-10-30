# 2026-10-30-hardening-release · 验收（M6）

- 状态：`in-progress`
- 验收人：pending
- 证据目录：`target/evidence/m6/`

## 1. 自动验证（每项 exact command + 阈值/失败即红）

| # | 命令 | 通过条件 | 证据 |
| --- | --- | --- | --- |
| 1 | `bash scripts/security-audit.sh` | 13 项 JSON，0 高危，status=pass | `target/evidence/m6/security.json` |
| 2 | `bash scripts/backup-restore.sh` | restore→migrate→start→login/smoke→reconcile/audit pass | `backup-restore.log` |
| 3 | `bash scripts/upgrade-drill.sh --full` | backup→upgrade→rollback→restore pass；Docker 子项 UNVERIFIED | `upgrade-full.log` |
| 4 | `bash scripts/fault-injection.sh` | kill -9/PG stop/disk/audit fail 矩阵 pass；每项一致性断言 | `fault-injection.json` |
| 5 | `bash scripts/perf-budget.sh` | server ready ≤2s、health p95 ≤50ms、admin ≤400KB gzip、desktop ≤500KB gzip、mock 首 token ≤300ms；镜像/网关记录 | `perf.json` |
| 6 | `bash scripts/reproducible-build.sh` | 两次冷构建 zk/server/desktop/admin dist sha 完全一致（镜像 UNVERIFIED） | `reproducible.json` |
| 7 | axe 矩阵 | admin/client/portal 每页 0 serious/critical；Tab/Esc/⌘K/focus/reduced motion/zh-en | screenshots + axe.json |
| 8 | SBOM/license | 每产物 CycloneDX + license allowlist；deep OEM 包子集 | `SBOM/` |
| 9 | `python3 scripts/docs-check.py` / `spec-check.py` / `review-check.py` | 0 errors | 日志 |
| 10 | 独立评审 | APPROVED | `docs/reviews/2026-10-30-m6-implementation.md` |

## 2. UNVERIFIED（环境阻塞，不得写 complete）

- Docker 镜像扫描/save/load/compose/PITR 实跑；三平台安装包；macOS 公证；镜像 tar 可复现；500 用户压测。

## 3. 说明

- 失败即红；P1 issue 需 owner/复评；环境项保持 UNVERIFIED。
