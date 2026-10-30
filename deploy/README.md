# ZeoKeel 部署（M2 P4 初版）

##  prerequisites
- Linux x86_64，Docker 24+ / compose v2。
- 6 个必填 `.env` 键：`DOMAIN`、`TLS_MODE`、`ADMIN_PASSWORD`、`PG_PASSWORD`、`SERVER_IMAGE`、`MASTER_KEY`（`docs/06` §5）。
- `MASTER_KEY` 单独离线保管，不进入备份包。

## 启动
```bash
cp deploy/.env.example .env && chmod 600 .env   # 填入 6 键
docker compose -f deploy/docker-compose.yml up -d
curl -fsS https://$DOMAIN/healthz
curl -fsS https://$DOMAIN/readyz
open https://$DOMAIN/admin/
```

## 构建镜像
```bash
docker build -f deploy/docker/Dockerfile.server -t zeokeel/server:0.1.0 .
```
镜像内包含 `zk-server` 与 admin SPA；migrations 已嵌入二进制。

## 备份 / 恢复
```bash
bash deploy/scripts/backup.sh
bash deploy/scripts/restore.sh ./backups/<stamp>
```
`master.key` 与 `.env` 的 `MASTER_KEY` 不进入备份包；恢复后手工注入。

## 升级 / 回滚
1. 备份；2. `docker load` 新镜像；3. 改 `SERVER_IMAGE`；4. `compose up -d`；5. 验证 `/readyz`。
失败：回旧 `SERVER_IMAGE`，必要时恢复 PG 备份。

## 已知限制（M2 初版）
- Caddy 自动 HTTPS；内网/自定义 TLS 需按现场改 Caddyfile。
- 管理台 SPA 目前由镜像 `ZEOKEEL_ADMIN_DIST` 提供，尚未 `include_dir!` 内嵌。
- 备份脚本为冷备份，PITR/对象存储后置。
- Docker 镜像未在 CI 构建；需补 `docker build` 门禁。
