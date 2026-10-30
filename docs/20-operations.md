# 20 · 运维、部署与 Day-0 开通（补充规范）

> 状态：Plan v3.2 补充规范。补齐 `06/09` 中"能装起来但不可运维"的缺口：
> **首次开通、备份/PITR、升级/回滚、监控告警、容量、故障 runbook、DR**。
> 默认支持拓扑 = 单机 docker compose；外部 PG/已有反代属于"高级部署"，不改变本节门禁。

## 1. Day-0 开通（从 `docker load` 到首轮对话）

### 1.1 部署方（客户 IT / 我方交付）

1. 准备主机：Linux x86_64，≥4 vCPU / 8 GB RAM / 100 GB SSD（PoC 最低 2C/4G）；Docker 24+ + compose v2。
2. 导入镜像：`docker load`（或 `zstd -dc ... | docker load`）；校验 `SHA256SUMS` + 镜像 digest。
3. 配置 `.env`（6 键，见 `09` §5）：`DOMAIN`、`TLS_MODE`、`ADMIN_PASSWORD`、`PG_PASSWORD`、`SERVER_IMAGE`、`MASTER_KEY`。
4. `docker compose up -d` → `/healthz`、`/readyz` → 门户 `https://DOMAIN/` 可访问。
5. 首登管理台：强制改密 → 完成初始化向导（§1.2）。
6. 交付记录：`VERSION`、`CHANNEL.json`、`SHA256SUMS`、SBOM、备份路径、管理员交接单。

### 1.2 管理台初始化向导（必须可执行、可重入）

| 步骤 | 必填 | 默认/校验 | 失败提示 |
| --- | --- | --- | --- |
| 1 管理员改密 | 是 | ≥12 位，非弱口令；写入 `must_change_password=false` | 弱口令/不一致 |
| 2 认证源 | 是 | local（默认）或 OIDC/LDAP；测试连接 | 元数据不可达/证书错误 |
| 3 上游 provider | 是 | base_url + key + 测试；key 加密存 `master.key` | 401/超时/模型列表空 |
| 4 模型目录 | 是 | 至少 1 个模型启用；默认模型 | 无可用模型 |
| 5 价格表 | 是 | 输入/输出/缓存价 + 峰谷倍率；未定价禁止启用 | 价格为 0/负/缺时段 |
| 6 月发放规则 | 是 | 全员、金额、accumulate/overwrite | 金额 ≤0 |
| 7 首个员工用户 | 可选 | 至少 1 个，用于验收 | 用户名冲突 |
| 8 连通性自检 | 是 | 网关 → 上游、门户、客户端清单、审计写入 | 任一项失败不完成 |

验收脚本：`scripts/day0-check.sh`【scaffold 后】（登录→bootstrap→一次非流式/流式对话→扣费→审计可查）；结果写 `evidence/day0-*.log`。

## 2. 拓扑与资源

| 服务 | 资源建议 | 说明 |
| --- | --- | --- |
| caddy | 0.5C/128MB | 唯一对外入口；TLS 自动/手动/internal |
| server | 2C/2GB 起 | 单进程；网关流式与 WASM 共享；按并发调 |
| postgres | 2C/2GB + SSD | 单机默认；外部 PG 需同版本/同迁移 |
| WASM 并发 | 每应用 1–2 实例 | 全局/每用户配额见 `24`/`07` |

端口：仅 80/443 暴露；server 仅 `expose 8787`；PG 不出宿主。数据卷：`pgdata`、`/data`（含 `master.key`、应用 SQLite、日志）。

## 3. 备份与恢复（必须可验证）

| 项 | 规则 |
| --- | --- |
| 备份内容 | PG（`pg_dump -Fc` + **WAL 归档/PITR**）、`/data`（WASM DB/配置/日志，**不含 master.key**）、`.env`（**剥离 MASTER_KEY**）、`VERSION/CHANNEL.json` |
| 密钥 | `master.key` 与 `.env` 中的 `MASTER_KEY` **单独离线保管，不进入备份包**；恢复时手工注入 |
| 加密 | 备份用 age/KMS 加密；**密钥与备份分离保管**；禁止 master.key 与密文同包 |
| 频率/保留 | 每日全量 + WAL 连续；保留 30 天；月度归档 12 个月（可配，不低于审计下限） |
| RPO/RTO | **暂不承诺**（D64）；记录每次备份/恢复实际耗时，持续改进 |
| 恢复演练 | 每月自动 restore 到临时 PG + 跑 smoke/对账；每季度人工演练；结果进 evidence |
| 恢复后 | 自动跑账本↔usage 对账、审计完整性校验；差异报告需人工确认 |
| 异地 | 可选：离线只读介质或客户已有 WORM；对象存储/KMS 属高级部署，不作为单机默认强制项 |

**无备份不升级**；备份不可读/大小异常 → 中止升级。

## 4. 升级与回滚（必须闭合）

### 4.1 服务端

```text
检查清单 → 备份（含 PITR 点）→ 下载 tar.zst → 校验 SHA256 + 签名 + 镜像 digest
→ docker load → 记录旧 digest（保留前 2 版）→ 改 SERVER_IMAGE → compose up -d
→ 迁移（expand/contract）→ /readyz → 门户/清单校验 → 首轮对话扣费自检
失败 → 回旧镜像（N-1 读兼容）→ 必要时恢复备份；compose 为 server 设 `stop_grace_period: 45s`，先停旧再起新
```

规则：

- 迁移必须 **expand/contract**：先加列/表，后切读写，最后清理；保证 N-1 镜像可读新 schema（至少一个版本）。
- `SERVER_IMAGE` 使用 `tag@digest`；旧镜像保留 2 个版本；更新清单发布前校验 digest。
- 回滚 != 降级 DB：跨迁移边界回滚必须恢复备份；文档/管理台明确提示。
- 客户端 1:1：服务端回滚时清单带 `previous`（上一版客户端）并允许受控降级；禁止把已升级客户端锁死。
- 升级只允许人工触发；禁止自动拉取/自动升级（ADR-041）。

### 4.2 客户端

见 `19` §5；强制更新失败必须可回滚；离线宽限由服务端标志控制。

## 5. 监控、告警、SLO

| 指标 | 告警阈值（默认） |
| --- | --- |
| `/readyz` | 连续 2 次失败 1 min |
| HTTP 5xx 比例 | >2% 5 min |
| 网关上游错误/超时 | >5% 5 min |
| 流式首 token P95 | >3 s 10 min |
| 钱包负余额 | 低于 `22` §4 的负余额下限或超过宽限期才告警（正常流式透支不报） |
| 审计写入失败 | 任意 1 次立即（fail-closed） |
| 备份失败/恢复校验失败 | 任意 1 次立即 |
| 磁盘使用 | >80% 警告，>90% 严重 |
| 证书到期 | <30 天警告，<7 天严重 |
| WASM 执行失败率 | >10% 10 min |

SLO：可用性 99.5%（单机默认）；网关 P95 < 5s（不含上游）；管理面 P95 < 1s。日志：tracing JSON + request_id/user_id（脱敏）；保留 30 天（审计另存）。

## 6. 容量与压测

- 基线：100 员工、20 并发对话、10 并发 WASM；给出 CPU/内存/DB 连接/磁盘增长曲线。
- 压测场景：登录风暴、流式长连接、并发扣费、WASM 冷启动、审计写放大。
- 容量门禁：M2 出 100 用户基线；M6 出 500 用户建议配置；超配需拆分/外部 PG。
- 增长：`usage`/审计按月分区或归档；WASM 每应用 100MB 上限；磁盘告警联动清理策略。

## 7. 故障 Runbook（每项含症状/诊断/处置/回滚）

| 症状 | 诊断 | 处置 |
| --- | --- | --- |
| 502/门户不可用 | `compose ps`、caddy 日志、`/healthz` | 重启 caddy/server；证书/TLS 模式检查 |
| `/readyz` 失败 | DB 连接、迁移状态、包 Active | 恢复 DB/回滚镜像；看启动日志 |
| 登录 401 风暴 | token_epoch、时钟、refresh 轮换 | 校时；吊销设备；检查 IdP |
| 网关全 5xx | 上游 key/额度/base_url、DNS | 切备用 provider；未定价检查 |
| 余额异常 | 账本↔usage 对账、hold 泄漏 | 冻结用户、人工调账（审计）、修 hold |
| 审计写失败 | DB 磁盘/权限/事务 | 业务 fail-closed；恢复后补写？禁止补写，记差异 |
| 升级后起不来 | 迁移/镜像/配置 | 回旧镜像；跨迁移恢复备份 |
| 备份恢复失败 | 备份完整性/密钥 | 用异地副本；升级前必须演练 |

## 8. 安全硬化（部署侧）

- 非 root（uid 10001）、只读根文件系统（除 `/data`）、`no-new-privileges`、cap_drop ALL。
- 主机防火墙只开 80/443；SSH 仅密钥；fail2ban；Docker socket 不挂载。
- 管理台仅内网/VPN（可配）；`/admin` 限速；CSP/HSTS；Cookie Secure/HttpOnly/SameSite。
- 日志/备份不含明文 key；`.env` 0600；`master.key` 0600 + 离线备份。
- 定期 `docker scout`/镜像扫描；基础镜像按 digest 固定。

## 9. 证据与验收

- Day-0 脚本输出、备份/恢复日志、升级/回滚演练、压测报告、告警演练、**内部安全评审报告**。
- 全部进 `evidence/` 或 CI artifact；无证据不算完成（`11` §5）。
- M6 退出：完成一次"备份→升级→回滚→恢复"全链路演练并留证。
