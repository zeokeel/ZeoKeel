# 2026-10-30-m2-enterprise-server · 提案

- 状态：`approved`（C19 R7 APPROVED）
- Owner：lead + 多 teammate
- 日期：2026-10-30
- 关联：docs/06、docs/07、docs/12 M2、docs/04、docs/13 ADR-028/030/031/032/041/042/044
- 评审：`docs/reviews/2026-10-30-m2-plan.md`（修订后复评）
- 变更预算：按 P1–P4 分片；每片独立可运行/可回滚

## 1. 背景 / 问题

M1 个人版客户端已可运行（workspace 会话、流式 agent、工具、设置、e2e）。M2 需要企业版弱管控配套：
认证/JWT、用户/角色、钱包/配额、AI 网关（仅 Responses）、模型目录/更新通道 bootstrap、审计、管理台、docker 交付、
企业客户端登录/bootstrap/网关 provider。当前服务端只有 `/healthz` 骨架。

## 2. 目标（M2 增量）

- 服务端：axum + PostgreSQL 单进程；迁移不可变；auth（local/OIDC/LDAP 单主源 + 应急超管）、JWT access/refresh、token_epoch 即时失效。
- 用户/角色：超管/审计员/员工；无部门/用户组；内容可见性全员/指定用户。
- 钱包：单人余额一本账、月发放、管理员充值、峰谷价、超额硬阻断（流式允许透支）。
- 网关：只 `POST /v1/responses`，上游 key 只在服务端，未定价模型拒绝，结算+usage 同事务。
- 模型目录/更新通道 bootstrap；技能/连接器完整中心属 M3（M2 不做）。
- 审计/用量：≥180 天/≥12 个月；管理操作/登录/审批/下发全链路。
- 管理台：登录、用户/角色、provider/峰谷价、用量、系统配置；构建期包化。
- 部署：多阶段 Dockerfile + compose + Caddy + 门户；备份/恢复脚本；健康/就绪。
- 企业客户端：登录/刷新、bootstrap 目录、网关 provider（无自配 key/base_url）、未登录态；构建期裁剪个人 provider 设置。

## 3. 非目标

- 不做部门/用户组/企业审批/工具白名单/网络限制（C6/C8）。
- 不做多租户、服务端 agent/技能执行、MCP 代理、S3/Redis/MQ。
- 不做完整计费开票/收款；余额=企业配额。
- 不做 OIDC/LDAP 真实 IdP 联调以外的高级 SCIM；不做 DPA。

## 4. 成功指标

- 新环境 `docker load` + `compose up` + 初始化管理员 + 员工登录 + 一轮对话扣费（e2e）。
- 余额不足网关明确拒绝；JWT 刷新/吊销正确；三种认证源可切换。
- 企业客户端无 provider 设置入口（bundle/设置快照验证）。
- 目录下发变更后客户端选项变化；员工不能本地绕过网关。
- 管理台改配置即时生效/明确重启；升级/回滚演练通过。
