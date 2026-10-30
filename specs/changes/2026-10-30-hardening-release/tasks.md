# 2026-10-30-hardening-release · 任务清单

| # | 任务 | 写作用域（文件/目录） | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | 威胁模型/13 项安全门禁 + security-audit | `docs/33-threat-model.md`, `docs/README.md`, `scripts/security-audit.sh`, `docs/00/21` | — | `bash scripts/security-audit.sh` | ☐ |
| 2 | 备份/恢复全链路 | `scripts/backup-restore.sh`, `docs/20` | — | `bash scripts/backup-restore.sh` | ☐ |
| 3 | 升级/回滚/恢复演练 | `scripts/upgrade-drill.sh`, `docs/20` | 2 | `bash scripts/upgrade-drill.sh --full` | ☐ |
| 4 | 故障注入矩阵 | `scripts/fault-injection.sh` | 2 | `bash scripts/fault-injection.sh` | ☐ |
| 5 | 日志/错误/审计脱敏检查 | `scripts/security-audit.sh`, server/auth | 1 | grep + 运行期负向 | ☐ |
| 6 | axe/键盘/i18n 页面矩阵 | `e2e/m6-a11y.spec.ts`, scripts | — | Playwright + axe | ☐ |
| 7 | 性能预算 | `scripts/perf-budget.sh` | — | `bash scripts/perf-budget.sh` 阈值 | ☐ |
| 8 | 可复现/SBOM/license | `scripts/reproducible-build.sh`, `scripts/sbom.sh` | — | 两次 hash 一致 + SBOM | ☐ |
| 9 | 1.0 文档/CHANGELOG/README/CI | `docs/29-32`, `CHANGELOG.md`, `README.md`, `.github/workflows/security.yml` | 1–8 | docs-check | ☐ |
| 10 | 验收/归档 | acceptance | 1–9 | review-check; 独立评审 | ☐ |

## 2. 写作用域

- 只写：上述脚本/docs/e2e/CI/CHANGELOG/README；不改业务契约/迁移。
- 环境阻塞项：Docker/证书/平台 runner 标 UNVERIFIED。

## 3. 验收前检查

- [ ] `security-audit` 0 高危
- [ ] `backup-restore` / `upgrade-drill --full` / `fault-injection`
- [ ] axe 矩阵 + 键盘/i18n
- [ ] `perf-budget` 阈值
- [ ] `reproducible-build` 两次一致 + SBOM/license
- [ ] docs/spec/review + 独立评审
