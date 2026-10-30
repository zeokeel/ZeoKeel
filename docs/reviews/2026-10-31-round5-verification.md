# Round-5 独立评审（426 升级闸门 / 强制升级 UX / e2e / OEM pin）

- 评审对象：`HEAD 9e2ae62`（round-5 提交 `8f1aba7`、`aeff13d`、`9e2ae62`；base `b4f4b44` = round-4 APPROVED）。评审时工作树 clean（`git status --short` 为空）；本记录是唯一新增文件。
- 评审者：auditor-rig（独立评审）。
- 与实现者关系/独立性：round-5 分别由 auditor-rust（服务端 426 层）、auditor-web（web/e2e）、Lead（OEM pin/docs）实现；评审者未参与任何 round-5 代码，只作为 task-44 只读评审。
- 关联 spec：`specs/changes/2026-10-31-round5-upgrade-and-e2e`（spec/design/tasks/acceptance；proposal 已链接本记录）。
- 评审日期：2026-10-31
- 结论：**APPROVED**（P0 = 0；P1 = 0；P2 = 10，均为非阻塞澄清/加固/证据项）
- 复评：首评，无前次记录（本记录即 round-5 的 C19 记录）。

## 1. 评审范围

- 业务面 426：`packages/server/http/src/lib.rs`（`is_client_business_path` / `client_version_gate` / `upgrade_required`）、`apps/server/src/main.rs`（layer 挂载、CORS `allow_headers`）。
- 客户端身份与强制升级：`apps/desktop/web/src/{client,upgrade,UpgradeGate,App}.tsx?`、`auth.ts`、`store.ts`、`transport/http.ts`、`state/{controller,reducer}.ts`、`generated/brand.ts`、`tools/zk/src/main.rs`（生成 `clientVersion`）。
- e2e：`e2e/serve-m1.sh`、`e2e/m1-serve.spec.ts`、`e2e/m1-serve-security.spec.ts`、`playwright.config.ts`、`scripts/m1-e2e.sh` 及 apps/admin/login/updates/oem 脚本的客户端头更新。
- OEM pin：`tools/zk/src/main.rs`（gitlink / dir-hash 双路径、`desktop_version`、pin 单测）、`docs/09`。
- 文档真源：`docs/04`（业务面 426、错误模型、客户端头）、`docs/09`（pin/门户）、`docs/19`（升级门禁动作）。

## 2. 门禁复跑（独立执行；均为 EXIT 0）

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | 0 |
| `cargo test --workspace --offline` | 0（418 条 `test result: ok`，无 failed/error） |
| `cargo test --workspace export_schema --offline` + `git status --short` | 0；生成物无 diff（确定性） |
| `zk validate` / `zk compose --edition personal --profile default --channel official --check` / `zk capability --check` / `zk docs --check` / `zk db lock --check` / `zk lint duplicates` | 0 / 0 / 0 / 0 / 0 / 0（`lint: ok`） |
| `bash scripts/schema-check.sh` / `python3 scripts/docs-check.py` / `python3 scripts/spec-check.py` | 0 / 0 / 0 |
| `pnpm -r typecheck` / `lint` / `test:unit` / `build` | 0 / 0 / 0 / 0（desktop web 83 tests） |
| `bash scripts/m1-e2e.sh`（mock profile → restore → real serve/SQLite） | 0；serve **7 passed**（usage reload、真实 git changes/空态在内） |
| `bash scripts/agent-e2e.sh` | 0；**9 passed** |
| `bash scripts/login-e2e.sh` | 0；**4 passed**（enterprise 登录 + 客户端头 + dev CORS 预检路径） |
| `bash scripts/admin-e2e.sh` | 0；**11 passed** |
| `bash scripts/apps-e2e.sh` / `bash scripts/updates-e2e.sh` / `bash scripts/oem-e2e.sh` | 0 / 0 / 0 |
| 真实 PG 426 冒烟（临时 DB/端口，cleanup 已回收；`target/debug/zk-server`） | 见 §3.1 |

`review-check.py` 在写本记录前报 1 条（round-5 proposal 的 review record 不存在）；本记录写入后应转 0（Lead 复跑确认）。

## 3. 结论依据

### 3.1 业务面 426（正确性/安全/预检）

真实 PG + 真实 server 冒烟（curl，`/api/client/v1/skills`）：

- 缺 `X-ZeoKeel-Client` → **426** + `{"error":{"code":"CLIENT_TOO_OLD","message":…,"current":null,"minimum":"0.1.0","portal_url":"/portal"}}`（fail-closed，符合 spec §1 选择）。
- `0.0.1` 头 → **426** `CLIENT_TOO_OLD`；`0.1.0` 头（无 JWT）→ **401**（说明中间件放行后进入原有鉴权，未误杀）。
- 挂载真实生效（`apps/server/src/main.rs` 合并后统一 `layer`），路径前缀判断只看 `/api/client/v1/*`；`/api/meta/*`、`/api/channel`、`/api/updates/*`、`/healthz`、`/admin` 不在前缀内。
- 豁免：`auth/login`、`auth/refresh`、`config/bootstrap` 精确匹配；`config/bootstrap` 由 handler 用 `client_is_too_old` 再做一次（缺失/非法头放行以满足首配，合法旧版本 426），与 docs/04 §2.2 的 `CLIENT_TOO_OLD` 一致。
- OPTIONS 显式旁路；dev CORS `allow_headers` 加入 `x-zeokeel-client`；login-e2e 的浏览器路径（自定义头触发预检）通过，证明 CORS 层与 gate 顺序正确。无 dev CORS 时 OPTIONS 返回 405（浏览器不跨域；Tauri 见 P2-9）。
- 头值仅作版本判定、非授权（docs/04 明确“recorded for 灰度/排障, not authorization”）；伪造高版本可绕过 426 是设计内（C6 弱管控），不是安全边界。

### 3.2 客户端身份 + clientVersion + 强制升级 UX

- `tools/zk` 生成 `BRAND.clientVersion`（apps/desktop/Cargo.toml → workspace version，当前 0.1.0）；`compose --check` 绿说明生成物与生成器一致。
- `client.ts` 对所有 web 直连 server 的 fetch 注入 `X-ZeoKeel-Client: <semver>;<brand.channel>;desktop;<device_id>`（auth bootstrap/login/refresh、skills/connectors/apps/archive/open/assets/install/tombstones、transport 426 读取）。
- 426 → `UpgradeRequiredError`（`readJsonOrUpgrade`/transport），`App` 顶层 `state.upgrade` 优先于 boot/login 渲染 `UpgradeGate`：全屏 `role=alert`、无关闭/重试按钮、portal 链接（`server_url + /portal` 兜底）、zh/en i18n；vitest 覆盖解析（flat/nested/空 body）、门禁渲染、不重试。
- 真实 server 的 426 body（bootstrap handler 的 flat `{ok:false,error:"CLIENT_TOO_OLD"}`）由 `parseUpgradeInfo` 兼容；gate 不依赖具体 body 形状（426 状态本身即升级信号）。

### 3.3 e2e 真实性

- `scripts/m1-e2e.sh` 先 mock-profile（`ZEOKEEL_E2E_SKIP_SERVE=1`）后 `restore-baseline` 再跑 personal serve：避免 mock compose 的 agent-loop 污染 serve 的 agent-rig/usage 证据。
- `e2e/serve-m1.sh`：真实 `zk-desktop --serve` + 临时 `ZEOKEEL_HOME` + 真实 SQLite（sessions.db）+ 真实 git 仓库 workspace + 确定性假上游（无外网）；`m1-serve.spec` 断言发送→工具卡→usage（输入10/输出5/合计15）→ **reload 后 usage 仍在**（真实 SQLite 持久化，非 mock）。
- changes tab 测试真实修改/新增/删除仓库文件并断言 inspector 列表，随后恢复仓库断言空态；`workspace.changes` 实现只读、固定 argv、无 shell、5s/2MiB/500 条上限（安全）。
- security spec 在共享 serve 进程内把 provider 写回假上游，避免污染后续 chat 套件（说明测试共享状态，见 P2-10）。
- apps/updates/oem/login/admin/agent e2e 复跑均 0（含 OEM portal 品牌与 pin/fallback、login CORS/登录、admin 11 tests）。

### 3.4 OEM pin gitlink / fallback

- `channel_validate`：当 index 根是 git 仓库且子模块被跟踪为 mode 160000 → 读 `git ls-files -s` gitlink 作为 pin；否则回退目录哈希，错误信息明确标注 fallback 模式（`tools/zk/src/main.rs:1625-1640`、`gitlink_hash:1828`）。
- 单测用 `git update-index --cacheinfo 160000` 造 gitlink 验证 helper；oem-e2e 走 fixture 目录哈希回退 + compose/brand/residue。docs/09 已更新为双路径真源。
- 生产路径（真实私有索引仓 + submodule）仍无自动化 e2e（无网络/私有仓，见 UNVERIFIED），但代码路径与文档一致。

## 4. P0 / P1 / P2

**P0：无。**

**P1：无。**

**P2（建议修复；不阻塞本轮 APPROVED）**

| # | 位置 | 问题 | 修法 |
| --- | --- | --- | --- |
| P2-1 | `apps/desktop/web/src/store.ts:43,63,99,115,134,141,158,174`；`upgrade.ts:96-106` | skills/connectors/apps/asset 等直连 fetch 的 426 只抛 `Error("… HTTP 426")`，`upgradeInfoOf` 只按 `CLIENT_TOO_OLD` 文本识别 → 这些 426 会落到 contentError 而不是 `ui.gate.update`（正常“旧客户端”场景由 bootstrap 426 兜底触发 gate，故非阻塞；但 spec §2“transport 统一 426 → 强制升级”未覆盖这些入口） | 所有 server 直连 fetch 走 `readJsonOrUpgrade`/统一 426 映射；补 store 426→gate 单测 |
| P2-2 | `packages/server/http/src/lib.rs:49`；`apps/desktop/web/src/UpgradeGate.tsx:9`；`web/tests/upgrade.test.tsx` | gate 的 426 body `portal_url` 是相对 `/portal`，客户端优先直接使用 `info.portalUrl`（只有缺省才用 `server_url + /portal`）；现有单测只覆盖绝对 URL/兜底。当前路径下 nested body 未被 gate 使用（见 P2-1），故为潜在缺陷 | 客户端把相对 portal_url 解析到 `activeServerUrl`；补相对 body 用例；或服务端返回绝对 URL |
| P2-3 | `packages/server/http/src/lib.rs:42-51`；`packages/server/settings/src/lib.rs:118-119`；docs/04 §7 | 426 body 未按统一错误信封带 `retryable`/`request_id`，且业务面 gate 是 nested `{error:{...}}` 而其它 handler 是 flat `{ok:false,error}`；客户端容错已覆盖，但 docs“统一信封”仍不符 | 网关/业务面统一 envelope（带 request_id/retryable），或 docs/04 明确 426 例外 |
| P2-4 | `apps/desktop/web/src/client.ts:20`；docs/04:46；`crates/zk-kit-server/src/lib.rs:399` | 头里 platform 固定为 `desktop`（docs 示例是 `linux-x64` 等），且仍未带 `X-Request-Id`；服务端只消费版本的 channel，暂不影响 426 | 生成真实平台 id（构建期/运行时）并统一 request-id；或 docs 放宽格式 |
| P2-5 | docs/19:25；`UpgradeGate.tsx` | docs/19 门禁允许“下载更新、重试、查看版本说明”，实现只有 portal 链接（无重试/版本说明）；spec §2 又要求“不重试业务请求”，文档需与实现/意图对齐 | 增加“检查更新”/版本说明，或收窄 docs/19 为“打开下载页”（并说明不重试业务请求） |
| P2-6 | docs/04:161；`crates/zk-kit-server/src/lib.rs:377` | docs 说 `min_client_version` 按渠道配置；实现是进程级 `ZEOKEEL_MIN_CLIENT_VERSION`（全局） | 要么接渠道设置（server settings），要么 docs 改为“服务端全局 env，渠道由发布元数据表达” |
| P2-7 | `e2e/`、`web/tests/upgrade.test.tsx` | 没有真实 server 的 426 e2e：e2e 脚本都带合法头，gate 永不触发；强制升级只有 vitest 单测（proposal 成功指标写“单测 + e2e”） | 新增一个 serve/enterprise e2e：`ZEOKEEL_MIN_CLIENT_VERSION=9.9.9` 启动 server，断言 426→gate→portal 链接（不重试） |
| P2-8 | `tools/zk/src/main.rs:1625-1640,1828,2868`；docs/09:91 | gitlink 生产路径只有 helper 单测；`channel_validate` 的 gitlink 接受/拒绝未测；目录哈希回退对任意非 gitlink 索引生效，docs 称“仅 demo/测试”但代码无 allowlist；SHA-256 仓（64 hex）会静默回退 | 加 `channel_validate` 双路径测试（gitlink 匹配/不匹配、回退标志）；回退时输出显式 warning/仅 fixture flag |
| P2-9 | `apps/desktop/tauri.conf.json:20`；`apps/desktop/capabilities/main.json`；`docs/21:83`；`apps/desktop/web/src/{auth,store}.ts` | shipped Tauri 壳的 CSP `connect-src ipc: http://ipc.localhost` 未含 server/upstream（与 docs/21 冲突），而 web 直接 fetch 登录服务器；capabilities 无 opener/shell，`UpgradeGate` 的 `target=_blank` 外链在 Tauri 中不可打开。round-5 的客户端身份/426 gate 证据全部来自 browser/serve 载体 | 明确 Tauri 企业网络路径（CSP 加 server 域或 host 代理 + CORS），接 external-open（plugin/命令），补 Tauri 路径验证；若本轮不做，acceptance 的 UNVERIFIED 必须显式包含“Tauri 真机 enterprise 登录/升级” |
| P2-10 | `apps/desktop/src/changes.rs:15-24`；`e2e/m1-serve.spec.ts:53-75` | changes 是**文件级粗粒度** add/del（非行数），e2e 断言 `+1 −0` 的字面像行数；文档应明确“文件级计数/无行级 diff”，避免后续误读 | docs/27 或 contracts 注释写明 coarse 语义；UI 文案可改“文件变更数” |

## 5. UNVERIFIED（本轮/环境限制，不构成本轮缺陷）

- 真实签名密钥/证书、Docker/Caddy/镜像、三平台安装包、真实 OEM 私有索引仓 + submodule/deploy-key（无网络/无 Docker）。
- Tauri 真机（企业登录、客户端头、426 gate、外链打开）——所有证据来自 browser/serve 载体（与 P2-9 对应，需在 acceptance UNVERIFIED 说明）。
- `cargo-deny` 本地、`zk api` 路由真源、真实超大规模仓库 changes、`login-e2e` 之外的 TLS 模式。

## 6. 复评与状态

- 本轮无 P0/P1，P2 均为非阻塞项；结论见首部“结论：APPROVED”（review-check 以最后一处 `结论` 行为准）。
- acceptance 状态仍为 `in-progress`；Lead 可在回填本记录链接/UNVERIFIED 后改为 done（本评审不修改 acceptance，仅记录）。
- 若后续修复 P2-1/2/7/9 涉及产品代码，需要新一轮 C19 复评。
