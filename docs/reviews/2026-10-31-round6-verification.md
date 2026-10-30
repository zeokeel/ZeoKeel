# Round-6 独立评审（426 闸门/渠道/门户、OEM gitlink、Tauri CSP/open_url、docs truth）

- 评审对象：`HEAD 4eddfcb`（首评；`fix(hardening): unified 426 gate/UX, channel min version, OEM gitlink-only, Tauri path`）。复评对象：`HEAD 3c24611`（`fix(round6): native client identity, authoritative channel minimum, portal lock, Windows opener`）→ **最终复评对象 `HEAD 7cd0362`**（`build(compose): commit server CHANNEL const in generated baseline`；P1-R1 修复；详见 §8）。评审/复评时工作树 clean（唯一未跟踪文件即本记录）。
- 评审者：**auditor-build**（独立评审；本轮未实现任何被评审代码）。
- 独立性：round-6 由 Lead（proposal owner）与 auditor-web / auditor-rust 实现（见 proposal）；评审者此前只做过 delivery/e2e 审计（task-34）与 tooling 修复（task-12），未参与本提交。
- 关联 spec：`specs/changes/2026-10-31-round6-hardening`（proposal/spec/design/tasks/acceptance；proposal 已链接本记录）。
- 评审日期：2026-10-31
- 结论：**APPROVED**（最终 HEAD **7cd0362**：P0=0 / P1=0 / P2=9 中 2 项已闭环、其余为记录在案的非阻塞残留）。首评 4eddfcb 的 4 个 P1 全部修复并有单测/现场验证；复评新发现的唯一 P1-R1（`apps/server/src/generated/packages.rs` 漏提交 `CHANNEL`）已由 7cd0362 以 1 行生成物修复，`cargo run -q -p zk -- compose ... --check` 在 7cd0362 干净树上 **EXIT 0**；review-check **0**。详见 §8。

## 1. 评审范围

- 服务端 426：`packages/server/http/src/lib.rs`（`is_client_business_path`/`client_version_gate`/`upgrade_required`/request-id/CORS）、`crates/zk-kit-server/src/lib.rs`（`client_channel`/`min_client_version_for_channel`）、`packages/server/updates/src/lib.rs`（manifest 426/ETag/Range）、`packages/server/settings/src/lib.rs`（bootstrap 426）。
- 客户端：`apps/desktop/web/src/{client,upgrade,UpgradeGate,App,auth,store}.ts*`、`transport/http.ts`、`state/{controller,reducer}.ts`、`i18n.tsx`、`protocol.ts`、`generated/brand.ts`；`apps/desktop/src/{ipc,main}.rs`；`apps/desktop/tauri.conf.json`、`capabilities/*.json`。
- e2e：`e2e/client-upgrade.spec.ts`、`scripts/client-upgrade-e2e.sh`、`playwright.config.ts`；m1/agent/login/admin/apps/updates/oem 脚本与用例。
- 组合/OEM：`tools/zk/src/main.rs`（`is_git_index`/`resolve_index_pin`/`gitlink_hash`/`channel_pin_tests`）、`docs/09`。
- 文档真源：`docs/04`（§2.2/§2.7/§7/§10）、`docs/09`（pin/更新）、`docs/19`（升级门禁）、`docs/21`（CSP/serve）、`docs/27`（inspector 文件级）。

## 2. 门禁复跑（独立执行；全部本机真实运行）

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | **EXIT 0** |
| `cargo clippy --workspace --all-targets -- -D warnings` | **EXIT 0**（`target/evidence/audit/round6/clippy.log`） |
| `cargo test --workspace` | **EXIT 0**（418 个 test-result 块，382 passed / 0 failed；`workspace-tests.log`） |
| `cargo test --workspace export_schema` + `git diff --exit-code` | **EXIT 0 / 0**（生成物确定性；`export-schema.log`） |
| `bash scripts/schema-check.sh` | **EXIT 0** |
| `zk validate` | **EXIT 0**（114 packages；114 scanned） |
| `zk compose --edition personal --profile default --channel official --check` | **EXIT 0**（16 files up to date；注意：须用 HEAD 重新编译的 `zk`，旧二进制会造成假红） |
| `zk capability --check` / `zk docs --check` / `zk db lock --check` / `zk lint --duplicates` | **0 / 0 / 0 / 0**（45 capabilities；duplicates 61 windows ≤ baseline 61） |
| `python3 scripts/docs-check.py` / `spec-check.py` / `review-check.py` | **0 / 0**；review-check 在写本记录前 **1**（缺本文件，预期） |
| `pnpm -r typecheck` / `lint` / `test:unit` / `build` | **0 / 0 / 0 / 0**（`pnpm-gates.log`/`pnpm-build.log`） |
| `bash scripts/m1-e2e.sh` | **EXIT 0**（mock 7 passed；真实 serve/SQLite 7 passed） |
| `bash scripts/agent-e2e.sh` | **EXIT 0**（9 passed） |
| `bash scripts/login-e2e.sh` | **EXIT 0**（1 passed + 3 passed；真实 server/serve；端口已释放） |
| `bash scripts/admin-e2e.sh` | **EXIT 0**（11 passed，含 m5-portal/m6-a11y） |
| `bash scripts/client-upgrade-e2e.sh` | **EXIT 0**（真实 426 server + serve；1 passed；gate 显示 0.1.0/9.9.9、portal、无重试） |
| `bash scripts/apps-e2e.sh` / `updates-e2e.sh` / `oem-e2e.sh` | **0 / 0 / 0** |
| 工作树/清理 | 每次 compose 型 e2e 后 `zk compose personal --check` **0**；无本轮残留 server/DB；`git status` 仅本记录（写入后） |

> 说明：首次用旧 `target/debug/zk` 跑 `compose --check` 得到假红（generated 文件 key 顺序差异），重新 `cargo build -p zk` 后为 **EXIT 0**；不是提交态缺陷，但记录复现命令需用 HEAD 二进制。

## 3. 结论依据（按 focus）

### 3.1 web 426 统一映射 / 相对 portal / 版本说明 / platform / request-id
- `client.ts` `serverFetch` 对 **13 处** web 直连 server 调用统一注入 `X-ZeoKeel-Client`（真实 platform：linux-x64/windows-x64/macos/other）与 `X-Request-Id`，426 一律抛 `UpgradeRequiredError`；`auth.ts`（login/refresh/bootstrap）与 `store.ts`（skills/connectors/apps/archive/assets/install/tombstones）均改用 `serverFetch`（`grep fetch(` 仅剩 oauth 外部 token URL 与本地 `/ipc`，合理）。
- `upgrade.ts` `resolvePortalHref`：绝对 http(s) 原样、`/portal` 按 server origin、其它相对按 `<server_url>/`、缺失回退 server_url+/portal；`UpgradeGate` 全屏无关闭/无业务重试，显示 current/min/server（server 取 `meta.version`），Tauri 走 `system.open_url`、浏览器 `window.open(..., "noopener")`；reducer 合并多次 426 信息（后到的空信息不覆盖已有版本）。zh/en i18n 同步。
- 实测：426 响应 `{"error":{"code":"CLIENT_TOO_OLD","retryable":false,"request_id":...,"current":"0.1.0","minimum":"9.9.9","portal_url":"/portal"}}`，`X-Request-Id` 回显；200 稳定面也回显 request-id（probe.log）。真实 e2e 覆盖 gate 显示/portal/无重试。

### 3.2 真实 426 e2e 可信度
- `scripts/client-upgrade-e2e.sh` 真实 PG + `ZEOKEEL_MIN_CLIENT_VERSION=9.9.9` server + `zk-desktop --serve` + Playwright，EXIT 0；断言 gate 出现、0.1.0/9.9.9、portal=`<server>/portal`、业务按钮不存在、1500ms 内 426 请求数不增长。**真实、非 mock**。
- 缺口：仅测全局 min；未覆盖按渠道 min、body envelope 变体、native app 请求（见 P1/P2）。

### 3.3 426 envelope / 渠道 min version
- 业务面 gate：嵌套 envelope + `retryable:false` + request_id（header/body 一致）；缺头/非法头 fail-closed 426（符合 spec §1 选择）；`X-Request-Id` 客户端可传，缺失生成 UUID（live probe）。
- **P1-2/P1-3（渠道）**：`min_client_version_for_channel` 的顺序是“channel env → brand.min_client_version → global”，但：
  - channel 来自**客户端可控** `X-ZeoKeel-Client` 第二段（`zk-kit-server:412`；gate 使用 `http:103`），因此可伪造低门槛渠道通过闸门（live repro：global 0.1.0 + `ZEOKEEL_MIN_CLIENT_VERSION_OFFICIAL=9.9.9` 时，header `official`→426，header `beta`→**401** 而非 426 = 绕过该渠道硬门槛）；
  - brand 分支**不可达**：`brand_snapshot()` 读取生成 `shared/contracts/brand.json`，但 `tools/zk` `brand_json()` 不生成 `min_client_version` 字段，且 `Brand`/`ChannelFile` 都是 `deny_unknown_fields` 且无该字段（`brands/*/channel.json` 也无法配置）；单测只覆盖 channel env。
- 这与 `docs/04:161`“按渠道配置 min_client_version（env 或品牌编译值）”和 acceptance:22“`<CHANNEL>` override → brand → global”不符；round-5 P2-6 并未真正闭环。

### 3.4 OEM pin gitlink-only
- `tools/zk` `is_git_index`（`.git` 目录/文件，或 `git rev-parse --show-toplevel` 精确等于 index 根）→ git 索引必须有 mode 160000 gitlink 且 == pin，否则直接失败；只有非 git（fixture/export）回退目录哈希并显式 `fixture fallback`。`channel_pin_tests` 4 个（非 git 回退、git 无 gitlink 拒绝、gitlink 不匹配拒绝、匹配通过）全部 **14 passed**；`oem-e2e.sh` EXIT 0；`docs/09:91` 与实现一致。**该面通过**。
- 残余风险（P2，非阻塞）：`is_git_index` 对“位于其它 git 仓库内、自身无 .git”的目录会判非 git 并回退目录哈希（生产 index clone 自带 .git，正常不触发）；`gitlink_hash` 依赖 `git` 可执行文件（缺失时 git 索引会硬失败，属 fail-closed）。

### 3.5 Tauri CSP + system.open_url
- CSP `apps/desktop/tauri.conf.json:20` 增加了 `connect-src ... https:` 与回环 http/ws；`system.open_url`（`ipc.rs:602-660`）校验 absolute http(s)、无 credentials、无 fragment、长度 ≤2048，固定平台 opener（xdg-open/open/cmd start），spawn 不等待；单测覆盖 scheme/length/credentials（`ipc.rs:1548+`），`ipc_call` 对 app windows 禁止（`main.rs:65-70`），app_request 仅 app window。
- **P1-4（安全）**：Windows 分支 `cmd.args(["/C","start","", url])`（`ipc.rs:632-637`）把校验后的 URL 作为 cmd.exe 参数；Rust 的 `Command` 不会为 `& | ^ % < > ( )` 等 cmd 元字符加引号/转义，URL query 可含 `&`（如 `https://host/?a=1&calc`）→ cmd 将 `&` 后作为新命令执行（任意本地命令）。URL 来源包括 426 `portal_url`（服务端返回）与 webview 内调用；当前测试只验证字符串校验，不断言 argv/注入。Windows 运行时 **UNVERIFIED**（本机 Linux），但代码路径确定。
- **P2-5（CSP 文档真源）**：`tauri.conf.json` 是单一静态 CSP，`connect-src https:` 对 personal/enterprise 都生效；`docs/21:83` 声称“个人版 connect-src 仅配置的服务端/上游，企业版才允许 https”，且同一行声称“禁止 eval/内联脚本”，但 CSP 仍有 `script-src 'unsafe-inline'`。无构建期 edition/channel CSP 生成（tools/zk 无 CSP 相关代码），文档与实现不一致（Tauri 真机 UNVERIFIED 仅记录环境限制，不覆盖该逻辑差异）。

### 3.6 inspector 文件级 changes 文案
- `packages/ui/ui-inspector/web/src/index.tsx:70` 使用 `inspectorChangesFile`（“文件变更”）+ `aria-label`（path/add/del），与 `docs/27:53`“文件级 +add/−del（无行级 diff）”一致；i18n 中英齐备；m1/agent e2e 通过。**通过**。

### 3.7 docs truth / false-green
- 已核对并一致：`docs/09:91` pin、`docs/19:25` 门禁动作、`docs/27:53` changes、`docs/04:258` 业务面 426 层（登录/刷新/bootstrap 例外、缺失头 fail-closed）、`docs/04:196` 426 嵌套 envelope/回显（对**业务面 gate**成立）。
- 不一致（见 P1/P2）：`docs/04:161` 品牌编译 min + 硬门槛（brand 死代码、header 可伪造渠道）；`docs/04:196` 称 426 统一 envelope + `request_id 全链路透传`（bootstrap/updates 的 426 仍 flat `{"ok":false,"error":"CLIENT_TOO_OLD"}`；audit_log.request_id 列未被 `audit_tx` 写入）；`docs/21:83` CSP 版本身份/内联脚本；`docs/09:127` “低于 min_supported 时**业务面** 426”未覆盖 `/v1/*` 网关（旧客户端仍可聊天，见 P2-4）。
- acceptance：大部分命令/ e2e 声明经独立复跑成立（见 §2）；但 “426 envelope/**渠道**/request-id、Tauri CSP/open_url | 单测+真实 PG smoke” 这一行把 brand 渠道（不可达）与 Windows argv（未测）计入通过，属**证据夸大**（P1-3/P2-7），不是整行 false-green。

## 4. P0 / P1 / P2

**P0：无。**

**P1（阻塞 APPROVED）**

| # | 位置 | 问题 | 复现/证据 | 修法 |
| --- | --- | --- | --- | --- |
| P1-1 | `apps/desktop/src/ipc.rs:665-706`（`app_request`，endpoint `:691`，`client.post(url).bearer_auth(token)`）；`apps/desktop/src/main.rs:333-341`（app asset fetch，仅 bearer）；gate `packages/server/http/src/lib.rs:92-114`（缺头 fail-closed 426） | **Tauri WASM app 窗口在 enterprise 下不可用**：native 两个业务调用都不带 `X-ZeoKeel-Client`，被 gate 判 426；app_request 把 426 映射成 `APP_UNAVAILABLE`，asset 路由回 502。browser/serve 的 web store 走了 serverFetch 所以应用中心 e2e 通过，掩盖了 native 路径。 | live probe：`GET /api/client/v1/apps/abc/assets/index.html` 仅 Authorization → **426**（`target/evidence/audit/round6/probe.log`）；静态：两处 reqwest 均无客户端头。 | 抽共享 native HTTP client 注入 `X-ZeoKeel-Client`（version/channel/platform/device）与 `X-Request-Id`；加 Tauri app-window 单测/e2e 断言头存在；明确 426 在 native 路径的 UX。 |
| P1-2 | `crates/zk-kit-server/src/lib.rs:412`（`client_channel` 读请求头）、`:446-466`（`min_client_version_for_channel`）；`packages/server/http/src/lib.rs:103`；bootstrap `packages/server/settings/src/lib.rs:118-119` | **按渠道硬门槛可被客户端伪造 channel 绕过**：低版本客户端把 `X-ZeoKeel-Client` 第二段改成无 override 的渠道，即回退 global min 通过 gate（版本字段本身客户端可控属弱管控，但“渠道级硬门槛”变成客户端自选档位）。 | live repro：server `ZEOKEEL_MIN_CLIENT_VERSION=0.1.0` + `ZEOKEEL_MIN_CLIENT_VERSION_OFFICIAL=9.9.9`；header `0.1.0;official` → **426**，header `0.1.0;beta` → **401**（gate 已放行，probe.log）。 | 服务端以**权威渠道**（编译 brand.channel 或 server channel settings）解析 minimum；对 header channel 只做诊断；或取 max(header-channel, 权威渠道, global) 并拒绝未知渠道的更低值。 |
| P1-3 | `crates/zk-kit-server/src/lib.rs:454-464`（brand 分支）；`tools/zk/src/main.rs:2118`（`brand_json` 无 `min_client_version`）、`:392-399`（`Brand` deny_unknown_fields，无该字段）；acceptance:22；`docs/04:161` | **“品牌编译值”渠道 min 是不可达代码/文档承诺**：生成 brand.json 与 channel.json 都没有 `min_client_version`，唯一可用的是 env；round-5 P2-6 被记为闭环但实际只做了一半。单测 `per_channel_min_version_overrides_global_fallback` 只测 env。 | `grep min_client_version brands/*/channel.json shared/contracts/brand.json` 无结果；静态路径 `brand_snapshot()["min_client_version"]` 永远 None；channel.json 增加该键会被 `deny_unknown_fields` 拒绝。 | 打通 channel 配置→`ChannelFile`/`Brand` 字段→`brand_json`/`brand.ts`（并补 validate），或删除 brand 分支并同步 docs/04 + acceptance 为“env/服务端设置”；补 brand 路径单测 + 真实 PG smoke。 |
| P1-4 | `apps/desktop/src/ipc.rs:619-648`（`open_platform_browser`；Windows `:632-637`）；校验 `:602-617`；测试 `:1548+`（只测字符串） | **Windows `system.open_url` 命令注入（安全）**：`cmd /C start "" <url>` 未经 cmd 元字符转义；http(s) URL 的 query/path 可含 `&`/`|`/`^`/`%`/`<`/`>`，cmd.exe 会解析为命令分隔/展开 → 打开由服务端（426 portal_url）或 webview 控制的 URL 时可执行本地任意命令。 | 静态代码路径；Windows 运行时 **UNVERIFIED**（本机 Linux）。对 `https://host/?a=1&calc`，cmd 会把 `&calc` 当第二条命令。 | 用 `ShellExecuteW`/`open` crate 直接打开，或严禁 cmd 元字符并显式引号/转义；避免经 cmd；补 Windows 目标单测（argv 断言/拒绝元字符）。 |

**P2（建议修复）**

| # | 位置 | 问题 | 修法 |
| --- | --- | --- | --- |
| P2-1 | `apps/desktop/web/src/upgrade.ts:127-145`（`resolvePortalHref`）；`UpgradeGate.tsx` `window.open` | 426 `portal_url` 只按 http(s) 绝对/相对解析，未限制**同源**；`javascript:`/`//evil.com`/任意 https 会被使用（node 验证：`javascript:alert(1)`→同名、`//evil.com`→`https://evil.com`）。浏览器/serve 路径可被服务端/中间人诱导到恶意 URL；Tauri 路径有 open_url http(s) 校验。 | 将 portal_url 限制为 activeServerUrl origin 的 http(s)（或只允许 `/portal` 相对），新增 javascript/跨源单测。 |
| P2-2 | `packages/server/updates/src/lib.rs:268`（`error(UPGRADE_REQUIRED,..)`）；`packages/server/settings/src/lib.rs:118-119`；`crates/zk-kit-server/src/lib.rs:52-59`（`ServerError` → `{"ok":false,"error":code}`）；`docs/04:196` | **426 envelope 未统一**：业务 gate 为嵌套 envelope（retryable/request_id/current/min/portal），bootstrap/updates 的 426 仍是 flat `{"ok":false,"error":"CLIENT_TOO_OLD"}`（probe2.log）；docs/04 称 426 使用嵌套 envelope + request_id 全链路；acceptance 称“426 统一信封”。header request-id 由 middleware 回显，但 body 无 request_id/版本/portal。 | 统一 426 构造函数（bootstrap/updates 复用 gate envelope），或 docs/04 明确“426 仅业务 gate 为 nested，其余逐步迁移”并收窄 acceptance。 |
| P2-3 | `crates/zk-kit-server/src/lib.rs:672`（`INSERT INTO audit_log (actor_id, action, target, detail, prev_hash, hash)`）；迁移 `packages/server/audit/migrations/0006_audit.sql:5`（`request_id text NOT NULL DEFAULT ''`）；`docs/04:196` | `request_id` 未写入审计/用量：`audit_tx` 不绑定 request_id 列，审计行恒为 `''`，docs “request_id 全链路透传”不成立（probe2 无审计行可证；静态代码明确未写）。 | 在 gate 中把 request_id 注入 AppState/Extension 并在审计 INSERT 落库；或 docs 改“回显按请求，审计暂不落 request_id”。 |
| P2-4 | `packages/server/http/src/lib.rs:31-40`（`is_client_business_path` 只覆盖 `/api/client/v1/*`）；`packages/server/gateway/src/lib.rs:1204`（`/v1/responses`） | **强制升级未覆盖 AI 网关**：旧客户端（合法 token）仍可调用 `/v1/responses`（probe2：old header → 402 MODEL_NOT_PRICED，非 426），而 `docs/09:127` 说“低于 min_supported 时业务面 426”。 | 将网关路由纳入 426 覆盖（或显式在文档/ADR 声明网关不受 min 约束及原因）。 |
| P2-5 | `apps/desktop/tauri.conf.json:20`；`docs/21:83`；无构建期 CSP 生成 | CSP 单一静态：`connect-src ... https:` 对 personal 同样生效，与 docs/21“个人版仅配置服务端/上游”不符；`script-src 'unsafe-inline'` 与 docs“禁止内联脚本（除白名单）”不符。 | 生成期按 edition/channel 生成 CSP，或同步 docs；缩小 `https:` 到 server/更新域白名单。 |
| P2-6 | `apps/desktop/src/ipc.rs:645-647`（`.spawn()` 后立刻丢弃 Child）；`UpgradeGate.tsx` openPortal `.catch(console.warn)` | open_url 子进程不 `wait`/reap（Unix 僵尸直到 app 退出），且打开失败仅 console.warn 无用户反馈；`window.open` 返回 null 也无提示。 | spawn 后 detach 并处理 SIGCHLD/reap（或平台打开 API），失败回传 UI toast。 |
| P2-7 | `scripts/client-upgrade-e2e.sh`；`acceptance.md:13,16,22-24`；测试面 | 证据/测试覆盖缺口：per-channel min 无真实 PG smoke（仅 env 单测；brand 不可达；channel spoof 未测）；426 e2e 仅 global；native app_request/asset 426 无测试；Windows argv/cmd 元字符无测试；CSP 无构建产物断言；acceptance 把这些计入“通过”。 | 按 P1-1..P1-4 补测试；acceptance 把未覆盖项列入 UNVERIFIED 或明确 not-done。 |
| P2-8 | `packages/server/http/src/lib.rs:43-61`（`request_id_for`/`attach_request_id`）；`docs/04:196` | client 传入的 request_id 仅长度/非空校验，原样回显到响应头/body（HeaderValue 拒绝非法字符时**静默不加头**，body 仍有该值）→ “echo” 在非法字符时不一致；≤128 任意可见字符反射增加日志注入面。 | 校验字符集（如 `[A-Za-z0-9._:-]`），非法时生成新 id；文档/测试覆盖。 |
| P2-9 | `tools/zk/src/main.rs:1806-1818`（`is_git_index` 的 `git rev-parse` fallback）；OEM 交付流程 | 位于其它 git 仓库内、自身无 `.git` 的 index 目录会被判非 git 并走目录哈希回退（生产 clone 有 .git，正常不触发）；`git` 不在 PATH 时 git 索引硬失败（fail-closed）。 | 文档/实现显式 `--fixture` 标志替代“非 git 即回退”，或检测外层 repo 并拒绝。 |

## 5. UNVERIFIED（环境限制，不计入本轮缺陷）

- Windows 真机 `system.open_url` 命令注入的实际触发（P1-4 为代码级判定；本机 Linux 无法执行 cmd.exe）。
- Tauri 真机（CSP 生效、`__TAURI_INTERNALS__` 分支、open_url 真实打开、app window native 路径的可见性）；所有 UI/e2e 证据来自 browser/serve 载体。
- 真实 OEM 私有索引仓 + submodule/deploy-key（无网络；fixture 路径已覆盖 gitlink/回退/tests）。
- Docker/Caddy/镜像/三平台安装包/notarize；cargo-deny/audit（无网络）。
- 26/大数量 request-id 压力/日志注入实际影响（P2-8 静态）。

## 6. 复评与状态 + round-5 P2 闭环核对

- 本记录为 round-6 **首评**（4eddfcb）＋ **复评**（3c24611）＋ **终评**（7cd0362，§8）；最新结论：**APPROVED（7cd0362）**。
- 终评门禁（7cd0362）：fmt/clippy/workspace tests/export_schema/schema-check/zk validate/capability/docs/db lock/lint/docs-check/spec-check 均 **0**；**`zk compose personal --check` = 0**（P1-R1 已由 7cd0362 修复）；client-upgrade e2e **0**；review-check **0**。
- round-5 P2 闭环：
  - P2-1 统一 web fetch 426 → **已闭环（web）**；native 路径遗留 → 新 P1-1。
  - P2-2 相对 portal 解析 → **已闭环**；同源/协议约束缺口 → 新 P2-1。
  - P2-3 426 envelope → **部分**（业务 gate 有；bootstrap/updates flat）→ P2-2。
  - P2-4 platform/request-id → **已闭环（头/回显）**；审计落库缺失 → P2-3。
  - P2-5 版本说明 → **已闭环**（gate 显示 current/min/server，e2e 断言 current/min）。
  - P2-6 渠道 min → **未闭环**（brand 不可达 + header 可伪造渠道）→ P1-2/P1-3。
  - P2-7 真实 426 e2e → **已闭环**（真实 server e2e 0）；覆盖缺口 → P2-7。
  - P2-8 gitlink-only → **已闭环**（4 单测 + oem e2e 0；docs/09 同步）；残余 fallback 判定 → P2-9。
  - P2-9 Tauri 企业路径 → **部分**（CSP/open_url 已实现；Windows 注入新 P1-4；personal CSP 文档不一致 P2-5；真机 UNVERIFIED）。
  - P2-10 changes 文案 → **已闭环**（docs/27:53 + UI + 测试）。

## 7. 证据路径

- 门禁/e2e 日志：`target/evidence/audit/round6/{clippy,workspace-tests,export-schema,pnpm-gates,pnpm-build,zk-tests,client-upgrade-e2e,m1-e2e,agent-e2e,login-e2e,admin-e2e,apps-e2e,updates-e2e,oem-e2e}.log`
- 真实 server 探测：`target/evidence/audit/round6/probe.log`（channel env/伪造、native 无头 426、request-id 回显、updates 426/ETag/Range、portal `/`+`/portal`、shutdown），`probe2.log`（bootstrap 426 flat、gateway 未 426、路径变体、audit 行）。

## 8. 复评（HEAD 3c24611 → 终评 HEAD 7cd0362，2026-10-31）

- 复评对象：`HEAD 3c24611` `fix(round6): native client identity, authoritative channel minimum, portal lock, Windows opener`（父 4eddfcb = 首评）；**终评对象：`HEAD 7cd0362` `build(compose): commit server CHANNEL const in generated baseline`（父 3c24611；P1-R1 修复）**。工作树 clean（唯一未跟踪文件 = 本记录）。复评者 auditor-build，未参与修复。
- 复评结论（3c24611）：CHANGES-REQUESTED，P1=1 剩余（提交态生成物基线漂移），首评 4 个 P1 全部修复；**终评结论（7cd0362）：APPROVED（P0=0/P1=0；P2 中 2 项已闭环，其余为非阻塞残留，见 §8.4）**。P1-R1 已由 7cd0362 的唯一一行生成物修复，见 §8.2/§8.6。

### 8.1 首评 P1/P2 复核（已修复）

| 首评项 | 修复位置 | 复评证据 |
| --- | --- | --- |
| P1-1 native app 请求无客户端标识 | 新 `apps/desktop/src/native_client.rs`（`CLIENT_HEADER`/`REQUEST_ID_HEADER`/`apply_headers`）；`ipc.rs:725` `app_request_builder` 与 `main.rs:336` asset fetch 均 `apply_headers`；`tools/zk` 生成 `CHANNEL` 常量（`compose_lib:1435`/`generated_packages_rs:2313`） | 单测 `native_client::tests::{client_header_has_version_channel_platform_device,apply_headers_sets_client_and_request_id}`、`ipc::tests::app_request_builder_attaches_native_identity_and_request_id` 全部 pass（workspace tests 0） |
| P1-2 客户端 header channel 可降 min | `zk-kit-server` 新 `server_channel()`/`min_client_version_for_server()`（kit:440-489）；http gate 改用权威渠道（http:99-110）；bootstrap 走 `client_is_too_old`（`settings:118`） | 单测 `spoofed_header_channel_cannot_lower_the_authoritative_minimum` pass；live probe：global 0.1.0 + `ZEOKEEL_MIN_CLIENT_VERSION_OFFICIAL=9.9.9` + `..._BETA=0.0.1` 时 header `official` 与 `beta` 均 **426 minimum 9.9.9**（`round6-rereview/probe.log`） |
| P1-3 brand min 死分支/文档承诺 | kit-server 删除 brand 分支；`docs/04:161` 改为“服务端构建渠道 env + global 兜底，客户端头仅排障”；acceptance:22 同步 | `git diff` 核对；`grep min_client_version brands/*/channel.json` 仍无（已不再声称） |
| P1-4 Windows cmd 注入 | `ipc.rs:620-655`：`windows_open_url_invocation` → `rundll32 url.dll,FileProtocolHandler <url>` 固定 argv；`open_platform_browser` 不再经 cmd | 单测 `ipc::tests::open_url_invocation_keeps_url_as_one_arg_without_a_shell` pass；`grep 'cmd' ipc.rs` 无 opener 路径。Windows 运行时仍 **UNVERIFIED**（Linux host） |
| P2-1 portal URL 同源/协议 | `upgrade.ts:118-160`：仅 http(s) + 与 activeServerUrl **同 origin**，否则回退 `<origin>/portal`；无效 server → null | `pnpm --dir apps/desktop/web test` 中 `upgrade.test.tsx` “locks portal URLs to http(s) same-origin…”（javascript:/data:/file:/offsite/protocol-relative 全部回退）pass；desktop 100 tests 0 |
| P2-5 CSP 文档真源 | `docs/21:83` 改为“个人版共享同一静态 CSP（构建期不按 edition 分叉）…不允许任意明文远程 HTTP” | 文档复核：与 `tauri.conf.json:20` 单静态 CSP 一致（`connect-src https:` 的宽泛性作为设计记录，不再声称个人版不同） |

### 8.2 首评后新增 P1-R1（已于 HEAD 7cd0362 关闭）

**P1-R1 · 提交态生成物基线漂移：`apps/server/src/generated/packages.rs` 缺 `pub const CHANNEL`**

- 证据：`git show HEAD:apps/server/src/generated/packages.rs | grep -c CHANNEL` = **0**（生成器 `generated_packages_rs`（`tools/zk/src/main.rs:2313`）现在无条件输出 `CHANNEL`；同批提交已更新 desktop/composed，漏 server）。
- 复现（官方 CI 同命令，干净 archive 同样复现；非 worktree 伪影）：
  `cargo run -q -p zk -- compose --edition personal --profile default --channel official --check`
  → **EXIT 1**，`zk: generated files differ: apps/server/src/generated/packages.rs`。
  在 `git archive HEAD` 解压目录运行同一命令 → 同样 EXIT 1；`zk compose`（写模式）预期唯一 diff：
  `+pub const CHANNEL: &str = "official";`（实测 diff 1 file/1 insertion）。
- 影响：C11/CI compose 门禁在 HEAD 红；acceptance §1 表格仍写 `zk ... compose ... 0`（与事实不符，false-green）。这是提交生成物问题，不是代码缺陷。
- 修法（Lead，1 行生成物）：在 HEAD 用 **`cargo run -q -p zk -- compose --edition personal --profile default --channel official`**（不要用先经 multi-package build 的 `target/debug/zk`，见 P2-N1），提交 `apps/server/src/generated/packages.rs`（预期唯一 diff 就是 `CHANNEL` 行），复跑 `--check` 应 0；随后本记录可转 APPROVED。
- **修复复核（7cd0362）**：`git show HEAD:apps/server/src/generated/packages.rs` 现含 `pub const CHANNEL: &str = "official";`；提交 `7cd0362` 为 `1 file changed, 1 insertion(+ )`（仅该文件）；在干净树与 `git archive HEAD` 解压目录分别运行 `cargo run -q -p zk -- compose --edition personal --profile default --channel official --check` 均 **EXIT 0**（`compose: up to date (16 files)`）。**P1-R1 关闭。**

### 8.3 复评/终评门禁（3c24611；7cd0362 复核，独立执行）

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | **EXIT 0** |
| `cargo clippy --workspace --all-targets -- -D warnings` | **EXIT 0**（`round6-rereview/clippy.log`） |
| `cargo test --workspace` | **EXIT 0**（418 blocks / **386 passed** / 0 failed；含 native_client、app_request_builder、spoof、open_url argv 新测试） |
| `cargo test --workspace export_schema` + `git diff --exit-code` | **EXIT 0 / 0** |
| `bash scripts/schema-check.sh` | **EXIT 0** |
| `zk validate` / `zk capability --check` / `zk docs --check` / `zk db lock --check` / `zk lint --duplicates` | **0 / 0 / 0 / 0 / 0**（capability 45；duplicates 61≤61） |
| **`cargo run -q -p zk -- compose ... --check`** | 3c24611: EXIT 1（仅 server generated CHANNEL，P1-R1）；**7cd0362: EXIT 0**（`compose: up to date (16 files)`，P1-R1 已修复） |
| `python3 scripts/review-check.py` | 3c24611: 1（CHANGES-REQUESTED）；**7cd0362: 0（本记录终评 APPROVED）** |
| `pnpm -r typecheck` / `lint` / `test:unit` / `build` | **0 / 0 / 0 / 0**（desktop web 100 tests + admin 1；`round6-rereview/pnpm-gates.log`） |
| `bash scripts/client-upgrade-e2e.sh`（HEAD，真 426 server/serve） | **EXIT 0**（1 passed；权威渠道修复未破坏 global-min 426 gate） |
| `python3 scripts/docs-check.py` / `spec-check.py` | **0 / 0** |
| `python3 scripts/review-check.py` | **1**（本记录最新结论 CHANGES-REQUESTED，须 P1-R1 修复后转 APPROVED；预期） |
| m1/agent/login/admin/apps/updates/oem e2e | 首评 4eddfcb 均 0；本轮未重跑（修复仅 native identity/portal resolver/Windows opener/权威渠道，client-upgrade 已重跑 0；Lead 复跑全量 e2e 后 record 可更新） |

### 8.4 其余 P2 残留（非阻塞）

- P2-2 426 envelope 未统一：bootstrap（`settings:118-119`）与 updates（`updates:268`）仍返回 flat `{"ok":false,"error":"CLIENT_TOO_OLD"}`；`docs/04:196` 仍写 426 nested envelope（业务 gate 成立，其它 handler 未迁移）。
- P2-3 `request_id` 未写入 audit（`audit_tx`：`crates/zk-kit-server/src/lib.rs:672` INSERT 列不含 request_id；docs/04:196 “全链路”仍 overclaim）。
- P2-4 网关 `/v1/*` 不在 426 覆盖（`is_client_business_path` 仅 `/api/client/v1/*`），强制升级仍不覆盖 AI 调用。
- P2-6 `open_url` 子进程未 reap/失败无 UI 反馈；P2-8 request-id 字符集/非法值静默丢头；P2-9 OEM 目录嵌套外层 git 仓时 fallback 误判；P2-7 native app 路径仍无 e2e（仅单测/builder 断言）。
- **P2-N1（复评新增，非阻塞）**：`zk` 生成 JSON 的 key 顺序取决于 zk 二进制的构建图——先 `cargo build -p zk -p zk-server -p zk-desktop` 再直接跑 `target/debug/zk compose --check` 会把 `shared/contracts/{brand,ui-contributions}.json` 也记为差异（serde_json preserve_order 被统一开启的迹象）；用文档命令 `cargo run -q -p zk` 重建后仅剩 server 差异。CI 用 doc 命令不受影响，但本地/脚本直接调 `target/debug/zk` 会 false-red；建议在 zk 侧显式固定序列化顺序（或统一启用 preserve_order 并重生成基线），并固定检查命令用 `cargo run -p zk`。

### 8.5 复评证据

- `target/evidence/audit/round6-rereview/{clippy,workspace-tests,export-schema,pnpm-gates}.log`
- `target/evidence/audit/round6-rereview/client-upgrade-e2e.log`（EXIT 0，1 passed）
- `target/evidence/audit/round6-rereview/probe.log`（权威渠道 spoof：official/beta header 均 426 min 9.9.9）
- 干净 archive 复现（3c24611）：`git archive HEAD` + 上述 compose 命令 → EXIT 1（仅 `apps/server/src/generated/packages.rs`）。
- 复评 UNVERIFIED：Windows rundll32 运行时；Tauri 真机（CSP/open_url/native app window 可见性）；真实 OEM 私有索引/submodule；Docker/安装包/notarize。上述为环境项，不构成代码缺陷。

### 8.6 终评（HEAD 7cd0362）

- 修复提交：`7cd0362 build(compose): commit server CHANNEL const in generated baseline`（`apps/server/src/generated/packages.rs | 1 +`，与 P1-R1 修法一致）。
- 干净树复核（本机，HEAD 7cd0362）：
  - `cargo run -q -p zk -- compose --edition personal --profile default --channel official --check` → **EXIT 0**（`compose: up to date (16 files)`）；`git archive HEAD` 解压目录同样 **EXIT 0**。
  - `git show HEAD:apps/server/src/generated/packages.rs` 含 `pub const CHANNEL: &str = "official";`。
  - `python3 scripts/review-check.py` → **0（ok）**；`docs-check.py` / `spec-check.py` → **0 / 0**；`git status --porcelain` 仅本记录未跟踪。
  - 7cd0362 仅新增 1 行生成物，3c24611 已独立复跑的 fmt/clippy/workspace tests/export_schema/schema-check/zk gates/pnpm gates/client-upgrade e2e 结论不变（代码未改）。
- P2-N1（zk JSON key 顺序依赖构建图）记录为 **round-7 后续项**（Lead 已确认 serialization determinism 下一轮处理），非 round-6 blocker；§8.4 其余 P2 为非阻塞残留。
- 终评结论：**APPROVED（HEAD 7cd0362；P0=0 / P1=0；P2=9 中 2 项已闭环，其余记录在案）**。
