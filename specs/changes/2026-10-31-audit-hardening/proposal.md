# 2026-10-31-audit-hardening · 提案

- 状态：`done`
- Owner：lead（Agent Teams：auditor-rust/web/build/rig + fixer-server/contracts）
- 日期：2026-10-31
- 关联：ADR-064/065/066；docs/38；docs/05/06/10/24
- 评审：auditor 四路只读审计报告见 `target/evidence/audit/`（用户指令优先，C19 记录在验收）
- 变更预算：跨域一次性加固（用户明确要求"循环审计直到完美"）；按域提交 8 个 Conventional Commits

## 1. 背景 / 问题

用户要求重新评审"代码是否按文档施工完成"，并给出核心目标：企业插件/服务端对接完成、整个 agent 功能完美、rig 所有能力支持、所有项目可配置。四路只读审计发现：已提交生成物基线违反 docs/24 §1.2（enterprise 而非 personal）、桌面默认组合仍是 M0 agent-loop/llm-mock（rig 未发货）、企业网关无法用 rig、内核 zeokeel-cordis 未接入、IPC 边界仍是手写镜像（T22 未完成）、多处门禁 false-green（fmt/capability/web lint/unit、release/deploy 脚本）、网关流式 usage=0 与客户端可控计费幂等键等 P0。

## 2. 目标

- 生成物基线回归 personal×default×official，`zk compose --check`/capability/docs/db lock 全绿。
- 两端 boot/shutdown 经 `zeokeel-cordis`（新 `zk-bootstrap`）。
- 个人/企业默认 agent.loop = agent-rig；企业经网关 rig Responses 适配（流式/工具/usage）。
- T22 后端：全部边界 DTO 生成 JSON Schema + TS interfaces + `SCHEMAS`（53 schema）。
- 服务端计费/认证/审计/迁移安全修复（usage 结算、服务端幂等键、fail-closed 审计、checksum 校验、DB 路由）。
- web 门禁真实化（26 包 lint/typecheck/test:unit），C14 渠道地址 UX，a11y/i18n 修复。
- release/deploy 不再 false-green；真 e2e（login/admin/agent/M1）可复现通过。

## 3. 非目标

- 不实现 `zk api` 路由清单（server `[[routes]]` 真源）与 `zk lock/verify-lock/metadata snapshot`；不实现媒体 provider（image/audio/transcribe/rerank）。
- 不接系统密钥库（M1b 仍为后续）；不做 Docker/SBOM/三平台安装包实测（环境无 Docker/签名）。
- 不改 edition/OEM 业务范围。

## 4. 用户与场景

| 用户 | 场景 | 期望 |
| --- | --- | --- |
| 个人用户 | 安装默认客户端聊天/工具/审批 | rig agent 真实多步工具、审批、取消、usage |
| 企业员工 | 登录网关后聊天/技能/连接器/应用 | 登录/刷新/bootstrap/网关工具调用可用，C14 地址锁定 |
| 管理员 | 管理台/计费/审计 | 真实 PG e2e、fail-closed 审计、可配置峰谷 |
| 开发者/CI | 门禁与生成物 | 一条命令复现全绿，无 false-green |

## 5. 成功指标

- Rust：fmt/clippy/test 全绿；zk 6 门禁全绿；web 4 门禁全绿；docs/spec/review/schema 绿。
- e2e：login 4/4、admin 11/11、agent 9/9（连续两次）、M1 14 passed/15 skipped。
- 生成物：compose personal --check / capability / docs / db lock 全 up-to-date。

## 6. 风险与未知

| 风险/未知 | 影响 | 缓解/验证 |
| --- | --- | --- |
| 大规模跨域改动 | 回归 | 分域提交 + 全量门禁 + 四套 e2e |
| Docker/SBOM/多平台不可验证 | 交付证据缺失 | 明确 UNVERIFIED，脚本不再 false-green |
| 契约新增 DTO/依赖 | 兼容性 | serde default、只增字段、schema/TS 生成校验 |

## 7. 需要用户拍板

- 无阻塞项；后续 backlog 见 acceptance §3（T22 前端 Ajv、zk api、媒体能力、provider key 加密）。
