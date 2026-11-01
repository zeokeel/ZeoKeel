# ZeoKeel 部署（单机 compose，M2/M5 初版）

> 文档真源：`docs/06` §5（配置）、`docs/09` §5（交付物/镜像）、`docs/20` §2–§4（拓扑/备份/升级）。
> 本目录只含**可执行**部署文件；镜像由 `deploy/docker/Dockerfile.server` 构建（Lead 维护）。

## 1. 前置条件
- Linux x86_64，Docker 24+ / Compose v2。
- **6 个必填 `.env` 键**（`docs/06` §5）：`DOMAIN`、`TLS_MODE`、`ADMIN_PASSWORD`、`PG_PASSWORD`、`SERVER_IMAGE`、`MASTER_KEY`。
  `docs/09` §5 把 `MASTER_KEY` 标为“推荐”，而 `docs/06` §5 与 `deploy/docker-compose.yml` 按必填处理（不静默降级）。
- `MASTER_KEY` / `master.key` 单独离线保管，不进入备份包（`docs/20` §3）。

## 2. 启动（在 `deploy/` 目录内执行；compose 默认读取 `deploy/.env`）
```bash
cd deploy
cp .env.example .env && chmod 600 .env     # 填 6 键；SERVER_IMAGE 用 tag（交付时用 tag@digest）
mkdir -p data caddy/certs                 # /data 持久卷 + manual TLS 证书
sudo chown -R 10001:10001 data             # 容器内 uid 10001
docker compose config >/dev/null           # 先做一次配置校验
docker compose up -d
curl -kfsS https://$DOMAIN/healthz         # TLS_MODE=internal 时用 -k
curl -kfsS https://$DOMAIN/readyz
open https://$DOMAIN/admin/
```
- 端口：只有 caddy 发布 `80/443`；server 仅内部 `8787`（`expose 8787`，与二进制一致；`docs/20` §2 的 8080 是历史写法）。
- `/data`（`./data` bind）保存应用 SQLite/日志，**master.key 也应放这里并离线备份**；客户端更新清单默认来自**镜像内嵌**的 `/srv/updates`（服务端 `/portal` 可直接下载企业版客户端）；如需热替换更新面，设 `UPDATE_DIR=/data/updates` 并把目录只读挂载到该路径（`deploy/enterprise-updates/README.md`）。
- `TLS_MODE=auto|manual|internal` 由 `deploy/caddy/entrypoint.sh` 选择 `Caddyfile.auto|manual|internal`；`manual` 需要 `deploy/caddy/certs/{fullchain,privkey}.pem`。

## 3. 构建镜像
```bash
# 在仓库根目录执行（Dockerfile 内会 compose enterprise×official；OEM 渠道需先修正 Dockerfile 的 channel 参数）
docker build -f deploy/docker/Dockerfile.server -t zeokeel/server:0.1.0-official .
```
镜像内含 `zk-server` + admin SPA；migrations 已嵌入二进制。OEM 镜像的品牌注入尚未在 Dockerfile 中接通（见 `target/evidence/audit/build.md` P1）。

## 4. 备份 / 恢复（脚本自带 `cd deploy`，可直接从任意目录调用）
```bash
cd deploy
bash scripts/backup.sh                      # -> ./backups/<stamp>/{zeokeel.dump,data.tgz,env.redacted,README.txt}
bash scripts/restore.sh ./backups/<stamp>   # 需先注入 MASTER_KEY 到 .env
```
- 备份 = `pg_dump -Fc` + `/data`（排除 `master.key`）+ 脱敏 env；**PITR/WAL 归档仍是 UNVERIFIED**（`docs/20` §3 要求，脚本输出会显式标注）。
- 恢复后自动等待 `/readyz`；`master.key` 不随备份恢复，必须手工放回（离线保管）。

## 5. 升级 / 回滚（人工，`ADR-041`）
1. `bash scripts/backup.sh`；2. 下载 `zeokeel-enterprise-<ver>-official.tar.gz` 并校验 `.sha256`；3. `docker load`；
4. 修改 `.env` 的 `SERVER_IMAGE`（建议 `tag@digest`）；5. `docker compose up -d server caddy`；
6. 验证 `/readyz`、门户、管理台；7. 失败：改回旧 `SERVER_IMAGE` 并 `compose up -d`，跨迁移回滚用备份恢复。
- 真实演练：`bash scripts/upgrade-drill.sh --docker --dist dist/<ver>/<channel>`（仓库根目录执行；无 Docker 时明确 `exit 2 UNVERIFIED`）。

## 6. 本地无 Docker 时的验证边界
- `docker compose config`、镜像构建/运行、Caddy TLS、备份恢复实跑均标注 **UNVERIFIED**；本仓库只做语法/静态校验（见 task 报告）。
- 已知限制：Dockerfile 目前只 compose enterprise×official；OEM 镜像品牌注入、三平台安装包、notarize 仍需发布环境完成。

## 7. 已知限制（M2 初版）
- 管理台 SPA 由镜像 `ZEOKEEL_ADMIN_DIST` 提供，尚未 `include_dir!` 内嵌。
- 备份为冷备份（pg_dump + /data tar），PITR/对象存储后置。
- 镜像 digest 尚未钉死（`docs/09` §4 要求交付时用 `tag@digest`）。

## 8. 更新面夹具 `deploy/updates/`（测试专用，不是交付物）
- `deploy/updates/` 是**提交进仓库的更新面夹具**：`clients/ZeoKeel_0.1.0_linux_x64.AppImage` 是 18 字节占位，
  `CLIENT-RELEASE.json` 故意登记该占位的 SHA-256。`scripts/updates-e2e.sh` 用它验证 manifest/渠道拒绝/版本拒绝/门户；
  `scripts/upgrade-drill.sh` 在无服务端时把它作为本地 fallback manifest。
- **不得把它当交付物**：`scripts/build-dist.sh` 对位于 `deploy/`（以及 `oem/`、`*/fixtures/`）的输入一律判为
  **INVALID（exit 1）**，只有真实输入才写 `CLIENT-RELEASE.json`/`SHA256SUMS`；`--self-test` 固定覆盖
  “`--artifacts deploy/updates/clients` 必须 exit 1” 的负向用例。发布 dist 由 `.github/workflows/release.yml`
  用三平台真实安装包 + `docker save` + `zk sbom` + `KNOWN-LIMITATIONS.md` 组装。
- 夹具版本固定 `0.1.0`，与发布版本解耦；不要把真实发布产物拷进该目录。详见 `deploy/updates/FIXTURE.md`。
