# 2026-10-30-hardening-release · 设计（M6）

- 状态：`draft`

## 1. 脚本/证据

- `scripts/security-audit.sh`：静态规则（secret/unsafe/unwrap/权限/路由）+ `cargo deny/audit`/`pnpm audit`（存在则强制，缺失记 UNVERIFIED）+ threat-model 文档检查；输出 `target/evidence/m6/security.json`。
- `scripts/backup-restore.sh`：临时 PG+data 目录；pg_dump -Fc、WAL 占位、/data tar、.env 脱敏、CHANNEL/VERSION；恢复→迁移→server→login/业务 smoke；审计链/对账。
- `scripts/fault-injection.sh`：进程级 kill -9/PG stop/磁盘满模拟（tmpfs/`fallocate`）/审计失败（revoke insert?）；每项断言；Docker 缺失标 unverified。
- `scripts/perf-budget.sh`：启动/ready、health p95、bundle size、mock 首 token；输出 JSON+阈值。
- `scripts/reproducible-build.sh`：env 固定、两次构建、sha256 比较；覆盖 zk/zk-server/zk-desktop/admin dist；镜像 tar 无 Docker 标 unverified。
- `e2e/m6-a11y.spec.ts`：复用 admin/client/portal server，逐页 axe/Tab/Escape/⌘K/reduced motion/zh-en。

## 2. 文档/CI

- 新增 docs/29–32、CHANGELOG、CONTRIBUTING；根 README 更新；弱管控声明。
- `.github/workflows/security.yml`：security-audit、reproducible、axe；Docker/证书 job 条件化并标 UNVERIFIED。
- 1.0 checklist 绑定 M0–M5 证据路径。

## 3. 测试/边界

- 无 Docker/证书/平台 runner：镜像/安装包/公证/PITR 标 UNVERIFIED；脚本仍可运行并输出该状态。
- 独立安全评审 audit-arch R1/R2。
