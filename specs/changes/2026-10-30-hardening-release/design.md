# 2026-10-30-hardening-release · 设计（M6）

- 状态：`draft`

## 1. 脚本/证据

- `scripts/security-audit.sh`：静态规则（secret/unsafe/unwrap/权限/路由）+ `cargo deny/audit`/`pnpm audit`（CI 缺失=红；本地缺失=UNVERIFIED 且不能通过）+ threat-model 文档检查；输出 `target/evidence/m6/security.json`。
- `scripts/backup-restore.sh`：临时 PG+data 目录；`pg_basebackup -Ft -X stream` + WAL 归档目录、`/data` tar、`.env` 脱敏、CHANNEL/VERSION；恢复用 basebackup + `recovery.signal`/`restore_command`（`ZEOKEEL_PITR=1`），否则 `pg_dump -Fc` 并标 PITR UNVERIFIED；恢复→迁移→server→login/业务 smoke→审计链/对账。
- `scripts/fault-injection.sh`：进程级 kill -9 迁移中/事务中、PG stop、磁盘满（tmpfs/fallocate）、审计失败（`REVOKE INSERT ON audit_log` 后业务请求必须 fail-closed，恢复权限后成功）；每项断言账本/审计/迁移一致性；Docker 缺失标 unverified。
- `scripts/perf-budget.sh`：启动/ready、health p95、admin/desktop bundle、镜像体积（本地 tar 或 UNVERIFIED）、mock 首 token、网关 p50/p95（记录字段）；输出 JSON+阈值。
- `scripts/reproducible-build.sh`：env 固定、两次构建、sha256 比较；覆盖 zk/zk-server/zk-desktop/admin dist；镜像 tar 无 Docker 标 unverified。
- `e2e/m6-a11y.spec.ts`：复用 admin/client/portal server，逐页 axe/Tab/Escape/⌘K/reduced motion/zh-en。

## 2. 文档/CI

- 新增 docs/29–32、CHANGELOG、CONTRIBUTING；根 README 更新；弱管控声明。
- `.github/workflows/security.yml`：安装并强制 `cargo-deny`、`cargo-audit`、`cargo-machete`、`cargo-geiger`、`trivy`、`pnpm audit --prod`；security-audit/reproducible/axe；Docker/证书 job 条件化并标 UNVERIFIED。
- 1.0 checklist 绑定 M0–M5 证据路径。

## 3. 测试/边界

- 无 Docker/证书/平台 runner：镜像/安装包/公证/PITR 标 UNVERIFIED；脚本仍可运行并输出该状态。
- 独立安全评审 audit-arch R1/R2。
