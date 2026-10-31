# Round-10 独立评审（文档/gate 真源 / request-id charset / async stress / schedule e2e）

- 评审对象：`HEAD 9e950a9`（首评；round-10：`docs+test(round10): honest zk api/lock status, request-id charset, async stress, schedule e2e`）。**复评对象：`HEAD 459d363`（`docs: mark zk api/lock/verify-lock/metadata as backlog everywhere`；父 9e950a9；详见 §8）**。复评开始时工作树 clean（唯一未跟踪文件即本记录）。
- 评审者：**auditor-build**（独立评审；round-10 未实现任何被评审代码；仅 task-65）。
- 独立性：round-10 由 Lead 与 auditor-rig / auditor-web / auditor-rust 实现（proposal/commits）；评审者未参与本提交。
- 关联 spec：`specs/changes/2026-10-31-round10-truth-and-p2`（proposal/spec/design/tasks/acceptance；proposal 已链接本记录）。
- 评审日期：2026-10-31
- 结论：**APPROVED**（最终 HEAD **459d363**：P0=0 / P1=0；首评 P1-1 文档 false claims 已修复并经复评确认；P2=5 中 P2-1 已闭环，P2-2/3/4/5 为非阻塞残留）。所有 `zk api`/`zk lock`/`zk verify-lock`/`zk metadata snapshot` 引用已统一标为未实现/backlog；docs/04 已记录 `X-Request-Id` charset `[A-Za-z0-9._-]{1,128}`；docs-check/spec-check 0，compose 0。详见 §8。

## 1. 评审范围

- 文档/gate 真源：`AGENTS.md` §6、`docs/03-composition-and-ci.md` §4/§5.5/§6.1、`docs/24-composition-and-runtime-spec.md` §3/§4/§8、`docs/04`（路由/生成物）、`docs/06`/`docs/10`/`docs/13`/`docs/18`、`docs/generated/README.md`、`.github/workflows/ci.yml`、`scripts/check.sh`。
- request-id：`crates/zk-kit-server/src/lib.rs` `request_id_from_headers`/`valid_request_id`、`packages/server/http/src/lib.rs` 测试、bootstrap/updates/gateway/audit 复用；`docs/04` §7。
- async：`packages/client/agent-rig/src/lib.rs`（spawn hook、real-thread stress、retention 测试）、`crates/zk-kit-subagent/src/lib.rs` `forget`、`docs/38` retention。
- schedule e2e：`e2e/m1-serve.spec.ts` schedule 设置保存/重载/settings.get；`apps/desktop` settings/automation（前轮已改）。
- false-green：acceptance/tasks 与本地 gates/e2e。

## 2. 门禁复跑（独立执行；均在本机真实运行）

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | **EXIT 0** |
| `cargo clippy --workspace --all-targets -- -D warnings` | **EXIT 0**（`target/evidence/audit/round10/clippy.log`） |
| `cargo test --workspace` | **EXIT 0**（418 blocks，**423 passed / 0 failed**；含 `request_id_rejects_invalid_charset_and_overlong_values`、`invalid_and_overlong_request_ids_are_replaced_consistently`、`forget_removes_one_child_record`、`async_spawn_failure_leaks_no_slot_or_record`、`async_real_thread_stress_respects_live_cap_and_legal_states`、`retention_evicts_oldest_completed_but_keeps_running`） |
| `cargo test --workspace export_schema` + `git diff --exit-code` | **EXIT 0 / 0** |
| `bash scripts/schema-check.sh` | **EXIT 0** |
| `zk validate` / `compose personal --check` / `capability --check` / `docs --check` / `db lock --check` / `lint --duplicates` | **0 / 0 / 0 / 0 / 0 / 0** |
| `python3 scripts/agent-tool-matrix.py --check` | **EXIT 0** |
| `pnpm -r typecheck` / `lint` / `test:unit` / `build` | **0 / 0 / 0 / 0** |
| `bash scripts/m1-e2e.sh`（mock 7 + real serve/SQLite 7，含 schedule 持久化 spec） | **EXIT 0**（`settings persist the schedule config across reload (serve + SQLite)` pass） |
| `bash scripts/agent-e2e.sh` / `login-e2e.sh` / `admin-e2e.sh` | **0 / 0 / 0**（9 / 3 / 11 passed） |
| `bash scripts/srv-auth-test.sh`（真实 PG） | **EXIT 0** |
| `python3 scripts/docs-check.py` / `spec-check.py` | **0 / 0** |
| `python3 scripts/review-check.py`（写本记录前） | **1**（缺本记录；本结论 CHANGES-REQUESTED，写入后仍应为 1 直到 P1 修复+复评） |
| 实际 zk 命令探测 | `zk api`、`zk lock`、`zk verify-lock`、`zk metadata` 均 `unknown command`（见 §3.1） |

## 3. 结论依据

### 3.1 文档/AGENTS/CI 真源（P1）

- 已修：`AGENTS.md:200` 把 `zk api --check` 注释为“未实现（backlog）”，从可执行命令清单移除；`docs/03 §6.1` 增加实现状态说明并从 drift 示例删 `zk api`；`docs/24 §3` 把 `api-catalog`/`errors`/`metadata.snapshot` 行和组合锁行标为 backlog，`§4` 静态路由真源标 backlog；`docs/generated/README` 已有未实现说明；`.github/workflows/ci.yml` 与 `scripts/check.sh` 实际没有 `zk api`/`zk verify-lock`/`zk lock`/`zk metadata snapshot`。
- **P1-1（false claims 仍存在）**：多处权威/规划文档仍把上述不存在命令当作当前命令或 CI gate：
  - `docs/03:182` `zk verify-lock` 命令表（CI 用途“变体构建”）、`:184` `zk api`/`--check`、`:201` `zk lock`、`:216` 生成物分工、`:300` 路由静态真源、`:326` writer 分工；
  - **`docs/03:352`** §5.5 仍写“其他组合的构建 … CI … → `zk verify-lock ...`”；这是明确的 false CI gate（§6.1 的 note 在更后面且只说示例为计划形态，不能覆盖此表）；
  - `docs/24:318` §8 仍写“离线构建使用 `metadata.snapshot.json`（入库）并校验工作区哈希”，与 §3 表“metadata snapshot 未实现”直接矛盾；
  - `docs/04:171` 仍称 manifest `[[routes]]` 静态真源 + `zk api` 生成 catalog；`:248` 仍把 `http-routes.json`/`permissions.json` 归 `zk api` 生成并 `--check`；
  - `docs/06:59`、`docs/10:92/117`、`docs/13:147/426`、`docs/18:191` 仍把 `zk api`/`zk verify-lock` 当作当前命令/CI 步骤。
- 实测 `./target/debug/zk api --check` / `lock` / `verify-lock` / `metadata snapshot` 均 `zk: unknown command: ...`；因此上述文档是 false claim。修法：按 `docs/24 §3` 的统一 backlog 标注/删除所有 current 表述，或把这些章节明确标为“计划形态（未实现）”；`docs/03:352` 与 `docs/24:318` 必须改。
- **P2 相关**：`scripts/docs-check.py` 只做结构/链接检查，无法发现这类语义 false claim；本轮 docs-check 0 不能证明“文档真源已诚实”。

### 3.2 request-id charset 行为

- `crates/zk-kit-server/src/lib.rs` `valid_request_id`：非空、字节长度 ≤128、字符仅 `[A-Za-z0-9._-]`；`request_id_from_headers` 非法/超长生成 UUID；HTTP gate 的 `request_context` 一次生成/回显，gate/bootstrap/updates/audit 复用 task-local；gateway settlement id 独立（round-8 语义保留）。
- 单测：kit `request_id_rejects_invalid_charset_and_overlong_values`（合法 `A.z_9-valid` 保留；`bad id`/`bad/id`/`bad:id`/`bad\tid`/`é`/129 字符 → UUID；128 字符保留）；HTTP `invalid_and_overlong_request_ids_are_replaced_consistently`（body/header 同为生成 UUID、不回显非法值）与 `valid_request_id_charset_is_kept_verbatim` pass；workspace 423 passed。
- **P2-1**：`docs/04` §7 只澄清“HTTP request_id 全链路关联、gateway settlement id 独立”，**未记录允许字符集 `[A-Za-z0-9._-]{1,128}`**；客户端需知道哪些 id 会被替换。另 invalid-header 集成测试只覆盖 gate，bootstrap/updates/gateway/audit 的 invalid 输入依赖共享 helper 单测（可补端到端/多 surface 断言）。
- 未发现代码级 P1：invalid 值不会进入响应/审计（gate 测试）；gateway 计费键仍服务端 UUID。

### 3.3 async spawn-failure / real-thread stress / retention

- `RigSubagentRunner` 增加 `spawn_child`（test hook `fail_spawn` 注入确定性 spawn 失败）；失败路径释放 `SlotLease`、`live_threads.fetch_sub`、`clear_context_depth`、`forget(id)`（kit registry）并移除 children record；`async_spawn_failure_leaks_no_slot_or_record` 断言 live=0/children 空/kit 无记录/可重试。
- `async_real_thread_stress_respects_live_cap_and_legal_states`：40 次真实 start（奇数 interrupt），每次断言 live≤8，等待 live=0，所有 retained id 状态为 done/failed/cancelled；`async_live_thread_cap_and_slot_release`/`async_panic_child...`/`async_interrupt_wins...` 保持。
- `forget_removes_one_child_record` 与 `retention_evicts_oldest_completed_but_keeps_running` pass；`docs/38` 明确 children/kit 上限 64、淘汰最旧已完成、running 不淘汰、evicted→NotFound；`docs/38` 修正存活线程/cancel/panic 语义。
- **P2-2**：stress 仍由单线程顺序 start/interrupt 驱动（真实 OS 线程，但不是多调用方并发）；仍无 async e2e/真实 turn 集成（round-7/8/9 延续）。测试覆盖可接受，但不应宣称“完全并发压力”。

### 3.4 schedule e2e 真实性

- `e2e/m1-serve.spec.ts` 通过真实 serve carrier：打开设置 → “调度/自动化” → 勾选 enabled、scan=30 → 保存 → reload 新 boot → 重新打开设置断言值来自磁盘；再用 `POST /ipc settings.get` 断言 `agent.schedule={enabled:true,scan_seconds:30}`；测试通过（m1 serve 7 passed）。
- **没有 fake auto-run**：本 spec 只验证 settings 持久化/reload/settings.get，没有伪造 executor 运行；spec §3 允许“若自动 turn 不可稳定构造则记录 UNVERIFIED”。真实 executor 自动 turn e2e 仍 UNVERIFIED。
- **P2-3**：spec 保存后不恢复 settings（serve 临时 HOME 内 enabled/30 保留给后续测试）；且未验证 executor 实际按 30s 运行 turn（无 auto-run e2e）。

### 3.5 false-green 评估

- acceptance 声称的 fmt/clippy/test、zk/schema/docs/matrix、pnpm、m1/agent/login/admin e2e 均独立复跑成立（423 tests；m1 schedule spec pass；agent/login/admin 0）；srv-auth 0。
- **false-green**：`docs-check 0` 与 `docs/24 §3`/`docs/03 §6.1` 局部修复掩盖了 §3.1 列表中的其它 false claims；acceptance “docs/AGENTS/CI 真源”尚未完全成立。
- **P2-4**：round-10 `tasks.md` 仍全 `pending`、acceptance 仍 `in-progress`；CI integration job 未在 GitHub runner 验证（仅 YAML/本地 srv-auth 0）。
- **P2-5**：`m1-serve` schedule spec 的 settings 修改未 cleanup；长期看应恢复临时设置，避免测试间状态泄漏。

## 4. P0 / P1 / P2

**P0：无。**
**P1：1（文档仍把不存在的 zk 命令/CI gate 当作当前状态；首评阻塞，已于 459d363 修复，见 §8）。**

| # | 位置 | 问题 | 修法 |
| --- | --- | --- | --- |
| P1-1 | `docs/03:182,184,201,216,300,326,352`；`docs/24:318`；`docs/04:171,248`；`docs/06:59`；`docs/10:92,117`；`docs/13:147,426`；`docs/18:191` | 文档仍把 `zk api`/`zk verify-lock`/`zk lock`/`zk metadata snapshot` 当作当前命令/CI gate/生成 writer；实测 `zk` 对这些命令 `unknown command`。其中 `docs/03:352` 明确写 CI 执行 `zk verify-lock`，`docs/24:318` 写离线构建使用 `metadata.snapshot.json`，与 §3 backlog 表直接矛盾。 | 按 `docs/24 §3` 的统一 backlog 标注/删除所有 current 表述；`docs/03` CLI 表/§5.5/§5.3、`docs/24 §8`、`docs/04` 生成物/路由表、docs/06/10/13/18 至少加“未实现（backlog）”或移到计划章节；`docs/03 §6.1` note 只覆盖示例，需把表/CI 行同步。 |

**P2（非阻塞加固/证据/文档）**

| # | 位置 | 问题 | 修法 |
| --- | --- | --- | --- |
| P2-1 | `crates/zk-kit-server/src/lib.rs` valid charset；`docs/04` §7；`packages/server/http` tests | 实现允许 `[A-Za-z0-9._-]{1,128}`、非法→UUID，但 docs/04 未记录 charset；invalid-header 集成测试只覆盖 gate，bootstrap/updates/gateway/audit 未逐一验证（共享 helper 单测覆盖）。 | docs/04 写明 charset/替换规则；补多 surface invalid header 端到端断言。 |
| P2-2 | `packages/client/agent-rig/src/lib.rs` stress 测试；`e2e/` 无 async | stress 由单线程顺序 start/interrupt 驱动（真实线程但非并发调用方）；仍无 async e2e/真实 turn 集成。 | 增加多线程并发 start/interrupt 压测与 mock-model async e2e；或明确标注 synthetic stress。 |
| P2-3 | `e2e/m1-serve.spec.ts:77+` | schedule e2e 只验证 settings 持久化/reload/settings.get；未验证 executor 实际 auto-run；测试修改 settings 后无 cleanup。 | 补可稳定的 executor auto-run e2e（或 accept 为 UNVERIFIED）；测试结束恢复设置。 |
| P2-4 | `specs/changes/2026-10-31-round10-truth-and-p2/tasks.md`；acceptance；`.github/workflows/ci.yml` | tasks 仍 pending、acceptance in-progress；CI integration job 未在真实 hosted runner 验证（本地 srv-auth 0，systemd/apt PG 未验证），不应声称 CI 已绿。 | C19 通过后更新 tasks/acceptance；在真实 CI 跑一次 integration job 并记录。 |
| P2-5 | `scripts/docs-check.py`；docs truth | docs-check 只做结构/链接检查，无法发现“命令不存在/CI gate 虚假”的语义 claim；本轮 0 不等于文档真源完整。 | 在 docs-check 增加可选的命令清单/backlog 标记一致性检查，或 docs 评审清单要求逐条核对。 |

## 5. UNVERIFIED（环境限制，不计入本轮缺陷）

- `zk api`/组合锁/metadata snapshot 实现（本轮显式 backlog，未实现）。
- GitHub integration job 实际运行；Tauri 真机/Windows、真实模型调度压测、Docker/多平台、cargo-deny、私有 OEM。
- 真实 async e2e/真实 executor auto-run（仅单测 + settings 持久化 e2e）。

## 6. 证据路径

- 门禁：`target/evidence/audit/round10/{fmt,clippy,workspace-tests,pnpm-and-schema}.log`
- e2e：`target/evidence/audit/round10/{m1,agent,login,admin}-e2e.log`（m1 含 schedule 持久化 spec）
- srv-auth：`target/evidence/audit/round10/srv-auth.log`（真实 PG EXIT 0）
- 命令探测：`./target/debug/zk api --check|lock|verify-lock|metadata snapshot` → `unknown command`（§3.1）

## 7. 结论与状态

- 最新结论：**CHANGES-REQUESTED**（P0=0；P1=1；P2=5）。request-id charset/async 加固/schedule 持久化 e2e 通过；P1-1（文档仍把未实现 zk 命令/CI gate 当当前状态，含 docs/03:352、docs/24:318）阻塞 APPROVED。
- 写本记录前 `review-check.py` 报 1（缺本记录）；写入后仍 **1**（CHANGES-REQUESTED），P1 修复并复评 APPROVED 后应为 0。
- 本记录是唯一新增文件；未修改产品代码/生成物/committed 文件。

## 8. 复评（HEAD 459d363）

- 复评对象：`HEAD 459d363`（`docs: mark zk api/lock/verify-lock/metadata as backlog everywhere`；父 9e950a9）。工作树 clean（唯一未跟踪文件 = 本记录）。
- 复评结论：**APPROVED（P0=0 / P1=0）**。首评 P1-1 已修复并经独立文档复核；P2-1（charset 文档）已闭环；P2-2/3/4/5 作为非阻塞残留保留。

### 8.1 P1-1 复核（已修复）

| 项 | 证据 |
| --- | --- |
| docs/03 | `:182/:184/:201` CLI 表项均标“未实现，backlog”；`:216` 生成物分工标计划；`:300` 路由真源标计划/未实现；`:326` writer 标未实现；`:352` CI 表改为“组合锁校验（zk verify-lock，未实现/backlog）”；`:373` note 扩展为“适用于全文”。 |
| docs/24 | `:239/:242/:243/:245` 生成物/锁表 backlog；`:266` 路由真源 backlog；`:318` metadata snapshot 加“实现状态：未实现（backlog），当前 CI 不做该快照校验”。 |
| docs/04 | `:171` 路由唯一真源标“计划/未实现”；`:248` http-routes/permissions 标“未实现，backlog”。 |
| docs/06/10/13/18 | 均加“计划/未实现/backlog”限定（`docs/06:59`、`docs/10:92/117`、`docs/13:147/426`、`docs/18:189`）。 |
| AGENTS/generated README/CI | AGENTS:200 注释 backlog；generated README 未实现说明保留；`.github/workflows/ci.yml`/`scripts/check.sh` 无这些命令，gate 不再虚报。 |
| 命令实测 | `./target/debug/zk api --check` / `lock` / `verify-lock` / `metadata snapshot` 仍 `unknown command`；文档已诚实标注 backlog。 |
| request-id charset | docs/04:196 增加“客户端值仅接受 `[A-Za-z0-9._-]{1,128}`，非法/超长服务端生成 UUID”；实现/测试此前已验证。 |

### 8.2 复评门禁（459d363，独立执行）

- 本轮 diff **仅 7 个 docs 文件**（无代码/生成物/schema/CI 改动）。
- `python3 scripts/docs-check.py` → **0**（40 files）；`python3 scripts/spec-check.py` → **0**。
- `zk compose --edition personal --profile default --channel official --check` → **0**（16 files）。
- `python3 scripts/review-check.py` → 写本记录前 1；写入后应为 **0**（latest APPROVED）。
- 首评 9e950a9 的代码/gates 结果继续有效（docs-only diff）：workspace **423 passed/0 failed**、fmt/clippy 0、export_schema+diff 0、schema-check 0、zk/matrix/pnpm gates 0、m1 7+7（含 schedule 持久化 spec）、agent 9、login 3、admin 11、srv-auth 0。

### 8.3 剩余 P2（非阻塞，Lead 已确认保留）

- **P2-2**：async stress 由单线程顺序 start/interrupt 驱动（真实线程但非并发调用方）；仍无 async e2e/真实 turn。
- **P2-3**：schedule e2e 只验证 settings 持久化/reload/settings.get；未验证 executor 实际 auto-run；测试修改 settings 后无 cleanup。
- **P2-4**：round10 tasks 仍 pending、acceptance in-progress；GitHub integration job 未在真实 hosted runner 验证（本地 srv-auth 0）。
- **P2-5**：docs-check 只做结构/链接检查，不能发现语义 false claim；本轮已通过人工全文复核（P1 关闭），后续可考虑增加命令存在性/backlog 一致性检查。

### 8.4 复评证据

- `docs/03`/`docs/04`/`docs/06`/`docs/10`/`docs/13`/`docs/18`/`docs/24` diff（459d363）。
- `docs-check`/`spec-check`/`compose --check`/`review-check` 输出（本记录写入后 review-check 0）。

- 终评结论：**APPROVED（HEAD 459d363；P0=0 / P1=0；P2-2/3/4/5 已记录）**。
