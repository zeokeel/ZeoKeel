# 21 · 安全、隐私与信任边界（补充规范）

> 状态：Plan v3.2 补充规范。补齐 `04/05/06/07/09` 中"有清单、无威胁模型/密钥生命周期/隐私口径"的缺口。
> 原则：**企业版弱管控不改变安全责任**；服务端不为员工本机行为兜底（`00` §5），但必须为身份、钱、内容、审计、密钥负责。

## 1. 资产、角色与信任边界

| 资产 | 位置 | 保护目标 |
| --- | --- | --- |
| 用户身份/会话 | 服务端 + 客户端 | 机密性、完整性、可吊销 |
| 钱包账本/价格 | 服务端 PG | 完整性、不可抵赖、可对账 |
| 上游 provider key | 服务端（加密） | 绝不泄露到客户端/日志 |
| MCP OAuth 令牌 | 客户端本地（系统密钥库） | 服务端不存 token、不存 client_secret（公共客户端 + PKCE） |
| master.key / 备份 | 部署目录/离线 | 机密性、可恢复 |
| 更新签名私钥 | 离线/HSM | 绝不入 CI 镜像；泄露=供应链 RCE |
| 客户端本地文件/会话 | 员工机器 | 本地机密性；服务端不读取 |
| WASM 应用代码/DB | 服务端 | 租户隔离、资源限额 |
| 审计日志 | 服务端 + WORM | 完整性、不可篡改 |

**信任边界**：

- 客户端 → 服务端：TLS + JWT；服务端不信任客户端任何用户/角色字段。
- 服务端 → 上游：仅服务端出网；客户端不持上游 key。
- WASM guest：不可信代码；只能经宿主四个函数访问 DB/日志/资源。
- 应用 WebView：不可信页面；Tauri capability 只允许 `app_request`。
- 本地 HTTP 载体：仅本机页面；Host/Origin/token 三重校验。
- 构建输入：源码 + 锁 + 评审 + 可复现构建；运行时 manifest 不是信任根。

## 2. 认证与授权

### 2.1 认证

- 企业单主源（local/OIDC/LDAP）+ 本地应急超管；换源=换包。
- JWT：EdDSA、短效 15 min、refresh 轮换 + reuse 检测；每请求 `token_epoch`（≤5s 缓存，写操作查库）。
- 管理台：独立 Cookie+CSRF；**超管与审计员都必须有可用登录路径**（local 账号至少；OIDC/LDAP 组→角色可选）。
- 应急超管：仅库中无超管时创建；启用/重置需离线 break-glass 密封凭据 + 审计 + 告警；每季度演练。
- 管理登录：失败锁定（5 次/15 min）、限速、来源网段限制（默认本机/内网）；强制 TOTP 可选但超管建议强制。
- 设备：安装 UUID、可单独吊销；禁用/改密/降权/强制下线 → `token_epoch++`。

### 2.2 授权

- 角色：`super_admin` / `auditor`（只读：审计/用量/登录/内容变更）/ `employee`。
- 权限点 `<域>:<动作>[:范围]`；路由与权限同处申报；未授权资源 404，RBAC 不足 403。
- 审计员权限点显式声明；审计员的操作本身也留痕。
- 无部门/用户组；内容可见性 `all` / `specified_users`。

## 3. 密钥生命周期

| 密钥 | 生成/保管 | 轮换 | 丢失处置 |
| --- | --- | --- | --- |
| `master.key`（上游/MCP secret 加密） | 首启强制生成或显式提供；0600；离线备份 | 支持版本化：`master.key.vN`，新写用新 key，旧 key 保留解密 | 无备份=密文不可解；恢复流程需文档化并演练 |
| JWT 签名 key | 服务端生成/配置；`kid` | 双 key 重叠 ≥2× access TTL；旧 key 只验不签 | 立即轮换 + 全量 token_epoch++ |
| 更新清单签名私钥 | 离线/HSM；CI 只拿签名服务/短期凭据 | 年度或泄露即换；公钥内置客户端，支持双公钥过渡 | 吊销+客户端公告+强制更新；泄露视为供应链事件 |
| MCP OAuth | **公共客户端 + PKCE**；client_secret 不再下发客户端 | 按 IdP | 用户重新授权 |
| OEM 客户仓库 deploy key | 只读、按仓、可吊销 | 每客户独立 | 吊销并重发 |

- 禁止把 key 写入镜像、tar、日志、崩溃报告、审计 payload。
- 错误信息只报字段名/错误码，不回显 secret。

## 4. 网关与上游安全

- 只支持 `POST /v1/responses`；`base_url` 仅管理台配置，客户端不可传。
- SSRF：上游 URL 白名单/协议 https/禁私网地址（可配内网例外）；重定向不跟随或仅同域。
- 超时/重试/故障转移：只允许首字节前；首字节后禁止重试/换 provider（避免双计费）。
- 幂等：`request_id` 唯一索引；**同一 request_id 只结算一次**；不重放/不缓存响应；重试语义按 `04` §2.3。
- 日志：不记录完整 prompt/completion；只记 token 数、模型、耗时、request_id；需要排障时采样开关默认关且审计。
- 上游错误映射为封闭错误码；不回显上游细节。

## 5. 上传、解压与技能/MCP

- 技能/应用安装包：**客户端解压**（服务端不执行不扫描）。规格：
  - 只允许相对路径；拒绝绝对路径、`..`、符号链接/硬链接、设备文件；
  - 解压总量 ≤64MB（压缩包 ≤16MB），文件数上限；只写安装根；
  - 客户端解压：整树 sha256 校验；可执行位白名单；失败清理临时目录（服务端只审归档元数据，ADR-061）。
- MCP：stdio 命令/参数/环境变量由管理员配置；客户端执行；禁止 shell 拼接；env 白名单；工作目录固定。
- MCP OAuth：公共客户端 + PKCE；回调走本机 loopback/自定义 scheme；token 存系统密钥库；服务端不代理流量。
- OAuth token 由 app 的 keychain IPC 读写；仅无 Secret Service 的 headless e2e 在显式 `ZEOKEEL_DEV_KEYCHAIN_FALLBACK=1` 下写 `$ZEOKEEL_HOME/oauth-dev.json`（0600），生产不启用该回退。
- 上传：MIME/扩展名/大小校验；zip bomb 检测；病毒扫描暂不做（TODO，M3+；索引 [`15` §C](15-open-questions.md#c-待定清单)，不构成当前承诺）。

## 6. 客户端与本地载体

- Tauri capabilities：主窗口最小命令集；WASM 应用窗口**只允许** `app_request`；登录窗口只允许 auth 命令。
- CSP：`default-src 'self'`；`connect-src` 允许 `https:` 与本地回环（`http://127.0.0.1:*`/`localhost`，供企业动态 `server_url`、serve 载体与本地 provider），不允许任意明文远程 HTTP；个人版共享同一静态 CSP（构建期不按 edition 分叉），但个人版代码只访问配置的 provider/更新端点；禁止 `eval`/内联脚本，**当前白名单唯一例外**：桌面 WebView 的 `script-src` 允许 `'unsafe-eval'`（Ajv 运行时按生成的 JSON Schema 编译校验器，C10/T22）；只加载本地打包资源、不加载远程脚本，且 1.0.0 前必须改为构建期预编译 validator 并移除该例外（`KNOWN-LIMITATIONS.md` §4）。外部链接必须经受控 `system.open_url`（仅 http/https）。
- 深链：scheme 由渠道配置；仅白名单 action；解析失败静默忽略并记日志。
- 本地 HTTP `serve`：绑 127.0.0.1；Host 白名单；Origin 精确匹配；一次性 token 绑 Host+会话；必需自定义头；防 DNS rebinding。
- 本地数据：会话 SQLite WAL；密钥系统密钥库；导出脱敏；崩溃 dump 不含 token。

## 7. 审计完整性（必须可举证）

- 审计只追加；**哈希链必选**（`prev_hash` + `hash`），每日锚定到只写介质/外部时间戳（WORM/对象锁）。
- DB 权限：应用账号无 UPDATE/DELETE 审计表权限；DBA 操作独立审计。
- 记录字段：actor、action、target、before/after（钱/权限/配置）、request_id、ip/device、时间、结果。
- 钱操作审计矩阵见 `22` §8（唯一清单；缺项 CI 红）。
- 保留：≥180 天、用量 ≥12 个月；可配但不低于下限；删除只能由保留作业按策略执行并留证据。
- 审计写失败：**所有业务（含 AI 调用）fail-closed**，不得降级/补偿后写；记录差异并告警（ADR-049；`22` §8）。
- **实现状态（M2，2026-10-31）**：审计哈希链在业务事务内写入（`audit_tx` + advisory lock + canonical JSON 排序），
  钱包/用户/价格/设置/内容/技能/连接器/应用变更均 fail-closed；真实 PG 触发器负向测试证明审计 INSERT 失败时业务整体失败。
  历史 jsonb key 顺序未规范化的行无法重新校验，链格式升级需轮换/WORM 方案。

## 8. 隐私与数据保护

### 8.1 口径修正

- 服务端**不持久化** prompt/completion/文件内容；网关仅内存转发（排障采样需显式开启+审计+定期删除）。
- 服务端可见：身份、设备、用量/token 数、内容元数据（技能/应用名、大小、哈希）、审计事件。
- 个人版：数据本地；遥测默认关；更新走 GitHub Releases。
- 企业版：数据在客户服务器；上游 provider 由客户配置，跨境由客户自行评估；我方更新面只分发产物。

### 8.2 数据主体请求

| 请求 | 处理 |
| --- | --- |
| 导出 | 客户端导出会话/设置；服务端导出用量/审计（脱敏）；审计员可导出只读报表 |
| 删除 | 软删用户 → 内容转交/下架；保留审计与账本（审计保留策略）；余额按销户规则处理 |
| 保留例外 | 审计 ≥180 天、用量 ≥12 个月；与删除请求的优先级由部署方内部策略决定 |

### 8.3 开源产品的边界（D63）

- 本项目不提供商业数据处理服务，**不做 DPA/子处理者清单**；企业数据在客户自己的服务器。
- 上游 provider 由客户配置；数据是否出境由客户自行评估。
- 安全/隐私说明随发行说明发布，不构成对外承诺。

## 9. 事件响应

1. 发现/报告（内部或部署方）→ 2. 分级（P0 密钥/数据/供应链；P1 资损/越权；P2 其他）→
3. 遏制（吊销 key/设备/镜像下架）→ 4. 取证（审计/日志/镜像）→ 5. 修复/轮换 → 6. 通知部署方 → 7. 复盘与 ADR。

- 泄露通知：按部署方内部流程处理，不承诺商业 SLA。
- 供应链事件：撤销清单签名/下架产物/强制更新；保留证据。

## 10. 安全门禁（SSOT；13 项 + 拆分 5b）

> **唯一正文**：本节即安全门禁清单真源；`specs/changes/2026-10-30-hardening-release/spec.md` §1 与 `docs/37` 只链接本表，不重复枚举。
> **可执行实现**：`bash scripts/security-audit.sh` → `target/evidence/release-1.0-rc1/security2/security.json`
> （逐项 `id / rule / severity / owner / command / status / evidence / detail / env_only`；每项日志在同目录 `logs/`）。
> **状态口径（docs/11 §5）**：
> - `pass` 执行通过；`fail` 执行失败或真实实现缺口（strict 必红）；
> - `unverified` = CI 可满足但本轮未满足（缺 PG/二进制/工具），strict 视为 fail（exit 1）；
> - `unverified_external`（`env_only: true`）= CI 不可满足的部署/发布项（WORM、产物级 SBOM、无 trivy 报告等），strict 不因其红，但整体状态永不为 `pass`。
> **退出码**：非 strict：全 pass=0 / 有 unverified*=2 / 有 fail=1；strict（`ZEOKEEL_SECURITY_STRICT=1`）：有 fail 或 CI 可满足的 unverified=1，仅 env-only 未验证=0。
> **CI 环境契约**（`security.yml` 由 ci-release 提供）：PostgreSQL、`cargo fetch --locked` 后 `ZEOKEEL_SECURITY_SKIP_BUILD=1`、真实镜像 + 固定 trivy 报告
> `ZEOKEEL_SECURITY_TRIVY_REPORT` + `ZEOKEEL_SECURITY_IMAGE`、`pnpm audit --prod` 可联网；其余 env 见 JSON `env_contract`。
> 下表状态是 2026-10-31 round-2 证据快照，真实状态以 `security.json` 为准。

| # | 门禁 | 可执行命令 | 证据 | 本机 round-2 |
| --- | --- | --- | --- | --- |
| 1 | 威胁模型评审 + P0 缓解 | `docs/reviews/2026-10-31-threat-model.md` 必须含**精确整行** `- 威胁模型评审结论：APPROVED`（文件存在不算 pass） | `logs/01-threat-model.log`、`security2/security.json` gate 1 | pass（2026-10-31 复评 APPROVED，marker 已写入） |
| 2 | 认证绕过 / JWT 吊销 / refresh reuse / CSRF / 越权矩阵 | `bash scripts/srv-auth-test.sh`（脚本内注入 loopback dev flag + 临时测试 master key） | `logs/02-auth-jwt-csrf-rbac.log` | pass（真实 PG） |
| 3 | SSRF（上游 `base_url` 严格校验 + 重定向）+ 本地 `serve` rebinding | `cargo test -p zk-srv-gateway --lib -- security_round2_tests` + 脚本内置 provider CRUD 探针 + `zk-desktop --serve` Host/Origin/未认证 IPC 探针 | `logs/03-ssrf-rebinding.log` | pass（修复 P0-1） |
| 4 | zip 穿越/炸弹/链接（客户端）+ WASM 逃逸/资源限额 | `cargo test -p zk-desktop --bin zk-desktop -- hostile_entries`; `cargo test -p zk-kit-tools --test kit rejects_traversal_and_absolute`; `cargo test -p zk-kit-wasm` | `logs/04-zip-wasm.log` | pass |
| 5a | 命令注入 / 管理台输入校验 | `cargo test -p zk-tool-worktree --test tool rejects_injection_and_escape`; `cargo test -p zk-desktop --bin zk-desktop -- open_url`; `cargo test -p zk-srv-http` | `logs/05-command-injection.log` | pass（拆分自原 gate 5） |
| 5b | MCP 客户端 transport（stdio/http/sse、env 白名单、不拼 shell） | `cargo test -p zk-connector-mcp`（B16 实现；含 stdio/http/sse 正向 + 超时/畸形/非零退出/静态 secret 拒绝测试） | `logs/14-mcp-stdio-transport.log` | pass（B16 已落地；ADR-069 为历史决策） |
| 6 | 密钥生命周期：master.key 加密/恢复/轮换 | `cargo test -p zk-kit-server --test master_key_ring`；`bash scripts/master-key-rotate.sh`（离线 v1→v2 轮换演练） | `logs/06-key-lifecycle.log`、`target/evidence/m6/master-key-rotate.json` | pass |
| 7 | 审计哈希链 + 钱审计矩阵；WORM 外部锚定 | `bash scripts/srv-auth-test.sh`（audit-chain / fail-closed）+ `grep prev_hash crates/zk-kit-server/src/lib.rs` | `logs/07-audit-chain.log`；ADR-068 | unverified_external（WORM=部署控制，ADR-068） |
| 8 | 依赖审计 `cargo deny check` / `cargo audit` / `pnpm audit --prod` | 脚本真实检测并执行上述三个命令；缺工具/无网络=unverified，有漏洞/配置错=fail | `logs/08-dependency-audit.log`；CI `security.yml` | unverified（本机未安装） |
| 9 | SBOM：每个产物（镜像/客户端 dist/OEM）CycloneDX | `bash scripts/sbom.sh <out>`（源码 SBOM，真实执行）；产物级 SBOM 由 release pipeline 生成 | `logs/09-sbom.log`；`target/evidence/release-1.0-rc1/security2/sbom.json` | unverified_external（产物 SBOM 仅发布流水线） |
| 10 | 镜像扫描无可修复 HIGH/CRITICAL | CI: `trivy image --format json -o $ZEOKEEL_SECURITY_TRIVY_REPORT $ZEOKEEL_SECURITY_IMAGE`；脚本解析报告，`FixedVersion` 非空的 HIGH/CRITICAL 必须为 0；基础镜像无上游修复的 CVE 记入 `KNOWN-LIMITATIONS.md` §2（不谎称已修复）；本地无报告=env-only | `logs/10-image-scan.log` | unverified_external（本机无 docker/trivy/镜像） |
| 11 | 隐私口径：不得称“服务端不读取会话”除非已验证；审计/日志脱敏 | 脚本内置 `python3` 静态检查（§8.1 口径 / claim grep / migrations grep；脱敏由 gate 12 负责） | `logs/11-privacy-wording.log` | pass |
| 12 | 日志/错误响应/审计 payload/崩溃 dump 无 token/key/secret/password | `cargo test -p zk-desktop --bin zk-desktop -- published_runtime_snapshot_never_contains_provider_secrets`; `cargo test -p zk-desktop --bin zk-desktop -- hook_events_are_redacted_before_logging` + 日志宏/secret literal/unsafe 静态扫描（Rust 源码 + TS/TSX/JS 的 console/logger + 代码/配置中的私钥/AKIA/sk- literal） | `logs/12-secret-leaks.log` | pass |
| 13 | 弱管控声明写入 `docs/00`/`docs/21`；销售材料不得宣称限制员工 | 脚本内置 `python3` 静态检查（docs/00 §5 / docs/21 前言 / 产品文档肯定式措辞） | `logs/13-weak-control-statement.log` | pass |

**gate 3 SSRF 规则（P0-1，2026-10-31 修复）**：上游 `base_url` 仅允许 `https://<public host>`；`http://` 仅在显式 `ZEOKEEL_ALLOW_INSECURE_UPSTREAM=1` 时允许 loopback（本地 fake upstream/测试专用，部署禁用）；拒绝 userinfo、空 host、loopback/private/link-local/multicast/unspecified/保留段 IP（v4/v6 含 IPv4-mapped）、`localhost`/`*.localhost`/`*.local`/`*.internal`/`metadata`/`metadata.google.internal`/`instance-data`、数字混淆 host；网关 `reqwest` 统一 `redirect(Policy::none())`，不跟随 302/307。测试：`zk-srv-gateway` 单测矩阵 + `security-audit` provider CRUD 探针。

**master.key（gate 6）**：`crates/zk-kit-server::master_key` 实现 AES-256-GCM 版本化密钥环，密文格式 `vN:<base64(nonce||ct)>`；密钥来源 `<ZEOKEEL_MASTER_KEY_DIR>/master.key.vN`（`master.key` = v1，0600，拒绝组/全局可读）或 env `ZEOKEEL_MASTER_KEY`（32B hex/base64，版本 `ZEOKEEL_MASTER_KEY_VERSION` 默认 1）。新写入用最高版本；解密按前缀选版本；旧明文字段（无 `vN:` 前缀）**可读但告警**，下次写入即加密。无密钥时**新写入 fail-closed**（`MASTER_KEY_UNAVAILABLE`），已有明文仍可读以兼容旧 e2e/DB。JWT 签名密钥/更新签名密钥轮换仍是离线/HSM 部署控制（无本地演练）。

> **2026-10-31 最终本机快照**（HEAD `6d89107` + A18 worktree）：`bash scripts/security-audit.sh` 非 strict →
> **exit 2、overall `unverified`、pass=10 / fail=0 / unverified=1（gate 8：本机无 cargo-deny/audit、pnpm 网络）/ unverified_external=3（gate 7/9/10）**；
> gate 1（威胁模型 APPROVED）与 gate 14（B16 MCP transport）均为 `pass`。`ZEOKEEL_SECURITY_STRICT=1` 本机 exit 1
> 仅因 gate 8/trivy 的 CI 前置未在本机；CI `security.yml` 提供这些前置后 strict 只应因 env-only 未验证而 exit 0。
> 证据：`target/evidence/release-1.0-rc1/security2/security.json`、`security2-strict/security.json`、`target/evidence/m6/security/logs/`。

执行要求：

- 任何 `fail`、或整体 `unverified`/`unverified_external`，不得对外写“安全门禁通过”；发布说明必须列 UNVERIFIED（`docs/11` §5）。
- 外部工具项只有在 CI（`security.yml`）真跑并转 `pass` 后才可宣称通过；不得以“安装但从不调用”充当门禁。
- 新增/变更门禁只能改本节 + `scripts/security-audit.sh`（单一实现），并同步 `docs/reviews/` 的威胁模型复核。

> M3 连接器仅支持无静态 secret 配置（ADR-060）；OAuth public client + PKCE 令牌只在客户端本地。MCP 客户端 transport（stdio/http/sse）已由 Phase B B16 实现并有可执行测试（ADR-069 记录"延后到 B16"的历史决策，现已落地）。

## 7.1 审计完整性链与 request_id

- 审计 hash 链覆盖 `prev_hash + action + target + detail`；`request_id` 是运营/排障字段（用于串联 HTTP 响应、日志与审计行），**不参与 hash 计算**，避免历史链因运营字段变化而不可验证。
- 如需把 actor/request_id 纳入完整性证明，必须走 schema/链版本化迁移（旧链按旧算法验证，新链按新算法），不得原地改算法。
