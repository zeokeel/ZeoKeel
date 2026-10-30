# 2026-10-30-hardening-release · 规格（M6）

- 状态：`approved`
- 依据：`docs/12` M6、`docs/21` §9–§10、`docs/20` §3/§4/§8/§9、`docs/11`、`docs/18`、`docs/09` SBOM、D60–D65

## 1. 安全门禁（docs/21 §10，13 项）

每项必须有命令/证据/owner/结论；`scripts/security-audit.sh` 输出 JSON `{rule,severity,file,line,status,owner}`，0 高危才通过：

1. 威胁模型评审：资产/信任边界/P0 风险/缓解/测试映射 `docs/21` §3–§8。
2. 认证绕过 / JWT 吊销 / refresh reuse / CSRF / 越权矩阵用例。
3. SSRF（网关/连接器/资产 URL）/ rebinding 用例；仅允许白名单出口。
4. zip 穿越/炸弹/链接（客户端）与 WASM 逃逸（无 AI/网络/文件/他 app DB）用例。
5. 命令注入/stdio 参数/env 白名单；管理台输入校验。
6. 密钥生命周期演练：master.key 恢复、JWT 轮换、更新签名轮换、OAuth token 失效。
7. 审计哈希链 + WORM 锚定验证；钱审计矩阵（充值/调账/结算/退款/月发放）无缺项。
8. 依赖审计：`cargo deny check`、`cargo audit`、`pnpm audit --prod`；锁定版本；高危=0。
9. SBOM：每个产物（server 镜像/client/dist/OEM 包）CycloneDX（cargo+pnpm+OEM），深度 OEM 包必须在 SBOM。
10. 镜像扫描（docker scout/trivy）无高危；基础镜像按 digest；`.dockerignore` 不含密钥。
11. 隐私口径：不声称“服务端不读取会话”除非网关不落库已验证；审计/日志脱敏。
12. 日志/错误响应/审计 payload/崩溃 dump 无 token/key/secret/password/会话正文；结构化字段白名单。
13. 弱管控声明写入 `docs/00`/`docs/21`；销售材料不得宣称限制员工。

## 2. 备份/恢复/升级/回滚全链路

- 备份内容（docs/20 §3）：PG `pg_dump -Fc` + WAL/PITR；`/data`（WASM DB/配置/日志，不含 master.key）；`.env` 剥离 `MASTER_KEY`；`VERSION/CHANNEL.json`；备份用 age/KMS 加密，密钥与备份分离。
- `scripts/backup-restore.sh`：
  - 备份→加密→临时 PG/空目录恢复→迁移→启动→`/readyz`→登录/业务 smoke→账本↔usage 对账→审计链校验；
  - 恢复 master.key 手工注入验证；记录耗时（RPO/RTO 未承诺 D64）；
  - 失败场景：备份不可读/大小异常→中止升级。
- `scripts/upgrade-drill.sh --full`：备份→升级（同版本镜像/占位）→回滚→恢复全链路；无 Docker 时进程级但标记 UNVERIFIED。
- 故障注入 `scripts/fault-injection.sh` 矩阵：kill -9 迁移中/事务中、PG crash、OOM、磁盘满、网络分区、审计写失败 fail-closed；每项断言账本/审计/迁移无损坏；无 Docker 标 unverified。

## 3. a11y/键盘/i18n/性能

- 页面矩阵：admin（登录/用户/钱包/价格/审计/用量/作业/技能/连接器/应用/设置）、client（登录/首配/会话/聊天/设置/技能/连接器/应用/错误页）、portal；每页 axe 0 serious/critical + Tab/Escape/⌘K/focus trap/reduced motion/对比度 + zh/en key 覆盖。
- `scripts/perf-budget.sh` 阈值：
  - server ready ≤2s（本地 PG）、`/healthz` p95 ≤50ms；
  - admin bundle ≤400KB gzip、desktop shell ≤500KB gzip；
  - mock 首 token ≤300ms（本地）；网关延迟记录 p50/p95（不设硬门禁，无真实上游）；
  - 500 用户容量：列表/登录压测报告，超过阈值记 issue。
- 阈值不达标=红，修复或 ADR 记录降级。

## 4. 可复现/供应链/1.0

- `scripts/reproducible-build.sh`：固定 rust/node/pnpm 版本、`--locked`、`SOURCE_DATE_EPOCH`、`--remap-path-prefix`、`-C build-id=none`；覆盖 `zk`/`zk-server`/`zk-desktop`/admin dist/镜像 tar；两次冷构建 sha 必须一致；非确定项必须 ADR 消除，不接受“记录即过”。
- SBOM/license：CycloneDX 每产物；license allowlist/deny；deep OEM 包；`cargo deny licenses`。
- 1.0：`CHANGELOG.md`、root README 10 秒入口、`docs/34-runbook`、`docs/35-api`（生成契约摘要）、`docs/36-troubleshooting`、`docs/37-release-checklist`、known limitations、版本决策 1.0.0。
- 环境阻塞（Docker/证书/平台 runner）标 UNVERIFIED；不得写 complete。

## 5. 验收断言

- `bash scripts/security-audit.sh`：0 高危，JSON 证据。
- `bash scripts/backup-restore.sh`：pass；`bash scripts/upgrade-drill.sh --full` pass 或明确 UNVERIFIED 子项。
- `bash scripts/fault-injection.sh`：矩阵 pass/标 unverified。
- axe 全页矩阵 + 键盘 + i18n 证据；`perf-budget.sh` 阈值；`reproducible-build.sh` 两次一致。
- SBOM/license；docs-check/spec-check/review-check；独立评审 APPROVED。
