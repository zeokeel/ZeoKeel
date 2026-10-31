# 37 · 1.0 发布清单（M6）

> 目标版本：**`1.0.0-rc.1`**（2026-10-31，本地验证）。`1.0.0` 正式版需 rc 验收 + 远端 CI/release 实跑通过后由用户确认，
> 不得据本清单声称 1.0.0 已发布。图例：`[x]` 有证据；`[ ]` **UNVERIFIED**（原因 + 复现命令）。
> 全部残余项的唯一真源：根 [`KNOWN-LIMITATIONS.md`](../KNOWN-LIMITATIONS.md)；证据目录 `target/evidence/`（不入库）。

| # | 发布项 | 状态 | 证据 / 原因与命令 |
| --- | --- | --- | --- |
| 1 | M0–M5 证据齐全；M5 环境阻塞项标 UNVERIFIED | `[x]` 软件/本地；`[ ]` 环境项 | 本地证据：`target/evidence/release-check/{rust,web,e2e,compose}/`；M5 环境项（镜像/三平台包/公证/真实 OEM 索引）见 `KNOWN-LIMITATIONS.md` §2 |
| 2 | 安全 13 项门禁 0 高危；威胁模型评审 | `[ ]` UNVERIFIED | 命令：`bash scripts/security-audit.sh`（外部工具/环境项 `unverified`，整体不得为 pass）；威胁模型评审记录 `docs/reviews/`（评审任务产出前 UNVERIFIED）；口径见 `docs/21` §10 |
| 3 | 备份→升级→回滚→恢复演练；故障注入无数据损坏 | `[ ]` UNVERIFIED | 本地：`scripts/backup-restore.sh`（pg_dump；PITR/WAL 未验证）、`scripts/fault-injection.sh`（3 probes pass）、`scripts/upgrade-drill.sh --docker`（无 Docker）；证据 `target/evidence/m6/`；见 `KNOWN-LIMITATIONS.md` §2 |
| 4 | axe/键盘/i18n 全页矩阵；性能预算达标 | `[ ]` UNVERIFIED | 现有 axe 用例随 `scripts/admin-e2e.sh`/`login-e2e.sh` 通过（`target/evidence/release-check/e2e/`）；client/admin/portal 全页矩阵未全量；`scripts/perf-budget.sh` 网关/镜像字段 unverified |
| 5 | 可复现构建两次一致；SBOM/license；依赖审计 | `[ ]` 部分 | `scripts/reproducible-build.sh` 仅 `zk` debug 二进制一致（server/desktop/admin/镜像 UNVERIFIED）；SBOM 本地已产出（`target/evidence/release-1.0-rc1/sbom/`，工作区级覆盖，见 `docs/09` §8）；`THIRD-PARTY.md`/cargo-deny/pnpm license 审计 UNVERIFIED |
| 6 | 三平台包/镜像/门户/文档（含 known limitations） | `[ ]` UNVERIFIED | 安装包/镜像/门户未在本机产出（无 Docker、Tauri CLI、证书/真机）；已知限制文档已有：`KNOWN-LIMITATIONS.md`（摘要 `docs/40`）；命令/原因见 `KNOWN-LIMITATIONS.md` §2 |
| 7 | CHANGELOG/版本 `1.0.0-rc.1`；Windows 未签名声明；macOS 公证就绪（证书后） | `[x]` 版本；`[ ]` 公证 | 版本跨面一致：`Cargo.toml`、根 `package.json`、`apps/*/web/package.json`、`apps/desktop/tauri.conf.json`；CHANGELOG 见 `CHANGELOG.md`；Windows **不签名**（ADR-033）；macOS 公证待证书后启用（ADR-037） |
| 8 | 1.0.0 正式版 | `[ ]` | 需 `1.0.0-rc.1` 全量验收 + 远端 CI/release 实跑（tag `v1.0.0-rc.1` → 三平台包/镜像/SBOM/SHA256SUMS）通过后由用户确认 |
