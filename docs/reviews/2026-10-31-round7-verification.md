# Round-7 独立评审（异步子代理 / 生成物确定性 / 426+审计 / open_url / matrix）

- 评审对象：`HEAD 25d1712`（round-7 提交 `2717fe5` stable_json → `c2ff21d` async subagents → `2066e77` nested 426+audit request_id → `25d1712` updates 证据；base `7cd0362` = round-6 APPROVED）。评审开始时工作树 clean（`git status --porcelain` 为空）；本记录是唯一新增文件。
- 评审者：**auditor-build**（独立评审；round-7 未实现任何被评审代码；仅上轮 task-53 评审者）。
- 独立性：round-7 由 Lead 与 auditor-rig / auditor-rust / auditor-web 实现（proposal/commits）；评审者未参与本提交。
- 关联 spec：`specs/changes/2026-10-31-round7-async-and-determinism`（proposal/spec/design/tasks/acceptance；proposal 已链接本记录）。
- 评审日期：2026-10-31
- 结论：**APPROVED**（P0 = 0；P1 = 0；P2 = 9，均为非阻塞加固/证据/文档项）。核心验收——异步子代理 API+工具、stable_json 多构建图确定性、bootstrap/updates 嵌套 426 + request-id 回显 + 审计落库、open_url reap、web 打开失败 UX——均经独立复跑/真实 PG 现场验证成立；异步资源表与文档真源有少量 P2 残留，不影响本轮合并。

## 1. 评审范围

- 异步子代理：`crates/zk-kit-subagent/src/lib.rs`（`AsyncAgentStatus`/`start/status/result/interrupt`/registry/MAX_ASYNC_AGENTS）、`packages/client/agent-rig/src/lib.rs`（`RigSubagentRunner` 后台线程、共享取消、并发/深度、结果注册表）、`packages/client/tool-subagent`（`subagent_start` + `extra_tools`）、`packages/client/tool-subagent-control`（`list_agents`/`subagent_status`/`subagent_interrupt` + `extra_tools`）。
- 生成物确定性：`tools/zk/src/main.rs` `stable_json`/`sort_json_keys` 及全部 JSON writer；多包构建后的 `target/debug/zk compose/db lock/capability --check`；committed baseline `shared/contracts/{packages,roles}.json`、`zk.lock`。
- extra_tools 生成约定：`tools/zk` `compose_lib` 源标记扫描、`crates/zk-composed/src/lib.rs` 生成 glue、docs/38 matrix。
- 426/审计：`crates/zk-kit-server` `request_id_from_headers`/`with_request_id`/`current_request_id`/`upgrade_required_response`/`audit_tx`；`packages/server/http` gate、`settings` bootstrap、`updates` manifest；`docs/04`/`docs/21`。
- open_url / web：`apps/desktop/src/ipc.rs` `spawn_and_reap`；`apps/desktop/web/src/{UpgradeGate,upgrade,i18n}.tsx?`；`apps/desktop/web/tests/upgrade.test.tsx`。
- 文档真源与 false-green：`docs/38`、`docs/generated/AGENT-TOOL-MATRIX.md`、`scripts/agent-tool-matrix.py`、`docs/24`、acceptance。

## 2. 门禁复跑（独立执行；均在本机真实运行）

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | **EXIT 0** |
| `cargo clippy --workspace --all-targets -- -D warnings` | **EXIT 0**（`target/evidence/audit/round7/clippy.log`） |
| `cargo test --workspace` | **EXIT 0**（418 test-result 块，**403 passed / 0 failed**；含 `async_start_status_and_result`、`async_interrupt_marks_cancelled`、`async_concurrency_cap_is_enforced`、`async_depth_limit_is_enforced`、`stable_json_tests`、`open_url_child_is_spawned_and_reaped_without_waiting`、`open_url_spawn_failure_is_returned`、`request_context_tests`、`manifest_below_min_returns_nested_426_envelope`） |
| `cargo test --workspace export_schema` + `git diff --exit-code` | **EXIT 0 / 0** |
| `bash scripts/schema-check.sh` | **EXIT 0** |
| `zk validate` | **EXIT 0**（114 packages） |
| **`cargo run -q -p zk -- compose --edition personal --profile default --channel official --check`** | **EXIT 0**（16 files；多包构建后同样 0） |
| **`cargo build -p zk -p zk-server -p zk-desktop` 后直接 `target/debug/zk compose --check` / `db lock --check` / `capability --check`** | **0 / 0 / 0**（P2-N1 修复的实测关键；见 §3.2） |
| `zk docs --check` / `zk db lock --check` / `zk lint --duplicates` | **0 / 0 / 0**（45 capabilities；duplicates 61≤61） |
| `pnpm -r typecheck` / `lint` / `test:unit` / `build` | **0 / 0 / 0 / 0**（desktop web **104** tests + admin 1；`round7/pnpm-and-schema.log`） |
| `bash scripts/updates-e2e.sh` / `agent-e2e.sh` / `m1-e2e.sh` | **0 / 0 / 0**（updates nested 426 断言、agent **9 passed**、m1 mock+serve **7+7 passed**） |
| `bash scripts/admin-e2e.sh` / `login-e2e.sh` / `client-upgrade-e2e.sh` | **0 / 0 / 0**（admin **11 passed**；login **1+3 passed**；真实 426 gate **1 passed**） |
| `python3 scripts/docs-check.py` / `spec-check.py` | **0 / 0** |
| `python3 scripts/review-check.py`（写本记录前） | **1**（缺本记录，预期）；写入后见 §7 |
| 真实 PG 现场探测 | business/bootstrap/updates 嵌套 426 + request-id 回显 + app.upload 审计 request_id（`round7/probe.log`/`probe-audit.log`） |

## 3. 结论依据

### 3.1 异步子代理（API / 线程 / 取消 / 并发 / 结果 / 兼容）

- kit 契约（`crates/zk-kit-subagent`）：`AsyncAgentStatus`；`SubagentRunner` 默认方法 `start/status/result/interrupt`（不支持 async 时诚实 Err，不假装成功）；`MAX_DEPTH=3`/`MAX_ASYNC_AGENTS=8`；registry `register/finish/mark_cancelled/list`；深度按 conversation id 传递（新线程下工具回调可恢复）。
- agent-rig：`RigSubagentRunner` 持有 `children: Arc<Mutex<HashMap<String, AsyncChild>>>`；`start` 在锁内检查并发上限并注册，后台线程用 `RigAgent::new(...).with_cancel_flag(flag)` 运行同一 run 逻辑；`interrupt` 置共享 `Arc<AtomicBool>` + 状态；完成写 `done/failed/cancelled` + result；`RigAgent` 的 `external_cancel` 防止 turn 开始时清掉早到的 interrupt。同步 `run/fork` 路径保持 join 语义。
- 工具：`tool-subagent` 的 `subagent_start{prompt}` 追加；`tool-subagent-control` 的 `list_agents`（sync+async、`max_async`）、`subagent_status{id}`（状态+result ok/error）、`subagent_interrupt{id}`；无 runner/unknown id/已完成均返回明确错误。
- 单测（403 passed 内）：start→running→done、interrupt→cancelled、并发上限、深度限制、默认 runner 诚实降级、list_agents async、工具 start/status/interrupt；`scripts/agent-e2e.sh` 9 passed 证明同步 subagent/fork 与既有 turn 未回归。
- P2 残留见 §4（并发上限在 cancel 后计数、panic/registry 生命周期、interrupt/completion race、async e2e 缺失）。

### 3.2 stable_json / P2-N1 确定性

- `tools/zk/src/main.rs:2091` `stable_json`：`serde_json::to_value` → 递归 `sort_json_keys`（对象转 BTreeMap，数组保序）→ pretty。`grep to_string_pretty` 仅剩 `stable_json` 内部一处；brand/ui-contributions/packages/roles/zk.lock/capability/lint baseline 均走该函数。
- 实测（关键）：先 `cargo build -p zk -p zk-server -p zk-desktop`（features 可能被统一，正是 round-6 P2-N1 复现条件），再直接运行 **`target/debug/zk`**：`compose --check`、`db lock --check`、`capability --check` 均 **EXIT 0**；随后 `cargo run -q -p zk -- compose --check` 也 **0**。committed `shared/contracts/packages.json`/`roles.json`/`zk.lock` 与生成结果一致（`git diff` clean）。
- 单测 `stable_json_tests::object_keys_are_sorted_recursively` pass；多构建图确定性由上述外部复跑覆盖。**P2-N1 闭环。**

### 3.3 extra_tools 生成约定

- `tools/zk` `compose_lib` 对每个选中的工具包读取 `src/lib.rs`，若含 `pub fn extra_tools` 则在主工具 vec 后追加 `tools.extend(<crate>::extra_tools());`（`tools/zk/src/main.rs:1471-1480`）。`crates/zk-composed/src/lib.rs:101-102` 生成基线包含 `zk_tool_subagent::extra_tools()` 与 `zk_tool_subagent_control::extra_tools()`；`compose --check` 0，证明 glue 与约定一致；工具包内 `extra_tools()` 返回 start/status/interrupt。
- 这是"源标记 + 生成 glue"约定，可工作且有 docs/38 说明；但存在隐式/静默风险（P2-4）。

### 3.4 426 嵌套 envelope / request-id / 审计

- kit 统一 helper：`request_id_from_headers`（缺失/非法 ≤128 时生成 UUID）、`with_request_id` task-local、`current_request_id`、`upgrade_required_response`（嵌套 `error{code,message,retryable:false,request_id,current,minimum,portal_url}` + 回显 `X-Request-Id`）；http gate/bootstrap/updates 共用，无第二实现。
- 真实 PG 现场（enterprise server + 临时 DB，`round7/probe.log`）：
  - business `/api/client/v1/skills` + old header → **426** nested，`x-request-id: probe-biz-rid`，body `request_id/current=0.1.0/minimum=9.9.9/portal_url=/portal`；
  - bootstrap `/api/client/v1/config/bootstrap` + old header + JWT → **426** nested，回显 `probe-boot-rid`；
  - updates `/api/updates/manifest` + old client `0.0.9` → **426** nested，回显 `probe-upd-rid`，`current=0.0.9/minimum=0.1.0`；
  - `updates-e2e.sh` 也已断言 nested `error.code` + `request_id`。
- 审计落库：`audit_tx` 现写入 `request_id` 列（kit:764），从 gate 的 task-local 读取。真实 PG 验证（`round7/probe-audit.log`）：app 上传带 `X-Request-Id: probe-audit-rid` → `audit_log` 行 `action=app.upload, request_id=probe-audit-rid`；`WHERE request_id='probe-audit-rid'` count=1；该行 hash 存在；链链接检查 0 violations（单行场景，完整链由既有 srv-auth-test 覆盖）。**不是 fake id、不是空串。**
- Web 兼容：`upgrade.ts` 解析 nested/flat，`requestId` 透出；`UpgradeGate` 显示 request ID（details）；测试覆盖 legacy flat、nested、request_id、offsite/javascript portal 回退。
- P2 残留：gateway 另生成 request_id、审计 hash 未覆盖 request_id/actor、无 header 时 gate 与 bootstrap/updates 各生成一次 id；见 §4。

### 3.5 open_url reap / 失败 UX

- `apps/desktop/src/ipc.rs:649` `spawn_and_reap`：spawn 失败返回 `open-url/launch-failed`；成功后 detached 线程 `child.wait()` 回收（避免 zombie），不阻塞 IPC。Unix 单测 `open_url_child_is_spawned_and_reaped_without_waiting`、`open_url_spawn_failure_is_returned` pass；Windows 仍用固定 argv `rundll32 ...`（无 cmd）。
- Web：`UpgradeGate` 对 Tauri 分支 `await transport.call("system.open_url")`，失败 set `role=status` 的 `upgradeOpenFailed`；浏览器 `window.open` 返回 null/throw 同样提示；不阻塞全屏 gate。`upgrade.test.tsx` 覆盖 Tauri 打开、浏览器打开、失败 toast；desktop web 104 tests 0。

### 3.6 docs/38 / matrix / docs truth

- docs/38 已新增异步子代理行（工具/状态/并发 8/深度 3/extra_tools/诚实降级），并更新已实现工具列表；`docs/generated/AGENT-TOOL-MATRIX.md` 中 subagent/subagent-control 行与实现一致（`subagent_start`、`subagent_status`、`subagent_interrupt`、cap 8）；重跑 `python3 scripts/agent-tool-matrix.py` 无 diff；`docs-check`/`spec-check` 0。
- P2 残留：matrix writer/CI 与 docs/24 的 one-path-one-writer 表存在不一致；见 §4。

### 3.7 false-green 评估

- acceptance 的门禁/e2e 声明经独立复跑成立：fmt/clippy/workspace tests、export_schema+diff、schema、zk gates（含多包构建 compose/db lock）、pnpm 全部 0；m1/agent/login/admin/client-upgrade e2e 全绿（updates 额外复跑 0）；真实 PG 426/审计现场验证成立。
- 唯一未覆盖项：**async 子代理没有 e2e/真实 turn 集成**（只有 kit/agent-rig/tool 单测；agent-e2e 是同步流程）——acceptance 未声称 async e2e，但 §4 P2-9 记录；**并发上限测试是 registry 假数据**（非 cancel 竞态），见 P2-1。
- matrix 无 `--check`/CI diff 门禁，属于文档 false-green 风险（P2-8），不是本轮代码 false-green。

## 4. P0 / P1 / P2

**P0：无。**
**P1：无。**

**P2（建议修复/记录，非阻塞）**

| # | 位置 | 问题 | 修法 |
| --- | --- | --- | --- |
| P2-1 | `packages/client/agent-rig/src/lib.rs:972-978`（只数 `status=="running"`）、`:1077-1078`（interrupt 立即置 `cancelled`）；`:1902-1921`（测试用假 registry 条目）；docs/38:28/56 声称“并发上限 8” | **并发上限可被 interrupt 绕过**：cancel 后线程可能仍在 model/tool 中运行，但状态已非 running、槽位立即释放；反复 start/interrupt 可让实际存活子线程 >8。测试只填假 running 条目，未覆盖 cancel→start 竞态。 | 以“存活线程/JoinHandle”计数（或 cancel 后保留槽位直到线程真正退出）并保持状态机；补 start−interrupt−start 竞态测试；文档注明计数口径。 |
| P2-2 | `agent-rig/src/lib.rs:1004-1046`（spawn closure 无 panic 防护；`clear_context_depth` 只在正常路径）；`children` map 从不删除已完成条目；kit `AGENTS` 列表只增不减 | 子线程 panic 会让状态永久 `running`（槽位/深度 context 泄漏）；每个 async child 的 prompt/结果永久保留，长会话内存无界增长。 | `catch_unwind`/RAII guard 在 panic 时置 failed、清深度；registry 加完成淘汰/上限与指标；panic 测试。 |
| P2-3 | `agent-rig/src/lib.rs:1017`（线程读 `cancelled`）vs `:1078`（interrupt 写 cancelled）与 `:1038-1044`（线程写回 status） | **interrupt/completion race**：interrupt 返回 Ok 后，线程若在 interrupt 前已读取 flag=false，会再把状态写成 done/failed，覆盖 `cancelled`；“interrupt→cancelled”不是严格保证。 | 在 children 锁内复核 flag/状态再写回；或使用 Generation 状态机；补 race 测试。 |
| P2-4 | `tools/zk/src/main.rs:1471-1481`；`docs/24-composition-and-runtime-spec.md:251`；docs/38:56 | **extra_tools 生成约定是隐式源文本标记**：只读 `src/lib.rs`、`unwrap_or_default()` 静默；移动/拆分源文件或拼写错误会静默不追加工具；未在 `zk.toml`/docs/24 writer 输入中声明，`zk validate` 无法校验。 | 在 manifest 声明 extra tools（或 `zk validate` 检查标记唯一性）；缺文件/标记异常时报错；在 docs/24 §3 writer 表登记该输入。 |
| P2-5 | `crates/zk-kit-server/src/lib.rs:762-764`；`docs/21-security-privacy.md` §7 | **审计哈希链只覆盖 prev+action+target+canonical(detail)**；round-7 新持久化的 `request_id`（以及 actor/ip/device）不在 hash 内，篡改 `request_id`/actor 不破坏链；docs/21 把 request_id 列为记录字段且称哈希链必选，完整性口径需澄清。 | 将 request_id（及 actor/ip/device 等完整性字段）纳入 canonical hash；或文档明确“哈希链只覆盖 action/target/detail，request_id 为关联字段”；如需升级链格式，按 docs/21 轮换/WORM 方案。 |
| P2-6 | `packages/server/gateway/src/lib.rs:105`（gateway 自生成 UUID 给 wallet_holds/usage）；`docs/04-contracts-and-api.md:196` “request_id 全链路透传” | gateway 的计费/usage request_id 与 HTTP/审计 request_id 不同；支持无法用同一个 id 串起 audit/usage/hold，docs“全链路”不严格成立。 | gateway 从 `request_id_from_headers`/task-local 取并贯穿 hold/usage/audit；或文档收窄“审计 request_id 透传，计费 request_id 为独立结算 id”。 |
| P2-7 | `packages/server/http/src/lib.rs:53`（gate 生成 request id A 并 task-local）；`settings/src/lib.rs:117-127`、`updates/src/lib.rs:266-279`（无 header 时各自再生成 B） | 无 `X-Request-Id` 的 bootstrap/updates 426 会生成两次 id（A 进入 task-local 但无审计；B 进入 body/header）；日志与响应可能不一致。 | 从 `current_request_id()`（task-local）取 gate 已生成的 id，避免 handler 二次生成；补无 header 测试。 |
| P2-8 | `docs/24-composition-and-runtime-spec.md:251`（`docs/generated/*.md` writer=`zk docs`）vs `docs/generated/README.md:12`（AGENT-TOOL-MATRIX writer=`scripts/agent-tool-matrix.py`）vs `docs/38:34`（“以 matrix 为准”）；CI 无 matrix `--check` | **一路径一 writer 冲突 + matrix 无 CI diff 门禁**：docs/24 表说 docs/generated/*.md 归 zk docs（实际只查 2 个文件）；README 说 matrix 归脚本且“不是产品 SSOT”；docs/38 却把 matrix 当 truth。matrix 可静默漂移（false-green）。 | 统一 writer 归属与 SSOT 表述；给 `scripts/agent-tool-matrix.py` 加 `--check` 并纳入 CI（或把 matrix 改由 `zk docs` 生成）；更新 docs/24 表。 |
| P2-9 | `scripts/agent-e2e.sh`（9 tests 同步流程）；`e2e/` 无 async 子代理用例；acceptance:13/16 只列 agent-rig/tools tests | **async 子代理无端到端/真实 turn 覆盖**：start/status/interrupt 只在单测与 fake runner 工具测试中验证；真实 Runtime + 模型 + 工具循环里的线程/取消/结果未被 e2e 覆盖。 | 新增一个 mock 模型 e2e/集成：subagent_start → 轮询 status → result 成功、interrupt 取消、并发上限；将结果列入 acceptance 证据。 |

## 5. UNVERIFIED（环境限制，不计入本轮缺陷）

- Tauri 真机 / Windows 运行时（open_url 实际打开、reap、web gate toast 在真机显示；Windows rundll32 路径仅单测/静态）。
- 真实媒体 provider、Docker/Caddy/镜像/多平台安装包、cargo-deny/audit（无网络）。
- 真实 OEM 私有索引/submodule/deploy-key。
- 真实大模型长时间异步子代理（真实 provider 网络取消时延）与超长会话 registry 压测；P2-1/P2-2 为代码级判定。

## 6. 证据路径

- 门禁：`target/evidence/audit/round7/{clippy,workspace-tests,pnpm-and-schema}.log`、`target/evidence/audit/round7-fmt.log`
- e2e：`target/evidence/audit/round7/{updates-e2e,agent-e2e,m1-e2e,admin-e2e,login-e2e,client-upgrade-e2e}.log`
- 真实 PG 探测：`target/evidence/audit/round7/probe.log`（enterprise compose + 业务/bootstrap/updates 426 nested/request-id），`probe-audit.log`（app.upload + `X-Request-Id` → audit_log.request_id + hash 链检查）。
- 多构建图确定性：见 §2 行 `cargo build -p zk -p zk-server -p zk-desktop` 后 `target/debug/zk` checks 0（终端记录/本轮复跑）。

## 7. 结论与状态

- 本记录为 round-7 **首评**；结论 **APPROVED**（P0=0/P1=0/P2=9）。
- 写本记录前 `review-check.py` 报 1（缺本记录）；写入后应为 **0**（proposal 状态 approved + latest conclusion APPROVED）。
- P2-N1（round-6 遗留）已闭环；P2 列表均为非阻塞加固/证据/文档项，其中 P2-1/P2-3 建议作为下一轮 async 加固优先项。
- 本记录是唯一新增文件；未修改任何产品代码/生成物/committed 文件。
