# 33 · 威胁模型（M6 / 1.0.0-rc.1）

> 范围：企业版单租户部署 + 个人版客户端；资产/信任边界/1.0 攻击面/P0 风险/缓解/可执行测试/残余风险。
> 门禁真源：`21` §10（13 项 + 拆分 5b）；可执行实现：`scripts/security-audit.sh`；证据：`target/evidence/release-1.0-rc1/security2/`。
> 独立评审：`docs/reviews/2026-10-31-threat-model.md`（round-2 修复后由评审者复评并在该文件写 `- 威胁模型评审结论：APPROVED` 才算通过；在此之前 gate 1 为 `unverified`）。

## 1. 资产

- 身份：JWT access/refresh、admin cookie/CSRF、OAuth token（客户端 keychain）。
- 钱：wallet 账本、usage、pricing、hold；审计链。
- 内容：技能 zip、连接器配置（无静态 secret）、WASM 应用/DB、更新清单/安装包。
- 密钥：`master.key` 密钥环（上游 provider key 的 AEAD 加密）、JWT 签名、更新签名（规划）、测试 fixture key（非真实）。
- 会话/工作区/提示词：客户端本地 SQLite/文件；服务端网关只内存转发，不落 prompt/completion（`21` §8.1）。
- Agent 输出/工具结果：进入客户端 UI/本地文件，按本地审批与工作区根治理。

## 2. 信任边界

1. 客户端机器/用户：工具/命令/文件/网络由员工机器与客户网络负责（弱管控，C6/ADR-030）；服务端不兜底。
2. 服务端进程：认证/钱/商店/审计/网关准入；上游 provider key 只在服务端，且以 `master.key` AEAD 加密落库。
3. 上游 provider：不可信网络对端；只能 `https://` 公网 host（显式本地 dev flag 例外），重定向不跟随。
4. WASM guest：不可信；服务端 wasmtime 沙箱，无网络/文件/AI/他 app DB；每 app SQLite。
5. 构建输入：源码+锁文件+评审+可复现构建；OEM 私有索引仓/submodule pin。

## 3. 1.0 攻击面（相对 M0 新增）

| 面 | 新增能力 | 主要威胁 |
| --- | --- | --- |
| 客户端工具 | terminal 多会话、pwsh-persistent、agent-team（coordinator + worker）、ralph/goal 轮次 | 命令/参数注入、会话间越权、worker 放大/hook 泄漏、输出外泄 |
| 客户端配置 | hooks/JSONL、memory、web-search endpoint（`AppSettings.*`） | 配置注入、搜索端点 SSRF/私网、密钥回显到 UI/日志 |
| 内容分发 | 技能/连接器元数据安装（客户端执行）、WASM 应用 | zip 穿越/炸弹/链接、恶意 guest 资源耗尽、元数据欺骗 |
| 企业网关 | 上游 provider CRUD（admin）、responses 转发、流式结算 | SSRF/redirect 到内网/元数据、key 泄漏、重复计费 |
| 管理台 | SPA + cookie/CSRF + 权限矩阵 | CSRF/越权、输入校验绕过 |
| 密钥 | `master.key` 版本化密钥环 | 密钥文件权限/轮换错误、密文篡改、缺 key 静默降级 |
| Agent 运行时 | rig 多步、工具审批、hook 事件 | prompt injection 诱导工具外发、审批绕过、输出泄漏 |
| 更新/交付 | 企业 1:1 更新、个人版 GitHub Releases、镜像升级 | 更新劫持、占位/伪造产物、未签名/未公证环境 |

## 4. P0 风险、缓解与可执行测试（映射 gate）

| 风险 | 缓解 | 可执行测试 / gate |
| --- | --- | --- |
| 认证绕过 / JWT 重放 / 禁用后旧 token 可用 | 每请求 `token_epoch`；refresh reuse family 吊销；admin cookie+CSRF；失败锁定 | `bash scripts/srv-auth-test.sh`（gate 2，真实 PG） |
| CSRF / 越权（admin/员工/审计员） | 独立 admin 会话 + CSRF；RBAC/权限矩阵；404 vs 403 | gate 2 |
| **上游 SSRF（P0-1，已修）**：scheme-only 校验 + reqwest 默认跟随重定向 → 被允许的 https 可 302 到内网/元数据 | 严格 `base_url` 校验（https-only；loopback http 仅 `ZEOKEEL_ALLOW_INSECURE_UPSTREAM=1`；拒绝 userinfo/私网/loopback/link-local/metadata/local/internal/数字混淆 host）；`reqwest` `redirect(Policy::none())`；provider key 加密 | `cargo test -p zk-srv-gateway --lib -- security_round2_tests`（矩阵 + redirect no-follow）+ `security-audit` gate 3 provider CRUD 探针 + serve Host/Origin/未认证 IPC 探针 |
| 上游 key 明文落库（P1-3，已修） | `crates/zk-kit-server::master_key` AES-256-GCM 版本化密钥环（`vN:<b64 nonce||ct>`）；新写加密、缺 key fail-closed、旧明文只读+告警；轮换离线演练 | `cargo test -p zk-kit-server --test master_key_ring` + `bash scripts/master-key-rotate.sh`（gate 6） |
| 审计篡改 | 哈希链（`prev_hash+hash`，同事务）；审计写失败 fail-closed；保留期下限 | gate 2 的 `audit-chain`/`fail-closed` 断言；WORM 外部锚定=部署控制（ADR-068，gate 7 `unverified_external`） |
| 网关重复计费/幂等 | server 生成 request_id；settle 幂等；hold 结算/void 全路径 | gate 2（`wallet-idempotency`/SSE 结算） |
| zip 穿越/炸弹/链接 | 客户端只允许相对路径、拒绝链接/设备、大小/数量上限、临时目录清理 | gate 4（desktop hostile_entries / kit traversal / WASM limits+SQL） |
| WASM 逃逸/DoS | 无网络/文件/AI；仅 4 个 host fn；fuel/time/memory/并发/DB 配额 | gate 4 |
| 命令注入（客户端工具/open_url/serve） | 参数数组不拼 shell；`open_url` 单参；管理输入校验；MCP stdio 子进程参数数组、env 白名单、静态 secret 拒绝 | gate 5a + gate 5b（`cargo test -p zk-connector-mcp`，B16 已实现） |
| SSRF/rebinding（本地 serve） | `serve` 绑 loopback；Host/Origin 精确白名单 + 一次性 token；CORS 固定 origin | gate 3（rebinding 探针；`e2e/m1-serve-security.spec.ts`） |
| 供应链 | 锁文件；CI `cargo deny/audit`、`pnpm audit`；镜像 trivy；SBOM | gate 8/9/10（外部工具/镜像环境项，见 §5） |
| 隐私夸大 | `21` §8.1 口径修正；不落 prompt/completion；日志/错误脱敏 | gate 11/12 |
| 弱管控被误解为强管控 | `00` §5 + `21` 前言；销售材料禁止“限制员工”宣称 | gate 13 |

## 5. 残余风险（1.0.0-rc.1，显式不隐藏）

- **WORM 外部锚定未实现**：代码只保证哈希链 + fail-closed；外部 WORM/对象锁属部署控制（ADR-068，`20` §3/§9）。gate 7 `unverified_external`。
- **MCP 客户端 transport（B16 已落地）**：stdio/http/sse 客户端已实现并有测试（gate 5b pass）；残余：HTTPS 的 MCP HTTP transport 明确不支持（诚实报错，ADR-060 无静态 secret），MCP 静态 secret 加密/受控交付仍延后 M3.1。
- **SSRF 的 DNS 解析后内网**：校验基于 host 字面量/IP 字面量，不固定 DNS 解析结果；重定向已禁止（`Policy::none`）。部署应使用可信公网 provider；后续可加 DNS pinning/egress 白名单（未实现，记录）。
- **旧明文 provider key**：历史行可读（迁移兼容）但会告警；需通过 admin update 重新加密（`master.key` 轮换流程见 `21` §3）。
- **JWT 签名密钥 / 更新签名密钥轮换**：离线/HSM 部署控制，无本地演练。
- **备份/PITR**：`pg_dump` + `/data` 演练通过；WAL/PITR、age/KMS 加密与异地未验证（`20` §3）。
- **镜像/三平台包/公证/真机**：本机无 Docker/证书/平台 runner → UNVERIFIED（`KNOWN-LIMITATIONS`）。
- **500 用户容量/真实 provider**：无 harness/key → UNVERIFIED；禁止 mock 冒充真实。
- **Prompt injection/agent 输出外泄**：由客户端工具审批与工作区根治理；无模型级防护承诺（弱管控边界）。

## 6. 明确非目标

- 不做工具白名单/网络限制/企业审批/部门（ADR-030/032）。
- 不做 Windows 签名（ADR-033）；macOS 证书后配（ADR-037）。
- 不做多租户/第三方安全认证（D60）；不承诺插件沙箱（WASM 是唯一沙箱）。

## 7. 评审状态与证据

- round-1 独立评审：`docs/reviews/2026-10-31-threat-model.md`（CHANGES-REQUESTED，P0-1 + P1×6）。
- round-2 修复：P0-1 SSRF 严格校验 + redirect none（gate 3）；P1-1 gate 1 结构化 marker；P1-2 gate 5 拆分 + 5b（ADR-069 延后；B16 已落地并测试转 pass）；P1-3 master.key AEAD + 轮换演练（gate 6）；P1-4 WORM 降级 ADR-068（gate 7）；P1-5 docs/21 命令与脚本对齐；P1-6 本文件更新 1.0 攻击面与 gate 映射。
- gate 1 只有评审者在复评后写入精确行 `- 威胁模型评审结论：APPROVED` 才转 `pass`；在此之前保持 `unverified`，不得宣称“威胁模型已通过”。
