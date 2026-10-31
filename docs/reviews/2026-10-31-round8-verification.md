# Round-8 独立评审（异步加固 / 单 request-id / gateway 例外 / matrix SSOT）

- 评审对象：`HEAD 01590c9`（首评；round-8 提交 `a740400` matrix/docs → `00f972c` async 加固 → `4b455f3` request-id/gateway 尝试 → `01590c9` gateway /v1 例外回退并记录；base `c980082` = round-7 APPROVED docs）。**复评对象：`HEAD a09a1e8`（`6f7701f` gateway server-generated settlement ids + `a09a1e8` matrix SSOT docs；父 01590c9；详见 §8）**。复评开始时工作树 clean（唯一未跟踪文件即本记录）。
- 评审者：**auditor-build**（独立评审；round-8 未实现任何被评审代码；仅上轮 task-53 评审者）。
- 独立性：round-8 由 Lead 与 auditor-rig / auditor-rust 实现（proposal/commits）；评审者未参与本提交。
- 关联 spec：`specs/changes/2026-10-31-round8-async-hardening`（proposal/spec/design/tasks/acceptance；proposal 已链接本记录）。
- 评审日期：2026-10-31
- 结论：**APPROVED**（最终 HEAD **a09a1e8**：P0=0 / P1=0；首评 P1-1 已由 `6f7701f` 修复并经复评确认；P2=6 中 P2-1/2/3 已闭环，剩余 P2-4/5/6 与新增 docs 口径项为非阻塞残留）。首评结论 CHANGES-REQUESTED（gateway 客户端 request-id 被当计费键，重复 id 丢请求/只扣一次费）已解决：结算键改为服务端每次准入 UUID，客户端 `X-Request-Id` 仅关联/审计，`scripts/srv-auth-test.sh` 真实 PG **EXIT 0**。详见 §8。

## 1. 评审范围

- 异步子代理：`packages/client/agent-rig/src/lib.rs`（live RAII 计数、catch_unwind、CAS 状态机、children/kit 64 保留、stress/panic/race/retention tests）、`crates/zk-kit-subagent/src/lib.rs`（bounded registry、MAX_AGENT_RECORDS）、工具（`tool-subagent`/`tool-subagent-control`）。
- request-id：`apps/server/src/main.rs` 最外层 `request_context`；`packages/server/http/src/lib.rs` gate/tests；`crates/zk-kit-server/src/lib.rs` `request_id_from_headers`/`request_id_for_request`/`current_request_id`/`with_request_id`；`settings` bootstrap、`updates` manifest、`gateway` settlement、`llm-gateway` client header、`apps/desktop/src/native_client.rs` env 安装。
- gateway `/v1` 例外：`packages/server/http` `is_client_business_path`/tests、`docs/04` §2.7、spec/acceptance、commit 01590c9。
- matrix/writer：`scripts/agent-tool-matrix.py --check`、`scripts/check.sh`、`.github/workflows/ci.yml`、`docs/24` §3、`docs/generated/README.md`、`docs/38`。
- 文档真源：`docs/04` §7/§2.7、`docs/21` §7/§7.1、`docs/24`、`docs/38`、`docs/generated/*`、round-8 tasks/acceptance。

## 2. 门禁复跑（独立执行；均在本机真实运行）

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | **EXIT 0** |
| `cargo clippy --workspace --all-targets -- -D warnings` | **EXIT 0**（`target/evidence/audit/round8/clippy.log`） |
| `cargo test --workspace` | **EXIT 0**（418 blocks，**412 passed / 0 failed**；含 `async_live_thread_cap_and_slot_release`、`async_panic_child_marks_failed_and_releases_slot`、`async_interrupt_wins_and_completion_never_overwrites`、`async_interrupt_completion_race_iterations_stay_legal`、`async_retention_is_bounded`、`request_id_is_generated_once_and_shared_with_gate_body`、`gateway_paths_are_not_version_gated_by_http_middleware`） |
| `cargo test --workspace export_schema` + `git diff --exit-code` | **EXIT 0 / 0** |
| `bash scripts/schema-check.sh` | **EXIT 0** |
| `cargo run -q -p zk -- validate` | **EXIT 0**（114 packages） |
| `cargo run -q -p zk -- compose --edition personal --profile default --channel official --check` | **EXIT 0**（16 files） |
| `cargo run -q -p zk -- capability --check` / `docs --check` / `db lock --check` / `lint --duplicates` | **0 / 0 / 0 / 0** |
| `python3 scripts/agent-tool-matrix.py --check` | **EXIT 0**（`agent tool matrix up to date`；`scripts/check.sh:10`、`.github/workflows/ci.yml:27` 已接入） |
| `pnpm -r typecheck` / `lint` / `test:unit` / `build` | **0 / 0 / 0 / 0**（`round8/pnpm-and-schema.log`） |
| `bash scripts/agent-e2e.sh` / `m1-e2e.sh` / `admin-e2e.sh` | **0 / 0 / 0**（agent 9 passed；m1 7+7；admin 11） |
| `bash scripts/login-e2e.sh` / `client-upgrade-e2e.sh` / `updates-e2e.sh` / `apps-e2e.sh` / `oem-e2e.sh` | **0 / 0 / 0 / 0 / 0**（login 3；client-upgrade 1；updates manifest/nested；apps；oem） |
| `bash scripts/srv-auth-test.sh`（真实 PG 集成测试） | 首评 01590c9：**EXIT 1**（先业务 426 缺客户端头；wrapper 后在 gateway 重复 id 断言失败，见 §3.2/P1-1）。**复评 a09a1e8：EXIT 0**（脚本已补客户端头并断言重复 id 计费两次、无客户端 id usage、≥2 settlement id；见 §8）。 |
| `python3 scripts/docs-check.py` / `spec-check.py` | **0 / 0** |
| `python3 scripts/review-check.py`（写本记录前） | **1**（缺本记录，预期；本结论为 CHANGES-REQUESTED，写入后仍应为 1 直到 P1 修复+复评） |
| 真实 PG 现场探测 | business/bootstrap/updates 426 与 audit request-id（`round8/probe.log`） |

## 3. 结论依据

### 3.1 异步子代理加固

- **live-thread 计数**：`RigSubagentRunner` 增加 `live_threads: Arc<AtomicUsize>`；`take_slot` 原子 `fetch_add` 检查 `MAX_ASYNC_SUBAGENTS`，失败回减；线程入口创建 `ChildThreadGuard`（Drop 减计数 + clear context depth），spawn 失败时 `SlotLease` 显式回减。实现不再是“status==running”计数，`interrupt` 置 cancelled 后槽位仍被存活线程占用（`agent-rig:1048-1068, 1160-1235`）。
- **panic 安全**：线程体 `catch_unwind(AssertUnwindSafe(...))`，panic → stderr 记录 + `Err("subagent thread panicked: ...")` → `ChildState::Failed`；`ChildThreadGuard` 保证槽位/深度释放；`complete_child` 在 registry 锁内做状态迁移。
- **CAS/状态机**：`ChildState { Running, Done, Failed, Cancelled }`；`interrupt` 只在 Running 时置 flag+Cancelled+取消结果；`complete_child` 在 Running 时根据 flag/outcome 迁移，遇到 Cancelled 保留取消结果，terminal 不覆盖；interrupt-after-finish 返回错误（`agent-rig:1070-1105, 1255-1285`）。
- **retention**：children map 上限 `MAX_CHILD_RECORDS`（64），只淘汰最旧 finished；kit `AGENTS` 同样 `MAX_AGENT_RECORDS=64`，running 不淘汰（`agent-rig:891-945`、kit-subagent:97-175）。
- **测试**：`async_live_thread_cap_and_slot_release`（8 持有线程、active 仍占槽、释放后恢复）、`async_panic_child_marks_failed_and_releases_slot`（panic failed/槽位复用）、`async_interrupt_wins_and_completion_never_overwrites`、`async_interrupt_completion_race_iterations_stay_legal`（32 轮）、`async_retention_is_bounded`（>64 记录与 kit 上限）均 **pass**；工作区测试 412 passed/0 failed。
- **P2 残留**：测试用 `TestHooks` 的 hold/panic 是确定性注入，真正的多线程 interrupt/completion 竞争窗口未压力覆盖；spawn 失败路径无专门测试；retention 淘汰语义未在 docs/38 说明（旧结果会从 `subagent_status` 消失）。见 §4 P2-4/P2-5。

### 3.2 单 request-id 与 gateway 计费键（P1）

- 正向：`apps/server/src/main.rs:197-198` 最外层 `request_context` 先执行；`packages/server/http/src/lib.rs:50-53` 一次读取/生成 request id 并写入 task-local，`attach_request_id` 对所有响应回显；gate `/api/client/v1/*` 使用 `request_id_for_request`；bootstrap/updates 用同一 helper；`audit_tx` 用 `current_request_id`。真实 PG 探测（`round8/probe.log`）：
  - business 426 无客户端头：生成 UUID，header 与 body request_id 相同；
  - bootstrap 426 带 `X-Request-Id: probe-boot-r8`：header/body 回显该 id；
  - updates 426 带 `probe-upd-r8`：header/body 回显；
  - app.upload 带 `probe-audit-r8`：`audit_log` 行 request_id 实为该值（count=1；非空/非 fake）。
- **P1-1（计费/结算键回归）**：gateway 现在用 `request_id_for_request(&headers)` 作为 `wallet_holds`/`usage_pending`/`usage` 的 request_id（`packages/server/gateway/src/lib.rs:106, 141, 227`；`admit` 插入 `:439-449`）。但 `usage_pending.request_id` 与 `usage.request_id` 均为 **UNIQUE**（`packages/server/gateway/migrations/0005_gateway.sql:14,19`），而客户端可提供任意 `X-Request-Id`。原 `scripts/srv-auth-test.sh:204-211` 明确断言“同一 client x-request-id 两次是两次独立准入、都计费；billing key 由服务端生成，绝不使用客户端头”。
  - 实测（reviewer wrapper 给业务请求补客户端头后跑 srv-auth，`round8/srv-auth-probe.log`）：两次 `x-request-id: probe-fixed-id` gateway 调用后钱包余额 **999900**（只扣一次 50），脚本期望 **999850**（两次）→ 断言失败；即第二次准入因 `usage_pending.request_id` UNIQUE 冲突失败/未计费，客户端可复用/重试 id 造成 500 或丢请求。
  - 该行为在 round-8 引入（commit 4b455f3）：把客户端相关 id 用作唯一结算键，既破坏重试幂等，也让计费键由客户端选择；`docs/04:196`/spec/acceptance 的“gateway settlement 复用同一 id”没有同步既有测试/边界说明。
  - 修法：保留 HTTP `request_id` 用于审计/关联，但 gateway 的 `wallet_holds`/`usage_pending`/`usage` 结算键应使用**服务端生成的 UUID**（或在明确 idempotency-key 语义下做 upsert 并更新 test/docs）；新增重复 header/重试单测；更新 `scripts/srv-auth-test.sh` 的预期与 docs/04 §7 语义。
- **P2（标准 srv-auth-test 红）**：`scripts/srv-auth-test.sh` 本身在 HEAD 先因业务请求未带 `X-ZeoKeel-Client` 收到 426（round-6 gate 起既有测试债），且不是 `scripts/check.sh`/CI 的组成；acceptance 的“gates 全 0”未覆盖它，属 false-green/测试债。应修复脚本客户端头并纳入某个 gate（或显式标记非门禁）。
- `gateway` non-stream 路径/streaming task 用捕获的 request_id 写 usage/wallet；无 audit 行（gateway settle 无 audit_tx），故“gateway settlement/audit 复用”在实现上仅指 usage/wallet 关联；建议文档明确。

### 3.3 `/v1` 426 例外诚实性（判断：诚实）

- spec §2 明确“本轮 `/v1/*` 426 门禁不启用：rig-core 原生 Responses 客户端不能附加自定义头；网关依赖 JWT+发布锁定；docs/04 记录例外，列入后续”。
- 最终代码 `is_client_business_path` 只匹配 `/api/client/v1/*`（`packages/server/http/src/lib.rs:35-44`），`/v1/*` 直通；测试 `gateway_paths_are_not_version_gated_by_http_middleware` 明确断言“authenticated-but-not-version-gated”，没有把它写成 426 覆盖；`docs/04:258` 写全例外原因与未来条件；`llm-gateway` 仍通过 `ZEOKEEL_CLIENT_HEADER` 发送标识（不改变“未门禁”事实）。
- 结论：**没有把 `/v1` 伪装成 426 覆盖**，例外文档明确；不构成 P1/P2 文档虚假。历史 commit 4b455f3 曾尝试启用，01590c9 回退并记录，最终态一致。

### 3.4 matrix writer SSOT + CI/check gate

- `docs/24:251` 已把 `docs/generated/AGENT-TOOL-MATRIX.md` 单列为 `scripts/agent-tool-matrix.py` writer（研究/对照用，不是产品 SSOT），`docs/generated/README.md:12` 同步；`scripts/agent-tool-matrix.py` 实现 `--check`（文件不一致 exit 1）；`scripts/check.sh:10` 与 `.github/workflows/ci.yml:27` 已调用；本轮 `--check` **0**。
- P2：`docs/38:34` 仍写“完整矩阵与测试状态以 AGENT-TOOL-MATRIX.md 为准”，与 docs/24/README 的“研究/对照用，不是产品 SSOT”措辞冲突；应按 one-SSOT 统一（工具覆盖矩阵可为准，但明确“非产品规范”）。见 P2-2。

### 3.5 audit hash / request-id 文档

- `docs/21` 新增 §7.1：hash 链覆盖 `prev_hash+action+target+detail`；`request_id` 为运营/排障字段，不参与 hash；若纳入 actor/request_id 需链版本化。实现 `crates/zk-kit-server/src/lib.rs:762-764` 与文档一致（hash 不含 request_id，INSERT 写入 request_id）。**诚实、无虚假覆盖**。

### 3.6 false-green 评估

- acceptance 声称的 fmt/clippy/test、zk/compose/schema/docs/db lock/lint/matrix、pnpm、agent/m1/login/admin/client-upgrade e2e 均独立复跑成立；updates/apps/oem 额外复跑 0。
- **未覆盖/红**：标准 `scripts/srv-auth-test.sh` 在 HEAD 红（先 426、后有 P1-1 的 gateway 重复 id 失败）；acceptance 与 check.sh/CI 未包含该脚本，属测试债/false-green 风险（P2-1）。task.md 状态仍全 `pending`、acceptance 仍 `in-progress`（正常待评审，但 tasks 未随实现更新，P2-3）。
- async 压力/panic/race/retention 单测通过，但无 async e2e/真实 turn 集成（round-7 P2-9 延续，P2-5）。

## 4. P0 / P1 / P2

**P0：无。**
**P1：1（P1-1，首评阻塞；已于 a09a1e8 修复，见 §8）。**

| # | 位置 | 问题 | 修法 |
| --- | --- | --- | --- |
| P1-1 | `packages/server/gateway/src/lib.rs:106`（`request_id_for_request`）、`:141/:227`（settlement id）、`:439-449`（admit 插入 `usage_pending`）；`packages/server/gateway/migrations/0005_gateway.sql:14,19`（request_id UNIQUE）；`scripts/srv-auth-test.sh:204-211`；spec/acceptance:20 | **客户端可控制的 `X-Request-Id` 被当作计费/结算唯一键**：重复/复用 id 的第二次 gateway 准入因 UNIQUE 冲突失败（实测两次同 id 仅扣一次费：余额 999900，期望 999850），违背既有“billing key 服务端生成、忽略客户端头”不变量；round-8 未同步 srv-auth-test/文档边界。 | HTTP request_id 仅用于关联/审计；wallet_holds/usage_pending/usage 用服务端生成的结算 UUID（或显式 idempotency-key 语义 + upsert），并更新 srv-auth-test、docs/04 §7/acceptance；补重复 header/重试测试。 |

**P2（非阻塞加固/证据/文档）**

| # | 位置 | 问题 | 修法 |
| --- | --- | --- | --- |
| P2-1 | `scripts/srv-auth-test.sh`（无 `X-ZeoKeel-Client`；未入 `scripts/check.sh`/CI）；`target/evidence/audit/round8/srv-auth-standard.log` | 真实 PG 集成脚本在 HEAD 先 426（round-6 gate 起既有测试债），且非门禁；acceptance “gates 全 0”未覆盖它。 | 给业务 curl 补客户端头并纳入 check.sh/CI（或明确标记非门禁）；修正 line 211 的语义（与 P1-1 一起）。 |
| P2-2 | `docs/38:34` vs `docs/24:251`/`docs/generated/README:12` | matrix SSOT 措辞冲突（docs/38 称“以 matrix 为准”，docs/24/README 称研究/对照用、不是产品 SSOT）。 | 统一为“工具覆盖矩阵以 matrix 为准，但不是产品规范 SSOT”，或调整 docs/38 指回 docs/24。 |
| P2-3 | `specs/changes/2026-10-31-round8-async-hardening/tasks.md`；acceptance | tasks 全部仍 `pending`（实现已完成/已验证），验收仍 in-progress；属文档状态真源滞后。 | C19 通过后把 tasks 标 done、acceptance 填证据并完成。 |
| P2-4 | `packages/client/agent-rig/src/lib.rs:891-945`（MAX_CHILD_RECORDS/prune）；kit-subagent:97-175 | retention（64，只淘汰 finished）未在 docs/38 说明；被淘汰的 completed 子代理无法再 `subagent_status` 读取，用户/上层可能误解为丢失。 | docs/38 明确保留上限/淘汰语义与 `NotFound` 行为；必要时在 tool 结果提示 retention。 |
| P2-5 | `packages/client/agent-rig/src/lib.rs` 测试（`TestHooks` hold/panic；race 32 轮顺序执行）；`scripts/agent-e2e.sh`（同步流程） | 虽然有 race/live/panic/retention 单测，但没有真实并发 interrupt/completion 压力、spawn 失败测试、async e2e/真实 turn 集成（round-7 P2-9 仍开）。 | 增加真实并发压测（多线程 start/interrupt/complete 无 hold）与 spawn-failure hook；补 mock-model e2e 覆盖 start/status/result/interrupt/cap；保留 UNVERIFIED 注明真实模型。 |
| P2-6 | `crates/zk-kit-server/src/lib.rs:496-542`（`request_id_from_headers` 仍接受任意 ≤128 客户端值） | 审计/gate 的 request_id 可被客户端任意设置（与 P1-1 分开的通用关联风险；不影响授权）。 | 限制字符集（如 `[A-Za-z0-9._:-]`）或对客户端值加前缀/记录来源；文档说明 request_id 非授权/非幂等键。 |

## 5. UNVERIFIED（环境限制，不计入本轮缺陷）

- Tauri 真机/Windows 运行时（gateway native header、async 取消、open_url 实际行为）。
- 真实模型长时取消/真实 SSE 断线结算压力；Docker/多平台/安装包/notarize；cargo-deny/audit 本地；真实私有 OEM 索引。
- `srv-auth-test` 首评时 gateway 重复 id 实测使用临时 wrapper（未改仓库脚本）绕过前置 426；**复评 a09a1e8 已修复脚本客户端头债务并将该断言纳入标准脚本，真实 PG EXIT 0**（§8）。

## 6. 证据路径

- 门禁：`target/evidence/audit/round8/{fmt,clippy,workspace-tests,pnpm-and-schema}.log`
- e2e：`target/evidence/audit/round8/{agent,m1,admin,login,client-upgrade,updates,apps,oem}-e2e.log`
- request-id/审计现场：`target/evidence/audit/round8/probe.log`（business/bootstrap/updates 426、app.upload→audit_log.request_id）
- gateway 计费键回归：`target/evidence/audit/round8/srv-auth-standard.log`（标准脚本 426）与 `srv-auth-probe.log`（wrapper 后 `BAL2B=999900` vs 期望 999850；line 204-211 断言失败）
- matrix：`python3 scripts/agent-tool-matrix.py --check` → 0；CI/check.sh 行号见 §2。

## 7. 结论与状态

- 最新结论：**CHANGES-REQUESTED**（P0=0；P1=1；P2=6）。异步加固、单 request-id 中间件、matrix gate、`/v1` 例外文档均通过；P1-1（客户端 request-id 作为 gateway 计费唯一键，重复 id 丢请求/只扣一次费）阻塞 APPROVED。
- 写本记录前 `review-check.py` 报 1（缺本记录）；写入后仍为 **1**（CHANGES-REQUESTED），P1 修复并复评 APPROVED 后应为 0。
- 本记录是唯一新增文件；未修改产品代码/生成物/committed 文件。

## 8. 复评（HEAD a09a1e8）

- 复评对象：`HEAD a09a1e8`（`6f7701f` gateway server-generated settlement ids；`a09a1e8` matrix SSOT docs；base 01590c9）。工作树 clean（唯一未跟踪文件 = 本记录）。
- 复评结论：**APPROVED（P0=0 / P1=0）**。首评 P1-1 已修复并独立复现通过；P2-1/2/3 已闭环，P2-4/5/6 与新增 docs 口径项为非阻塞残留。

### 8.1 P1-1 复核（已修复）

| 项 | 证据 |
| --- | --- |
| 结算键 | `packages/server/gateway/src/lib.rs:103-108` 改为 `let request_id = Uuid::new_v4().to_string();`，`admit`/`settle` 使用服务端生成 UUID；客户端 `X-Request-Id` 不再进入 `wallet_holds`/`usage_pending`/`usage` 唯一键。 |
| 文档 | `docs/04:196` 明确“`X-Request-Id` 仅关联/审计，不是业务幂等键或计费键；gateway 结算 id 由服务端每次准入生成 UUID”；acceptance §1.2 记录修复；docs/38 matrix 措辞同步为“对照参考，不是产品 SSOT”。 |
| 真实 PG 回归 | `scripts/srv-auth-test.sh` 已给业务/gateway curl 补 `X-ZeoKeel-Client`（round-6 债），并新增：同一 `x-request-id: probe-fixed-id` 两次 → 余额 **999850**（两次计费）、`usage_pending/usage` 无该客户端 id 行、`usage` 至少 2 个不同 settlement id；reviewer 独立运行 → **EXIT 0**（`target/evidence/audit/round8-rereview/srv-auth.log`，含 `gateway-billing/x-request-id-ignored`）。 |
| 其他 | `tool-subagent-control` 的 `list_agents_includes_async_children` 不再替换全局 runner，race 修复；`cargo test --workspace` **412 passed / 0 failed** 通过。 |

### 8.2 复评门禁（a09a1e8，独立执行）

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | **EXIT 0** |
| `cargo clippy --workspace --all-targets -- -D warnings` | **EXIT 0** |
| `cargo test --workspace` | **EXIT 0**（418 blocks / **412 passed** / 0 failed；含 gateway/matrix/tool-subagent 修复相关测试） |
| `cargo test --workspace export_schema` + `git diff --exit-code` | **EXIT 0 / 0** |
| `bash scripts/schema-check.sh` | **EXIT 0** |
| `zk validate` / `compose personal --check` / `capability --check` / `docs --check` / `db lock --check` / `lint --duplicates` | **0 / 0 / 0 / 0 / 0 / 0** |
| `python3 scripts/agent-tool-matrix.py --check` | **EXIT 0** |
| `pnpm -r typecheck` / `lint` / `test:unit` / `build` | **0 / 0 / 0 / 0** |
| `bash scripts/srv-auth-test.sh`（真实 PG） | **EXIT 0**（见 8.1） |
| `bash scripts/client-upgrade-e2e.sh` / `updates-e2e.sh` / `agent-e2e.sh` | **0 / 0 / 0**（1 passed / updates ok / 9 passed；复评重跑） |
| m1/admin/login/apps/oem e2e | 首评 01590c9 均 0；a09a1e8 仅 gateway/docs/test 变更，未重跑（不影响这些面） |
| `python3 scripts/docs-check.py` / `spec-check.py` | **0 / 0** |
| `python3 scripts/review-check.py` | 写本记录前 1；写入后应为 **0**（latest APPROVED） |

### 8.3 剩余 P2（非阻塞，Lead 已确认保留）

- **P2-1（部分）**：`scripts/srv-auth-test.sh` 已修复并通过，但仍未加入 `scripts/check.sh`/CI（CI 覆盖缺口）。
- **P2-2/P2-3**：docs/38 matrix 措辞与 tasks 状态已修复/闭环。
- **P2-4**：async retention 64/淘汰语义仍应在 docs/38 明确（被淘汰 completed 子代理 `subagent_status` 会 NotFound）。
- **P2-5**：async 测试仍以 TestHooks 确定性注入为主，缺真实并发 interrupt/completion 压力、spawn-failure 测试与 async e2e/真实 turn 集成。
- **P2-6**：`request_id_from_headers` 仍接受任意 ≤128 客户端值（关联/审计可被客户端设置；非授权/非计费键）。
- **P2-7（复评新增，文档口径）**：`docs/04:196` 前半段已区分“客户端 X-Request-Id 关联/审计、gateway 结算 id 服务端 UUID”，但同段末尾仍写“`request_id` 全链路透传”；建议明确“HTTP request_id 全链路关联；gateway settlement id 独立生成”，避免读者误认为同一 id 贯穿计费。

### 8.4 复评证据

- `target/evidence/audit/round8-rereview/{fmt,clippy,workspace-tests,export-schema,pnpm}.log`
- `target/evidence/audit/round8-rereview/srv-auth.log`（真实 PG EXIT 0，含重复 request-id 计费断言）
- `target/evidence/audit/round8-rereview/{client-upgrade-e2e,updates-e2e,agent-e2e}.log`

- 终评结论：**APPROVED（HEAD a09a1e8；P0=0 / P1=0；P2 残留已记录）**。
