# 2026-10-31-threat-model · 独立评审（docs/33 + docs/21 §10 安全 13 项门禁）

- 评审对象：`HEAD d0a1c95` 的 `docs/33-threat-model.md`（SHA256 `db8836f8…`）、`docs/21-security-privacy.md` §10
  （SHA256 `62d239f8…`）、`scripts/security-audit.sh`（SHA256 `51c233cb…`）、
  `target/evidence/m6/security/security.json`（SHA256 `7918cdf2…`，generated_at `2026-10-31T05:32:17Z`，
  head `d0a1c95`，version `1.0.0-rc.1`，summary `pass=7 / fail=0 / unverified=6`，overall `unverified`）、
  `target/evidence/m6/security/logs/`（13 份日志）、`target/evidence/release-1.0-rc1/security/summary.md`
  （SHA256 `540926dd…`）。
- 评审者：release-reviewer（独立只读评审；仅可写 `docs/reviews/`；未实现 `security-audit.sh`/docs/21/docs/33/任何产品代码）。
- 独立性：评审者未参与 A6（13 项门禁实现）或 A7 上游改动；本次逐项检查脚本实现、证据日志与相关源码
  （gateway/serve/kit-wasm）后独立判定，结论不由实现者自述决定。
- 关联 spec：`specs/changes/2026-10-31-release-1-0-readiness/`（A6 实现、A7 本记录）、
  `specs/changes/2026-10-30-hardening-release/`（13 项原始需求，acceptance 仍 `blocked`）。
- 评审日期：2026-10-31
- 结论：**CHANGES-REQUESTED**（P0=1 / P1=6 / P2=5；修复后由本评审者复评，阶段 3 终评再更新）。

## 1. 方法

- 只读核对：`docs/21` §10 逐项命令/证据/状态 vs `scripts/security-audit.sh` 的 gate1–gate13 实现 vs
  `security.json` 逐项 status/evidence/detail vs 13 份证据日志；对 marked-pass 项核对测试真实存在与断言失败行为。
- 未重跑 `scripts/security-audit.sh`（遵守"不跑长构建/不改证据"约束：脚本需 PostgreSQL、构建 `zk-server`/`zk-desktop`、
  会重写 `target/evidence/`；评审者不得改证据）。仅跑 1 条廉价命令验证 docs 命令有效性（§2 P1-5）。
- 源码核对点：`packages/server/gateway/src/lib.rs`（SSRF 校验与 HTTP client）、`apps/desktop/src/serve.rs`（本地载体）、
  `crates/zk-kit-wasm/tests/*`（WASM 限额/SQL/golden）、`scripts/srv-auth-test.sh`（gate2 断言来源）。

## 2. 13 项门禁逐项判定（"可执行检查是否真实"）

| # | SSOT 状态 | 评审判定 | 依据（命令/日志/源码） |
| --- | --- | --- | --- |
| 1 | unverified | **检查过浅（P1-1）**：脚本仅 `compgen -G docs/reviews/*threat-model*` 判文件存在，不校验评审结论/评审者/P0；本记录写入后脚本会机械转 `pass`，即使结论是 CHANGES-REQUESTED。 | `scripts/security-audit.sh:92-108`；`docs/21:143` |
| 2 | pass | **真实**：`srv-auth-test.sh` 以 `set -euo pipefail` 对真实 PG 断言，summary 行（`refresh-reuse-family`/`lockout`/`audit-chain`）只在断言通过后输出；gate 还要求这两个 marker。 | `scripts/srv-auth-test.sh:7,395,432-434`；`logs/02-auth-jwt-csrf-rbac.log` |
| 3 | pass | **false-green（P0-1）**：实现是 scheme 校验（任意 `https://` + 明文 `http://127.0.0.1|localhost`），不是 host 白名单；网关 `reqwest::Client` 未设 redirect policy（默认跟随最多 10 次），可被初始 https 地址 302/307 到内网；探针只测初始 URL scheme，未测内网 host/redirect；`docs/21:145` 写"Host/Origin"但探针未发恶意 Host。 | `packages/server/gateway/src/lib.rs:258-262,349-353`；`scripts/security-audit.sh:128-210`；`logs/03-ssrf-rebinding.log`；`docs/33:26` |
| 4 | pass | **基本真实，覆盖有缺口（P2-1）**：zip 穿越/路径逃逸/WASM fuel+memory+SQL DML 负向测试真实执行且失败即红；但"WASM 逃逸（无网络/文件/AI/他 app DB）"无负向测试（依赖 Linker 只注册 4 个 host fn 的结构性事实），golden 在 guest 缺席时静默 return（当前 guest 存在，8.37s 真实跑帧语义）。 | `crates/zk-kit-wasm/tests/{limits,sql,golden}.rs`；`logs/04-zip-wasm.log`（0 failed） |
| 5 | pass | **范围 over-claim（P1-2）**：三个 cargo 测试真实（worktree injection/open_url 单参数/srv-http 输入校验）；但门禁标题含 "stdio 参数 / env 白名单"，客户端无 MCP 执行路径（`grep -rniE 'mcp' packages/client apps/desktop/src` 无命中；`Stdio` 仅用于 open_url 子进程），证据与脚本均未覆盖该子项；`docs/33:29` 的 stdio 缓解同样未经验证。 | `docs/21:147`；`scripts/security-audit.sh:226-240`；`logs/05-command-injection.log` |
| 6 | unverified | **分类错误（P1-3）**：不是"缺外部工具/环境"，而是实现缺失（master.key 文件/加密/轮换未实现，`docs/07:111` 明确"未决"；JWT/更新签名轮换无演练）。按 `docs/21:133-139` 状态口径应为 `fail`，或先在 D/ADR 正式降级该门禁并写 KNOWN-LIMITATIONS。 | `scripts/security-audit.sh:241-256`；`logs/06-key-lifecycle.log`；`docs/21:148` |
| 7 | unverified | **部分真实 + 分类错误（P1-3）**：hash 链与 audit fail-closed 由 gate2 真实日志验证；WORM 锚定未实现（`docs/20:57` 称可选/高级部署，与 `docs/21:149` 门禁要求的 WORM 冲突）→ 应显式决策（实现 / 降级门禁 / 标 fail），不能以 `unverified` 混合"已验证的 hash 链"与"未实现的 WORM"。 | `scripts/security-audit.sh:257-274`；`logs/07-audit-chain.log`；`docs/21:149`；`docs/20:57` |
| 8 | unverified | **分类正确**：本机无 cargo-deny/audit、离线无 registry；CI `security.yml:34,36,50` 真实执行，属工具/网络缺失 → `unverified` 合规。 | `logs/08-dependency-audit.log`；`docs/21:150` |
| 9 | unverified | **分类正确（部分验证）**：源码 CycloneDX SBOM 真实生成（131 components）；产物级 SBOM（镜像/dist/OEM）本地未构建 → `unverified`。 | `logs/09-sbom.log`；`security/sbom.json`；`docs/21:151` |
| 10 | unverified | **分类正确**：无 docker/trivy/镜像 → 环境缺失 → `unverified`。 | `logs/10-image-scan.log`；`docs/21:152` |
| 11 | pass | **真实但标题映射偏窄（P2-2）**：三个静态隐私检查真实（§8.1 口径/无未限定声明/迁移无 prompt-completion 列）；门禁标题同时含"审计/日志脱敏"，该部分实际由 gate 12 覆盖，建议标题/证据互引。 | `scripts/security-audit.sh:338-382`；`logs/11-privacy-wording.log` |
| 12 | pass | **真实**：两个 redaction 测试真实执行；静态扫描 0 日志宏 secret 命中 / 0 secret literal / 0 手写 unsafe。注：`unwrap/expect` 328 命中为提示性（test-module 含），脚本明确以 clippy deny 为可执行门禁；日志宏扫描按"宏行含 secret 关键词"启发式，变量名间接泄漏不可检出。 | `scripts/security-audit.sh:383-416`；`logs/12-secret-leaks.log` |
| 13 | pass | **真实但启发式（P2-3）**：静态检查 docs/00 §5 + docs/21 前言 + 禁止肯定式"限制员工/白名单/网络限制/企业审批"措辞；英文或改写措辞可能漏检，但作为文档门禁可接受。 | `scripts/security-audit.sh:417-455`；`logs/13-weak-control-statement.log` |

## 3. P0（阻塞：门禁假绿/安全控制不成立）

### P0-1 · gate 3「SSRF」（标记 pass）是 false-green

- 事实：实现 `allowed_upstream_base()` 只做前缀判断：`https://` 任意 host 放行，另放行明文 `http://127.0.0.1`、
  `http://localhost`（`packages/server/gateway/src/lib.rs:258-262`）。网关请求 `reqwest::Client::builder()`
  **未设置 redirect policy**（`:349-353`），reqwest 默认跟随最多 10 次重定向 → 一个被允许的 `https://` upstream
  可 302/307 将请求导向内网（含 `http://169.254.169.254`、`http://localhost:<port>`、内网 https）。
- 探针不足：`scripts/security-audit.sh:128-210` 只测初始 URL 的 scheme（http/ftp 拒绝、https 接受、api_key 不回显）
  以及 serve 的 Origin/未认证 IPC/prototype；未测内网 host、redirect、也未测 Host 头（`docs/21:145` 明写 Host）。
  证据 `logs/03-ssrf-rebinding.log` 与 `docs/33:26`（"上游 URL/模型服务端固定"）均与之不符。
- 可利用性说明：provider 配置为超管专属（`admin_provider_create` 需 `require_admin`），所以需要"超管被攻陷/恶意 OEM
  预置/上游被控并重定向"才会触发；但门禁声明为 SSRF 白名单并以 `pass` 记录，属安全 gate 假绿（P0 级风险）。
- 修法（择一，二选合并）：
  1. 代码：`allowed_upstream_base` 改为可配置 host/域名白名单 + 拒绝私网/loopback/元数据地址（本地 provider 需显式
     配置开关），并给 `reqwest` 加 `.redirect(reqwest::redirect::Policy::none())`；
  2. 探针：新增 `https://127.0.0.1:<port>`、`https://169.254.169.254/latest/meta-data/`、`https→http 内网 redirect`
     三类负向用例，以及 `Host: evil.example` 用例（并确认 serve 的 cookie-token 授权即可，或补 Host 校验）。
  修完后重跑 gate 3 证据，gate 3 方能标 `pass`。
- 状态：open。

## 4. P1（应修）

| # | 位置 | 问题 | 修法 | 状态 |
| --- | --- | --- | --- | --- |
| P1-1 | `scripts/security-audit.sh:92-108`、`docs/21:143` | gate 1 只看 `docs/reviews/*threat-model*` 文件存在，不解析"评审者/结论/P0 是否已缓解"；本记录（CHANGES-REQUESTED + P0）写入后脚本仍会转 `pass` → 门禁假绿机制。 | gate 1 改为解析评审记录：要求 `评审者` 非实现者、`结论` 行 APPROVED、且无 open P0（或链接 review-check 结论）；否则 `unverified/fail`。 | open |
| P1-2 | `docs/21:147`、`scripts/security-audit.sh:226-240`、`docs/33:29` | gate 5 标题含 "stdio 参数 / env 白名单"，但客户端 MCP/stdio 执行未实现（无执行路径/无测试），pass 只覆盖 worktree/open_url/srv-http 输入校验。 | 拆分门禁：已实现部分保留 `command-injection-input` pass；stdio/env 子项标 `unverified` + KNOWN-LIMITATIONS（M3.1），并改 `docs/33:29` 措辞。 | open |
| P1-3 | `scripts/security-audit.sh:241-274`、`docs/21:148-149` | gate 6（master.key 加密/轮换未实现）与 gate 7（WORM 未实现）被标 `unverified`，混同"缺工具/环境"；按 `docs/21` 状态口径应是 `fail`，或先经 D/ADR 正式降级门禁（并同步 docs/21/known limitations）。 | 二选一：实现/演练（master.key 加密 + 轮换 drill；WORM 落地或明确 WORM 由部署方承担）；或文档化降级（改 docs/21 门禁 + D/ADR 记录 + 发布说明）。 | open |
| P1-4 | `.github/workflows/security.yml:52`、`docs/18:202` | CI 以 `ZEOKEEL_SECURITY_STRICT=1` 跑审计，但 workflow 未提供 PostgreSQL（gate2/3 不能 pass）、无 docker/trivy/镜像（gate10）、无 dist/产物 SBOM（gate9）、gate6/7 未实现 → unverified 在 strict 下转 fail → **该 job 每次必 exit 1**；而 docs/18:202 把它列为 CI 门禁，C2 要求 CI 全绿 → 发布路径被自身门禁永久阻塞。 | 拆 job：`security-required`（只对 `fail` 红，unverified 记录并上传证据）与 `security-strict`（宿主 runner + PG + docker/trivy + 产物构建，或仅在 release 矩阵跑）；同步 docs/18 状态列。 | open |
| P1-5 | `docs/21:147,154` | SSOT 命令列对 gate 4/5/12 用 `cargo test -p zk-desktop --lib -- …`，但 `zk-desktop` 无 lib target（仅 `main.rs`），实测 `exit 101: no library targets found`；脚本实际用 `--bin zk-desktop`。SSOT 命令不可复现，按 docs/21 直接复跑会失败。 | 命令列改为与脚本一致（`--bin zk-desktop`），或由脚本生成 docs 表格（单一实现）；并补 `cargo test -p zk-desktop --bin zk-desktop -- --list` 断言。 | open |
| P1-6 | `docs/33-threat-model.md`（全篇，44 行；`:22-33`） | 威胁模型未随 1.0 攻击面更新：R9 新增工具（terminal 多会话、pwsh-persistent、agent-team、hooks/JSONL 配置）无风险项；缺 prompt injection/工具审批绕过/会话文件权限/agent 输出外泄；缺 SSRF（P0-1）、master.key 明文、WORM 缺失、备份/PITR 等残余风险；测试列多为"单测/e2e"泛称，未映射 gate ID/命令。 | 补 1.0 攻击面（新工具/配置/agent 输出）、每条 P0 风险映射到 gate ID + 可执行测试命令；把已知残余（SSRF scheme-only 待修、master.key、WORM）写入残余风险并与 KNOWN-LIMITATIONS 对齐。 | open |

## 5. P2（建议）

| # | 位置 | 问题 | 修法 | 状态 |
| --- | --- | --- | --- | --- |
| P2-1 | `crates/zk-kit-wasm/tests/golden.rs:11-19` | golden 测试在 guest wasm 缺失时静默 `return`（测试仍 pass），CI/他机未 build `experiments/wasm-poc` 时会"空跑通过"；当前 guest 存在（`experiments/wasm-poc/guest-go/guest-go.wasm` 等）所以本次真实执行。 | 用 `#[ignore]` + 显式 skip 断言，或环境缺失时 `panic!`/标 unverified；在证据 JSON 记 golden 是否真实执行。 | open |
| P2-2 | `docs/21:153` | gate 11 标题含"审计/日志脱敏"，其可执行检查只覆盖隐私口径/迁移列；脱敏实际由 gate 12 覆盖。 | 标题或证据列互引 gate 12；或把标题收窄为"隐私口径"。 | open |
| P2-3 | `docs/21:137` | 状态口径句"非 strict：`pass=0`、`unverified=2`、`fail=1`"易被读成计数（实为 exit code 语义）；与当前 7/6/0 快照并列易误读。 | 改为"退出码：pass=0 / unverified=2 / fail=1（非 strict）；strict 下 unverified 视为 fail(1)"。 | open |
| P2-4 | `scripts/security-audit.sh:383-416` | gate 12 日志宏扫描仅匹配"宏行含 secret 关键词"，间接泄漏（变量名不含关键词、结构体整体 Debug 打印）不检出；`unwrap/expect` 328 为提示项不参与失败判定（已注明 clippy 为门禁，可接受）。 | 文档标注静态扫描局限；关键路径以 redaction 测试为准。 | open |
| P2-5 | `target/evidence/release-1.0-rc1/security/` | release 摘要目录只复制 `security.json`/`security-audit.md`/`summary.md`，未复制 13 份 logs（证据指针仍指向 `target/evidence/m6/security/logs/`）；跨目录取证需两处路径。 | 复制 logs 或写清指针解析规则（m6 为源、release 为摘要）。 | open |

## 6. 本记录对门禁状态的影响

- **gate 1**：按当前脚本实现（`security-audit.sh:92-108`）本文件存在后 gate 1 会从 `unverified` 机械转 `pass`。
  **评审者不建议把它记为 pass**：本记录结论为 CHANGES-REQUESTED 且含 P0-1，`docs/21:143` 的语义是"威胁模型评审通过 +
  P0 有缓解"，当前并不成立；这正是 P1-1 的假绿机制。修复 P0-1/P1 后，本评审者将更新本记录为 APPROVED，届时的 pass 才有效。
- 其它 12 项：本次评审**未改状态**（未重跑脚本，不改证据）；按 §2 判定 gate 3 应由 pass 降为 unverified/fail，
  gate 6/7 应由"unverified"改为 fail 或正式降级，gate 5 应拆分子项。重跑后 `security.json`/本记录对象 hash 会变化，
  按 `docs/reviews/README.md` 重新评审。

## 7. 复评记录

- 复评日期：pending（P0-1 + P1 修复后复评；阶段 3 终评再更新）。
- 复评结论：pending（需 P0=0 且 P1 有处置结论，才可 APPROVED）。

## 8. 关联 spec 与证据

- `specs/changes/2026-10-31-release-1-0-readiness/`（A6/A7）；`specs/changes/2026-10-30-hardening-release/`（13 项原始需求，acceptance `blocked`）。
- 证据：`target/evidence/m6/security/security.json` + `logs/*.log`；`target/evidence/release-1.0-rc1/security/summary.md`。
- 相关源码：`packages/server/gateway/src/lib.rs`、`apps/desktop/src/serve.rs`、`crates/zk-kit-wasm/tests/`、`scripts/srv-auth-test.sh`。

## 9. 当前结论

结论：**CHANGES-REQUESTED**（P0=1：gate 3 SSRF 假绿；P1=6；P2=5。修复后复评，阶段 3 终评可转 APPROVED；
在其之前 `docs/21` §10 不得宣称"安全门禁通过"，gate 1 不应以文件存在为由标 pass）。
