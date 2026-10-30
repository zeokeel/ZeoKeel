# 2026-10-30-m2-enterprise-server · 验收

- [ ] 新环境 docker load + compose up + 管理员初始化 + 员工登录 + 对话扣费
- [ ] 管理台独立 Cookie session + CSRF；员工 JWT 调 admin 401/403；auditor 只读
- [ ] local/OIDC/LDAP 可切换；JWT EdDSA/kid/epoch/device/channel；refresh 重用检测
- [ ] 余额/冻结/峰谷/月发放/超额阻断；hold/pending usage/对账
- [ ] 网关仅 /v1/responses + /v1/models；未定价拒绝；SSRF 白名单
- [ ] bootstrap 模型/更新目录；技能/连接器不在 M2
- [ ] 审计哈希链/WORM ≥180 天；usage ≥12 个月；管理台权限矩阵；bootstrap 目录空数组也合法
- [ ] 企业客户端无 provider 设置入口（构建裁剪）；enterprise `zk compose --check` 绿
- [ ] 升级/回滚/备份恢复演练
- [ ] `zk db lock --check`、`zk api --check`、enterprise/personal `zk compose --check`、e2e 全绿


## P1 evidence (progress)

- `bash scripts/srv-migration-test.sh` → `migration ok: 17 tables`（真实 PG 16）。
- `bash scripts/srv-auth-test.sh` → `auth ok: login/me/refresh/reuse/admin-csrf/users-auditor/wallet-ledger/pricing/gateway`（真实 server + PG）。
- `/healthz`、`/readyz` 200；`schema_migrations(0001_init)` 写入。

## P3 admin SPA (progress)

- `apps/admin/web`（Vite/React）：登录、用户/角色、钱包调账、模型价格、审计、用量。
- `bash scripts/admin-e2e.sh`：真实 PostgreSQL + server + 4 Playwright tests（建用户/钱包/价格/审计/用量/axe）全绿。
- 服务端以 `/admin/` 提供静态资源；`ZEOKEEL_ADMIN_DIST` 可覆盖。

## P4 deployment artifacts (progress)

- `deploy/docker/Dockerfile.server`：多阶段 server + admin build。
- `deploy/docker-compose.yml`：postgres/server/caddy，6 键 env。
- `deploy/caddy/Caddyfile`、`deploy/portal/index.html`、`deploy/scripts/{backup,restore}.sh`。
- 本机无 Docker，**未执行镜像构建/compose up**；需在 CI/主机补验证。

## P2 operational jobs (progress)

- `POST /api/admin/jobs/{hold-expire|usage-pending|wallet-reconcile}/run`，`GET /api/admin/jobs`。
- 后台 5 分钟循环；`job_runs` 记录幂等键/状态/详情。
- 实测 `wallet-reconcile`、`wallet-grant` 触发成功；月发放幂等（同月重复不重复发放）。

## C3 server split

- `crates/zk-kit-server` + 8 个 `packages/server/*` 域包（zk.toml/契约/README/conformance）。
- `zk-srv-http` 仅 health/ready/migrations/router base；`apps/server` merge 各域 `routes()`。
- 真实 PG：`scripts/srv-auth-test.sh` 全链路通过；`scripts/admin-e2e.sh` 6 Playwright tests 通过。
- 管理台已拆包并由 `zk compose` 生成 registry；`apps/admin` 只保留 registry 组合。
- 迁移 SQL 已按域归包；`apps/server` 按 core→auth→users→wallet→pricing→gateway→audit→settings→jobs 顺序执行。

## Contract DTOs (progress)

- `zk-contract-auth/user/wallet` 已承载登录/refresh/建用户/调账请求 DTO；服务端域包复用这些契约，不重复定义。
- 已生成 `shared/contracts/schema/*.json` + `shared/contracts/ts/contracts.ts`；管理台登录/建用户/调账用 Ajv 校验。`bash scripts/schema-check.sh` 检查同步。

## Auth lockout

- client/admin 连续 5 次失败锁定 15 分钟；`scripts/srv-auth-test.sh` 验证第 6 次 429 `ACCOUNT_LOCKED`。
