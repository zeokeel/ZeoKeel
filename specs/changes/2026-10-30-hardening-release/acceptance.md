# 2026-10-30-hardening-release · 验收（M6）

- 状态：`in-progress`（spec 阶段）
- 验收人：pending
- 证据目录：`target/evidence/m6/`

## 检查

- [ ] `bash scripts/security-audit.sh`（JSON 0 高危）
- [ ] `bash scripts/backup-restore.sh`
- [ ] `bash scripts/upgrade-drill.sh --full`
- [ ] `bash scripts/fault-injection.sh`
- [ ] `bash scripts/perf-budget.sh`
- [ ] `bash scripts/reproducible-build.sh`
- [ ] axe 页面矩阵 + 键盘/i18n
- [ ] SBOM/license
- [ ] docs/spec/review + 独立评审

## UNVERIFIED 清单

- Docker 镜像扫描/save/load/compose/PITR；三平台安装包；macOS 公证；真实镜像 tar 复现；500 用户压测。

## 检查（旧）

- [ ] `bash scripts/security-audit.sh`
- [ ] `bash scripts/backup-restore.sh`
- [ ] `bash scripts/fault-injection.sh`
- [ ] `bash scripts/perf-budget.sh`
- [ ] `bash scripts/reproducible-build.sh`
- [ ] axe + 键盘 e2e
- [ ] docs/spec/review
- [ ] 独立评审 APPROVED

## 说明

环境阻塞（Docker/证书/平台安装包）标 UNVERIFIED，不得写 complete。
