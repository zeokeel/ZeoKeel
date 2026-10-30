# 34 · 运维 Runbook（M6）

- **部署**：`deploy/docker-compose.yml` + `.env`（master.key 离线保管）；`/healthz`、`/readyz`。
- **升级**：先备份（`scripts/backup-restore.sh`）→ 导入镜像 → compose up → readyz → smoke；失败回滚上一镜像。
- **备份**：PG `pg_dump -Fc`/WAL、`/data`（不含 master.key）、`.env` 脱敏、`VERSION/CHANNEL.json`；加密且密钥分离；保留 30d/月归档 12m。
- **恢复**：空库/旧库恢复→迁移→启动→登录/业务 smoke→账本↔usage 对账→审计链校验；记录耗时。
- **故障**：DB 不可用 readyz 非 200；审计写失败 fail-closed；不要跳过迁移或手改 audit_log。
- **日志**：不落 token/key/secret/会话正文；审计/用量保留期不低于 180d/12m。
