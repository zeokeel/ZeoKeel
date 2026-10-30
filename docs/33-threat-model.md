# 33 · 威胁模型（M6）

> 范围：企业版单租户部署；资产/信任边界/P0 风险/缓解/测试。实现门禁见 `21` §10，验收见 M6 spec。

## 1. 资产

- 身份：JWT access/refresh、admin cookie/CSRF、OAuth token（客户端 keychain）。
- 钱：wallet 账本、usage、pricing、hold；审计链。
- 内容：技能 zip、连接器配置（无静态 secret）、WASM 应用/DB、更新清单/安装包。
- 密钥：`MASTER_KEY`、JWT 签名、上游 provider key、更新签名（规划）。
- 会话/工作区：客户端本地 SQLite/文件，服务端不存会话正文（网关不落库由实现保证；不得对外宣称除非已验证）。

## 2. 信任边界

1. 客户端机器/用户：工具/命令/文件/网络由员工机器与客户网络负责（弱管控，D1/D4）；服务端不兜底。
2. 服务端进程：认证/钱/商店/审计/网关准入；上游 provider key 只在服务端。
3. WASM guest：不可信；服务端 wasmtime 沙箱，无网络/文件/AI/他 app DB；每 app SQLite。
4. 构建输入：源码+锁文件+评审+可复现构建；OEM 私有索引仓/submodule pin。

## 3. P0 风险与缓解

| 风险 | 缓解 | 测试 |
| --- | --- | --- |
| 认证绕过/JWT 重放 | 每请求 token_epoch/设备/角色；refresh reuse 检测；admin CSRF | auth 单测/e2e |
| CSRF/越权 | admin cookie+CSRF；权限矩阵；内容 owner/grants | admin/e2e |
| 网关 SSRF/密钥泄漏 | 只 Responses；上游 URL/模型服务端固定；客户端不持 key | 网关 e2e/配置检查 |
| zip 穿越/炸弹/链接 | 客户端安全解压；服务端只元数据；路径/链接/大小拒绝 | kit/desktop 负向 |
| WASM 逃逸/DoS | 无网络/文件；SQL DML 闸门；fuel/time/memory/并发/DB 配额；host-managed schema | kit/apps e2e/负向 |
| 命令注入 | stdio 只允许无 secret 配置；管理台输入校验；不拼接 shell | 校验/审计 |
| 更新劫持 | 企业版只登录服务端 1:1；channel/edition/version 校验；SHA | updates e2e |
| 审计篡改 | 哈希链；保留期；审计写失败 fail-closed | audit 测试/retention |
| 供应链 | 锁文件/deny/audit/SBOM/镜像 digest；OEM pin | security-audit/CI |
| 隐私 | 不落会话正文；日志/错误/审计脱敏；隐私口径不夸大 | grep/负向 |

## 4. 明确非目标

- 不做工具白名单/网络限制/企业审批/部门（ADR-030/032）。
- 不做 Windows 签名（ADR-033）；macOS 证书后配。
- 不做多租户/第三方安全认证（D60）。

## 5. 残余风险

- 弱管控下员工行为风险由客户承担；服务端只记录可观测审计。
- 无 Docker/证书/平台 runner 的交付项未验证；M6 acceptance 标 UNVERIFIED。
