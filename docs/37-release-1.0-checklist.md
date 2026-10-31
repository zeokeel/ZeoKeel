# 37 · 1.0 发布清单（M6）

> 目标版本：**`1.0.0-rc.1`**（2026-10-31，本地验证 + GitHub Actions 远端 CI/security/release 实跑）。`1.0.0` 正式版需 rc 验收 + 远端 CI/release 实跑通过后由用户确认，
> 不得据本清单声称 1.0.0 已发布。图例：`[x]` 有证据；`[ ]` **UNVERIFIED**（原因 + 复现命令）。
> 全部残余项的唯一真源：根 [`KNOWN-LIMITATIONS.md`](../KNOWN-LIMITATIONS.md)；证据目录 `target/evidence/`（不入库）。

| # | 发布项 | 状态 | 证据 / 原因与命令 |
| --- | --- | --- | --- |
| 1 | M0–M5 证据齐全；M5 环境阻塞项标 UNVERIFIED | `[x]` 软件/本地；`[ ]` 环境项 | 本地证据：`target/evidence/release-check/{rust,web,e2e,compose}/`；M5 环境项（镜像/三平台包/公证/真实 OEM 索引）见 `KNOWN-LIMITATIONS.md` §2 |
| 2 | 安全 13 项门禁 0 高危；威胁模型评审 | `[x]` 本地 + CI strict | CI `security.yml` strict 实跑：**exit 0、pass=12 / fail=0 / unverified=0 / unverified_external=2**（gate 7 WORM 部署控制、gate 9 release-only SBOM）；gate1 威胁模型评审 **APPROVED**（`docs/reviews/2026-10-31-threat-model.md:3`）、gate10 trivy（0 fixable HIGH/CRITICAL；68 项无上游修复登记 `KNOWN-LIMITATIONS.md` §2）、gate14（B16 MCP）均 pass。本机非 strict：exit 2、pass=10/fail=0/unverified=1（gate 8 本机工具）/external=3，证据 `target/evidence/release-1.0-rc1/ci/final/security.json`。口径 `docs/21` §10、`docs/18` §7 |
| 3 | 备份→升级→回滚→恢复演练；故障注入无数据损坏 | `[ ]` UNVERIFIED | 本地：`scripts/backup-restore.sh`（pg_dump；PITR/WAL 未验证）、`scripts/fault-injection.sh`（3 probes pass）、`scripts/upgrade-drill.sh --docker`（无 Docker）；证据 `target/evidence/m6/`；见 `KNOWN-LIMITATIONS.md` §2 |
| 4 | axe/键盘/i18n 全页矩阵；性能预算达标 | `[x]` 本地；`[ ]` 生产容量 | a11y/键盘矩阵（B12）：client（serve 空态/聊天/设置/命令面板/检查器 + Tab/焦点/Esc/⌘K/Ctrl+B/reduced-motion）、admin（登录 + 10 页）、portal（runtime+静态 zh/en），0 serious/critical；证据 `target/evidence/release-1.0-rc1/phase-b/b12b14/{SUMMARY.md,m1-a11y.log,admin-e2e.log}`。性能（B14 修复后本地 500 用户 smoke）：**exit 0，0 错误，gateway p50 38.2ms / p95 46.5ms / max 131ms，稳定吞吐 98.41 settled req/s**，health p95 1.72ms；证据 `.../phase-b/b14-perf/load-smoke-500.json`。本机单机 PG+假上游，**生产容量 UNVERIFIED**（需 external 模式对真实部署复测） |
| 5 | 可复现构建两次一致；SBOM/license；依赖审计 | `[x]` 本地二进制/dist；`[ ]` 镜像/许可 | `scripts/reproducible-build.sh`（M6 复跑 `target/evidence/m6/reproducible.json`）：**zk / zk-server / zk-desktop / admin-dist 4 项两次构建一致**；镜像 tar 与严格 flag 集 unverified（无 Docker；CI 冷构建）。SBOM 本地已产出工作区级 CycloneDX（`target/evidence/release-1.0-rc1/sbom/`）；`THIRD-PARTY.md` 与 cargo-deny/pnpm license 审计仍未本机执行（CI `security.yml` 路径） |
| 6 | 三平台包/镜像/门户/文档（含 known limitations） | `[x]` CI 实跑产出（企业版）；personal 另跑 | GitHub Release `v1.0.0-rc.1` 已由 release workflow 实跑产出：真实 AppImage/deb/dmg/nsis + server/postgres/caddy 镜像 tar + CycloneDX SBOM + `SHA256SUMS` + 完整 dist 归档 + UNVERIFIED 说明（Windows 未签名 ADR-033、macOS 未公证 ADR-037）。**personal×official** 安装器需另行 `workflow_dispatch` 运行（C1/C13）；macOS notarize / Docker upgrade-rollback drill 仍为环境项（见行 3、7）。文档：`KNOWN-LIMITATIONS.md`（摘要 `docs/40`）、release 说明生成器 |
| 7 | CHANGELOG/版本 `1.0.0-rc.1`；Windows 未签名声明；macOS 公证就绪（证书后） | `[x]` 版本；`[ ]` 公证 | 版本跨面一致：`Cargo.toml`、根 `package.json`、`apps/*/web/package.json`、`apps/desktop/tauri.conf.json`；CHANGELOG 见 `CHANGELOG.md`；Windows **不签名**（ADR-033）；macOS 公证待证书后启用（ADR-037） |
| 8 | 1.0.0 正式版 | `[ ]` | 需 `1.0.0-rc.1` 全量验收 + 远端 CI/release 实跑（tag `v1.0.0-rc.1` → 三平台包/镜像/SBOM/SHA256SUMS）通过后由用户确认 |
