# 2026-10-31-threat-model · 独立评审（docs/33 + docs/21 §10 安全门禁；round-2 复评通过）

- 威胁模型评审结论：APPROVED
- 评审对象（round-2 复评快照，2026-10-31T14:05+08:00，内容变更即本记录失效）：`HEAD f0bb2b6`
  （`feat(security,ci): executable 13+ gates, SSRF hardening, master.key, honest release pipeline`）的
  `docs/33-threat-model.md`（SHA256 `f6777c76…`）、`docs/21-security-privacy.md` §10（`a3abac73…`）、
  `scripts/security-audit.sh`（`a10a277d…`）、`docs/13-risks-and-adr.md`（`7930363e…`）、
  `docs/06-server-runtime.md`（`dc9b048c…`）、`docs/07-server-domains.md` 工作树快照（`82b50c98…`，docs-version
  正在同步 §6.1）、`KNOWN-LIMITATIONS.md` 工作树快照（`3ab0ec08…`）。
  证据：`target/evidence/release-1.0-rc1/security2/security.json`（`ee88f225…`，generated `2026-10-31T05:53:32Z`，
  head `6d89107`，overall `unverified`，non-strict exit 2，pass=9 / fail=0 / unverified=2 / unverified_external=3）、
  `security2/security-audit.md`（`568a53db…`）、`security2-strict/security.json`（`15b87bce…`，strict exit 1，见 §5）、
  `security2/logs/01..14`（14 份逐 gate 日志）、`target/evidence/m6/master-key-rotate.json`（`7c302392…`，6/6 checks）。
- 评审者：release-reviewer（独立只读评审；仅可写 `docs/reviews/`；未实现 `security-audit.sh`/docs/21/docs/33/gateway/
  connector-mcp/任何产品代码）。
- 独立性：round-1（CHANGES-REQUESTED）与 round-2（本复评）均由评审者独立读取脚本/证据/源码判定；实现者只提供修复与
  证据，未参与本记录结论；本复评未重跑 security-audit（不构建、不改证据），仅核对代码/日志/配置。
- 关联 spec：`specs/archive/2026-10-31-release-1-0-readiness/`（A6/A7、B16）；`specs/changes/2026-10-30-hardening-release/`
  （13 项原始需求，acceptance 仍 `blocked`）。
- 评审日期：2026-10-31（round-1 13:33 CHANGES-REQUESTED；round-2 14:05 APPROVED）。

## 1. 结论摘要

round-1 的 1 个 P0（SSRF 假绿）与 6 个 P1 已全部真实修复（不是改文案/降级掩盖）：gate 3 现为严格 host/IP 校验 +
`redirect(Policy::none())` + 单元矩阵 + 11 条 provider-CRUD 探针 + serve 探针；gate 1 改为精确 marker；gate 5 拆分，
MCP transport（B16）落地并有可执行测试（id 14 pass）；master.key AEAD 密钥环 + 9 测试 + 轮换演练（gate 6 pass）；
WORM 经 ADR-068 正式降级为 `unverified_external`（env_only，明确不冒充 pass）；gate 8 缺工具/网络仅 `unverified`
（CI 可满足）；gate 9/10 产物 SBOM/镜像扫描为 release/CI 环境项 `unverified_external`。strict 语义已区分
`unverified`（CI 可满足，必红）与 `unverified_external`（env-only，不红且从不记 pass）。剩余为 P2 文档同步项（§4）。

## 2. round-1 P0/P1 处置核验

| # | round-1 问题 | round-2 处置（复核证据） | 状态 |
| --- | --- | --- | --- |
| P0-1 | gate 3 SSRF 假绿：scheme-only 校验 + reqwest 默认跟随重定向 | `packages/server/gateway/src/lib.rs:279-380`：https-only、loopback 明文仅 `ZEOKEEL_ALLOW_INSECURE_UPSTREAM=1`、拒绝 userinfo/private/loopback/link-local/multicast/reserved/IPv4-mapped/localhost/.local/.internal/metadata/数字混淆 host；`:378-379` `redirect(Policy::none())`。测试 `rejects_ssrf_targets`/`dev_flag_only_enables_loopback_plaintext`/`upstream_client_does_not_follow_redirects`（5 tests，日志 5 passed）。gate 3 探针 11 条（metadata/private/loopback/https-loopback/localhost/metadata-ip/userinfo/metadata-host/.local/.internal reject + https-public accept）+ serve rebinding（401/无 ACAO/404）。docs/21:145、docs/33:26 已于修复对齐 | **fixed** |
| P1-1 | gate 1 只看文件存在 → 假绿 | `security-audit.sh:98-120` 要求 `docs/reviews/2026-10-31-threat-model.md` 精确整行 `- 威胁模型评审结论：APPROVED`；本记录写入后 gate 1 才可 pass（见 §5）。gate1 当前证据 `01-threat-model.log` marker count=0（本轮修复前生成） | **fixed** |
| P1-2 | gate 5 标题含 stdio/env 但未实现/未测 | gate 拆 5a（worktree/open_url/srv-http，pass）+ 5b `mcp-stdio-transport`（id 14）：`packages/client/connector-mcp` 四件套落地（stdio spawn 参数数组、`env_clear`+allow-list、http JSON/SSE、超时/畸形/非零退出、static-secret 拒绝、HTTPS 诚实不可用），`cargo test -p zk-connector-mcp` 3+7 tests passed；ADR-069 更新记录 B16 落地；docs/07 §6.1 + crate README 明示残余（HTTPS/TLS、OAuth PKCE 流程、企业 catalog→client 配置 passthrough 均 UNVERIFIED）；docs/21:152 gate 5b pass | **fixed**（KNOWN-LIMITATIONS 同步见 P2-1） |
| P1-3 | gate 6/7 把实现缺口标 unverified | master.key：`crates/zk-kit-server::master_key` AES-256-GCM 版本化环（`vN:<b64 nonce\|\|ct>`，legacy 明文只读+告警，缺 key 新写 fail-closed），9 tests + `scripts/master-key-rotate.sh` 6/6；gate 6 pass。WORM：ADR-068（docs/13:620-625）正式降级为部署控制，gate 7 `unverified_external` + `env_only:true`；docs/06/07 已更新 master.key/SSRF 实现状态 | **fixed** |
| P1-4 | `security.yml` strict 必红（无 PG/docker/trivy/产物） | `.github/workflows/security.yml`：PostgreSQL service + 系统库、`cargo fetch`、预构建 `zk-server`/`zk-desktop`、build server image、pinned trivy 0.76.1 产出 `ZEOKEEL_SECURITY_TRIVY_REPORT`、deny/audit/geiger/pnpm audit、`SKIP_BUILD=1`、`KNOWN_GAPS=""`（gate 5b 已关闭的允许清单清空）。strict 语义见 `security-audit.sh:657-666`：仅 `fail` 或 CI-satisfiable `unverified` 红；env-only 不红 | **fixed** |
| P1-5 | docs/21 命令 `--lib` 无效（zk-desktop 无 lib target） | docs/21 §10 重写为 14 行（13 项 + 5b），命令统一为 `--bin zk-desktop`，并写明 strict/env 契约（`docs/21:131-170`）；脚本与 SSOT 一致 | **fixed** |
| P1-6 | docs/33 未覆盖 1.0 攻击面/残余风险 | `docs/33-threat-model.md` 重写（77 行）：资产/信任边界/1.0 新增攻击面（8 类）/P0 风险→gate 映射/残余风险（WORM、MCP 残余、SSRF DNS-pinning 未做、旧明文 key、JWT/更新签名轮换、备份/PITR、镜像/包、prompt injection 边界）/非目标/评审状态 | **fixed** |

## 3. 14 项门禁复核判定（round-2 证据）

| id | 门禁 | round-2 状态 | 复核判定（可执行检查是否真实） |
| --- | --- | --- | --- |
| 1 | 威胁模型评审 + P0 缓解 | unverified（待本 marker 重跑） | **真实**：精确 marker 机制（不再文件存在即 pass）；本记录写入后需重跑证据转 pass |
| 2 | 认证/JWT/CSRF/越权 | pass | 真实：`srv-auth-test.sh` 真实 PG + `set -euo pipefail` + 断言后 marker（`srv-auth-test.sh:432-434`） |
| 3 | SSRF + rebinding | pass | 真实：5 个 gateway 单元测试（含 redirect no-follow）+ 11 provider-CRUD 探针 + serve 探针，日志逐条 pass |
| 4 | zip/WASM 限额 | pass | 真实：desktop hostile_entries、kit traversal、kit-wasm limits/sql tests 执行；golden 缺 guest 时静默 pass 仍为 P2-5 |
| 5a | 命令注入/管理台输入 | pass | 真实：worktree injection、open_url 单参、srv-http 输入校验 tests |
| 5b | MCP transport（id 14） | pass | 真实：`zk-connector-mcp` 10 tests（stdio/http/SSE/超时/畸形/非零退出/secret 拒绝/HTTPS 诚实不可用） |
| 6 | 密钥生命周期 | pass | 真实：9 个 master.key tests（roundtrip/tamper/legacy/missing/rotation/permission）+ 6/6 离线轮换演练 |
| 7 | 审计链 + WORM | unverified_external | 分类正确：hash 链/fail-closed 由 gate 2 真实日志验证；WORM 为 ADR-068 明确部署控制（`env_only`，strict 不红且从不记 pass） |
| 8 | 依赖审计 | unverified | 分类正确：本机缺 deny/audit/网络 = CI-satisfiable；CI `security.yml` 真跑 |
| 9 | SBOM 每产物 | unverified_external | 分类正确：源码 SBOM 真实生成（132 components）；产物 SBOM 由 release pipeline 生成 |
| 10 | 镜像扫描 | unverified_external | 分类正确：本地无 trivy report；CI 构建真实镜像 + pinned trivy + 报告解析（0 HIGH/CRITICAL=pass） |
| 11 | 隐私口径 | pass | 真实：§8.1 口径 + 未限定声明 grep + 迁移列 grep；脱敏由 gate 12 负责（docs/21 已注明） |
| 12 | 日志/错误/审计 payload 无 secret | pass | 真实：2 个 redaction tests + 日志宏/secret literal/unsafe 静态扫描（0/0/0） |
| 13 | 弱管控声明 | pass | 真实：docs/00 §5 + docs/21 前言 + 禁止肯定式措辞扫描 |

## 4. 剩余 P2（不阻塞本次 APPROVED；phase 3 前应清理，否则终评回退）

| # | 位置 | 问题 | 修法 | 状态 |
| --- | --- | --- | --- | --- |
| P2-1 | `KNOWN-LIMITATIONS.md:74-82`（工作树） | §4 仍保留旧句"gate 5b 为 rc.1 已知产品缺口（MCP stdio 未实现），审计记 fail，CI 用 `ZEOKEEL_SECURITY_KNOWN_GAPS=14` 降级"；与现状（gate 14 pass、`KNOWN_GAPS=""`、ADR-069 更新、docs/07 §6.1）三处矛盾。发布说明会引用本文件。 | 换成当前残余：HTTPS MCP transport 不支持（诚实报错）、OAuth 授权码/PKCE 流程 UNVERIFIED、企业 catalog→client 配置 passthrough 待做、静态 secret 延后 M3.1；删除 allowlist 旧描述。 | open（docs-version 进行中，须在提交/release notes 前完成） |
| P2-2 | `docs/33:58` | §5 残余风险只写 MCP HTTPS + 静态 secret，未列 OAuth PKCE 流程与 enterprise catalog passthrough（docs/07 §6.1 已有）；docs/33 §5 自称"显式不隐藏"。 | 补两条残余并链接 docs/07 §6.1/ADR-069。 | open |
| P2-3 | `target/evidence/release-1.0-rc1/security2/*` | 证据 JSON `head=6d89107`，当前 `HEAD=f0bb2b6`（证据生成早于最终提交 2 分钟）；且 gate 1 需在 marker 写入后重跑才能转 pass。 | 在最终提交后重跑 non-strict + strict，生成 security2（+strict）新 JSON；更新 docs/18 §7.2 / docs/21 gate 1 快照行的"当前记录"措辞。 | open |
| P2-4 | `docs/18:220`、`docs/21:143` 快照行 | 本记录写入后，两处仍写"当前记录为 CHANGES-REQUESTED"/"unverified（评审者复评后写 marker）"，属于修复后的下一状态；docs 属于 docs-version 同步范围。 | 随 P2-3 证据重跑一起改状态为 APPROVED/pass（保留"以 JSON 为准"口径）。 | open |
| P2-5 | `crates/zk-kit-wasm/tests/golden.rs:11-19`（round-1 遗留） | golden 在 guest wasm 缺失时静默 `return`（仍 pass），跨机/CI 未 build `experiments/wasm-poc` 时空跑通过。 | `#[ignore]` + 显式 skip 断言，或缺失即标 unverified/失败；证据 JSON 记录是否真实执行。 | open |
| P2-6 | `docs/21:152` gate 5b 行 | 标题写"stdio/http/sse"，未在行内注明 HTTPS/TLS 不支持（docs/33 §5、docs/07 §6.1 已注明）。 | 行内补"HTTPS UNVERIFIED/不支持"，或链接 docs/07 §6.1。 | open |
| P2-7 | `scripts/security-audit.sh` gate 12/13 启发式（round-1 遗留） | 日志宏 secret 扫描按关键词、措辞扫描按中文正则，存在漏检；作为辅助门禁可接受，但应标注局限。 | 证据 JSON detail/文档注明"启发式，关键路径以 redaction tests 为准"。 | open |

## 5. 本记录对门禁状态的影响 + strict 预期

- **本评审只改变 gate 1**：本记录写入精确行 `- 威胁模型评审结论：APPROVED` 后，下一次 `security-audit.sh` 运行
  gate 1 由 `unverified` → `pass`（这是 admin/CI 重跑证据的动作，不是评审者改证据）。
- 其它 13 项状态不变；按 §2 复核，round-1 的"应降级/应拆分"已由修复落地：gate 3 pass（真）、gate 5 拆 5a/5b、
  gate 6 pass、gate 7 `unverified_external`（ADR-068）、gate 8 `unverified`（CI-satisfiable）、gate 9/10 `unverified_external`。
- strict 预期（CI，环境齐备）：gate 1 pass（marker）、gate 2/3/6 pass（PG+构建）、gate 8 pass（deny/audit/pnpm 网络）、
  gate 7/9/10 env-only 不红 → **strict exit 0**，且假绿被排除（env-only 永不记 pass）。
  本机 strict exit 1 仅因 gate 8 工具缺失（CI-satisfiable），属正确语义而非缺陷；证据 `security2-strict/security.json` 已如实记录。
- 评审对象 hash 若因 docs-version 继续编辑（KNOWN-LIMITATIONS/docs/07）而变化，本记录按 `docs/reviews/README.md` 失效重评
  （§4 P2-1/P2-2/P2-4 即要求这些同步完成）。

## 6. 复核方法（只读）

- 读：docs/33（77 行）、docs/21 §10（14 行）、docs/18 §7.1/§7.2、docs/13 ADR-068/069、docs/06 §5、docs/07 §6.1、
  `scripts/security-audit.sh` gate1–gate14 + aggregate、`.github/workflows/security.yml`、14 份 security2 日志、
  non-strict/strict security.json、master-key-rotate.json。
- 读源码核对：`packages/server/gateway/src/lib.rs`（validator/ip_is_disallowed/redirect/tests）、
  `packages/client/connector-mcp/`（lib+2 test files+README）、`crates/zk-kit-server/tests/master_key_ring.rs`、
  `scripts/master-key-rotate.sh`、`scripts/srv-auth-test.sh`。
- 未重跑 `security-audit.sh`（构建/证据写入非评审权限）；未修改任何产品代码/文档/脚本/证据。

## 7. 关联 spec 与证据

- `specs/archive/2026-10-31-release-1-0-readiness/`（A6/A7、B16/R9.6）；`specs/changes/2026-10-30-hardening-release/`。
- 证据：`target/evidence/release-1.0-rc1/security2/`（+ `security2-strict/`、`logs/`）；`target/evidence/m6/master-key-rotate.json`。
- 相关代码：`packages/server/gateway/src/lib.rs`、`packages/client/connector-mcp/`、`crates/zk-kit-server/src/master_key*`、
  `apps/desktop/src/serve.rs`、`crates/zk-kit-wasm/tests/`、`scripts/{security-audit,srv-auth-test,master-key-rotate}.sh`。

## 8. 当前结论

结论：**APPROVED**（round-2 复评：P0=0 / P1=0；round-1 的 SSRF/门禁拆分/master.key/WORM/strict-CI/命令真源/威胁模型 8 项修复
均经代码+证据+日志独立核实为真实修复；剩余 P2×7 为文档同步与启发式局限，不阻塞安全门禁，但 P2-1/P2-2/P2-3/P2-4
须在 phase 3 终评前完成，否则本评审将回退为 CHANGES-REQUESTED）。
