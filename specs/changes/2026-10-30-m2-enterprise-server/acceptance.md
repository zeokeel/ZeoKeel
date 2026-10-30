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
- 本地 Playwright 实测：登录 → 创建 e2e-user → 钱包调账 → 保存价格；axe serious/critical = 0。
- 服务端以 `/admin/` 提供静态资源；`ZEOKEEL_ADMIN_DIST` 可覆盖。
