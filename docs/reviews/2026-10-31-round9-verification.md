# Round-9 独立评审（调度配置化 / settings UI / automation 热生效 / CI integration）

- 评审对象：`HEAD e45b100`（首评；round-9：`187ad38` CI integration job → `e45b100` schedule settings/hot-apply）。**复评对象：`HEAD d83d965`（`fix(schedule): explicit saved defaults override env fallback; schema bounds`；父 e45b100；详见 §8）**。复评开始时工作树 clean（唯一未跟踪文件即本记录）。
- 评审者：**auditor-build**（独立评审；round-9 未实现任何被评审代码；仅 task-60）。
- 独立性：round-9 由 Lead 与 auditor-rig / auditor-web 实现（proposal/commits）；评审者未参与本提交。
- 关联 spec：`specs/changes/2026-10-31-round9-schedule-config`（proposal/spec/design/tasks/acceptance；proposal 已链接本记录）。
- 评审日期：2026-10-31
- 结论：**APPROVED**（最终 HEAD **d83d965**：P0=0 / P1=0；首评 P1-1 已修复并经复评确认；P2=6 中 P2-1/P2-2 已闭环，剩余 P2-3/4/5/6 为非阻塞残留）。显式保存/已存在 settings 文件（含默认 disabled/10）现在始终 pin override，env 仅在无 settings 文件的新装生效，用户可关闭自动调度；AgentSettings::validate/schema bounds/tool docs/docs38 同步修复。详见 §8。

## 1. 评审范围

- 契约/默认值/校验：`packages/contracts/config/src/lib.rs`（`ScheduleSettings`、`AgentSettings.schedule`、`AgentSettings::validate`、`AppSettings::validate`）、conformance/schema tests、`shared/contracts/{schema,ts}`。
- 设置 UI round-trip：`packages/ui/ui-kit/web/src/types.ts`、`packages/ui/ui-settings/web/src/{index,settings-validation}.tsx?`、`apps/desktop/web/src/protocol.ts`、i18n、vitest（`settings-validation`、`app-settings`、`validation`）。
- automation 热更新：`apps/desktop/src/automation.rs`（override/effective_config/env fallback/run_loop/start）、`apps/desktop/src/ipc.rs`（apply_schedule_settings、boot apply、settings.set）、`apps/desktop/src/main.rs`（任务启动顺序）、`packages/client/agent-rig/src/config.rs`。
- CI：`.github/workflows/ci.yml` `integration` job；`scripts/srv-auth-test.sh` 本地执行。
- 文档/矩阵：`docs/38`、`docs/generated/AGENT-TOOL-MATRIX.md`、`scripts/agent-tool-matrix.py`、`schedule`/`call-timeout` 行、`packages/client/tool-schedule` README/description、`docs/19`。
- false-green：acceptance 声称的 gates/e2e/srv-auth 与本地复跑。

## 2. 门禁复跑（独立执行；均在本机真实运行）

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | **EXIT 0** |
| `cargo clippy --workspace --all-targets -- -D warnings` | **EXIT 0**（`target/evidence/audit/round9/clippy.log`） |
| `cargo test --workspace` | **EXIT 0**（418 blocks，**416 passed / 0 failed**；含 `schedule_settings_validate_and_clamp`、`automation::tests::schedule_config_precedence_settings_env_default`、`ipc::tests::schedule_settings_apply_path_and_env_fallback`、`tool-schedule::schedule_crud`） |
| `cargo test --workspace export_schema` + `git diff --exit-code` | **EXIT 0 / 0**（新 schedule schema 确定性） |
| `bash scripts/schema-check.sh` | **EXIT 0** |
| `zk validate` / `compose personal --check` / `capability --check` / `docs --check` / `db lock --check` / `lint --duplicates` | **0 / 0 / 0 / 0 / 0 / 0** |
| `python3 scripts/agent-tool-matrix.py --check` | **EXIT 0** |
| `pnpm -r typecheck` / `lint` / `test:unit` / `build` | **0 / 0 / 0 / 0**（含 schedule UI/表单测试） |
| `bash scripts/agent-e2e.sh` / `m1-e2e.sh` / `admin-e2e.sh` / `login-e2e.sh` | **0 / 0 / 0 / 0**（agent 9 passed；m1 7+7；admin 11；login 3） |
| `bash scripts/srv-auth-test.sh`（真实 PG） | **EXIT 0**（auth/billing/audit/migrations/SIGTERM；`round9/srv-auth.log`） |
| CI YAML 解析 | `jobs` 含 `integration`；`timeout-minutes: 30`；步骤 = checkout/toolchain/cache/Local PostgreSQL/Build server/Server integration（见 §3.5；本地非 GitHub runner，UNVERIFIED） |
| `python3 scripts/docs-check.py` / `spec-check.py` | **0 / 0** |
| `python3 scripts/review-check.py`（写本记录前） | **1**（缺本记录；本结论 CHANGES-REQUESTED，写入后仍为 1 直到 P1 修复+复评） |

## 3. 结论依据

### 3.1 ScheduleSettings 默认值/校验

- 契约：`ScheduleSettings { enabled: false, scan_seconds: 10 }`；`AgentSettings.schedule: ScheduleSettings` with `#[serde(default)]`；`MIN/MAX_SCAN_SECONDS=5/3600`；`validate()` 拒绝区间外；`effective_scan_seconds()` 对 runtime 做 clamp；legacy settings 无 schedule 字段可解析为默认（conformance 测试）。
- schema/TS：`schedule-settings.json`、`app-settings.json`、`ipc-settings-set.json`、`runtime-settings.json` 重新生成且 `export_schema` deterministic；`ScheduleSettings` 在 contracts.ts 与 ui-kit 类型中存在。
- UI/desktop 保存路径：`settings.set` 的 `validate_settings` 显式调用 `settings.agent.schedule.validate()`；UI `validateSettings` 对 scan 5..3600 报错；`formToSettings` 构造 schedule；`toForm` 回填；vitest 覆盖默认/边界/非整数。
- **P2 缺口**：`AgentSettings::validate()` 本身**没有调用 `self.schedule.validate()`**（`packages/contracts/config/src/lib.rs:281-...`；grep 无 `self.schedule`），`AppSettings::validate` 因而也不校验；只有 desktop `ipc::validate_settings` 与 automation `apply_settings` 校验。生成 JSON Schema 的 `scan_seconds` 只有 `minimum: 0`、**无 max 3600 / min 5**（`shared/contracts/schema/schedule-settings.json`），因此 schema/Ajv 边界不拒绝 4/3601，靠 UI/Rust 运行期兜底。见 P2-2。

### 3.2 设置 UI round-trip

- `ui-kit` `AgentSettings.schedule` 类型与 `defaultAgentSettings` 默认；`protocol.parseAgentSettings` 对缺失 schedule 使用 serde 默认；`SettingsDialog` 增加“调度/自动化”开关 + 扫描间隔 numeric（min5/max3600）；i18n zh/en；`formToSettings/toForm/validateSettings` schedule 往返；`app-settings.test` 断言提交的 `agent.schedule = {enabled:true,scan_seconds:30}`；`validation.test` 断言 schema 接受合法/拒绝类型错误、旧 settings 解析默认。
- `settings.get` 的 masked settings 保留 schedule（仅掩码 provider key），保存 round-trip 可见。
- P2：UI 显示的是保存的 settings；当 `ZEOKEEL_ENABLE_SCHEDULE_AUTORUN=1` 且保存默认（enabled=false）时，UI 显示关闭但执行器实际启用（见 P1-1），用户无法从 UI 判断 effective 状态。

### 3.3 automation 热更新 / idle / scan 语义

- `start()` 总是启动线程（不再 env 才启动）；`run_loop` 每 100ms 读取 `effective_config()`，disabled 时 `elapsed=0` 并继续（idle，不 tick），enabled 时按 `scan_seconds` 累加 tick；启用/禁用/周期变更无需重启。`ScheduleExecutor` 由 kernel task 持有，Drop 停线程并 join；`STOP_POLL` 100ms。
- `apply_settings` 校验后再写全局 override；`settings.set` 先 apply 再 publish/save；boot `apply_persisted_settings` 在 `automation::start` 之前执行（main.rs:174 vs :212），有保存文件时应用、无文件时 clear。
- 单测：`executor_idles_when_disabled_and_hot_updates_period`（disabled 不 tick，enabled 后 tick，周期变更）；`schedule_config_precedence_settings_env_default`；`ipc::schedule_settings_apply_path_and_env_fallback`；`automation` 的 `valid_entry`/retry 逻辑保持。
- **P1-1（env fallback 优先级反例）**：`apply_settings` 用 `(*schedule != ScheduleSettings::default()).then_some(...)`（automation.rs:65-76），**默认值被当作“未显式保存”而清空 override**；`apply_persisted_settings` 在 settings 文件存在时也调用它（ipc.rs:238-246）。当 `ZEOKEEL_ENABLE_SCHEDULE_AUTORUN=1` 且用户保存/已有 `{enabled:false, scan_seconds:10}`（UI 的“关闭 + 默认扫描”）时，override 被清空，`env_fallback()` 返回 enabled=true，executor 继续自动运行；用户无法通过设置关闭。spec §1 明确“env 仅未保存设置时的 fallback；显式 settings 优先”，docs/38:33 也写“未保存设置时 env 兜底”。现有测试 `ipc.rs:1473-1510`/`automation.rs:422+` 反而把“default save → env 生效”断言为期望行为，固定了错误语义。
- 修法：区分“文件不存在/尚未保存”与“显式保存了默认值”：保存/文件存在时 override 应为 `Some(ScheduleRuntimeConfig{enabled:false,scan_seconds:10})`（或单独 explicit 标志）；仅无设置文件时 clear。补测试：env=1 + 显式 default（disabled）→ disabled；env=1 + 显式 enabled/30 → enabled/30。
- P2 相关：UI 不显示 effective env fallback；建议设置页提示 env fallback 或 meta.get 暴露 effective 状态。

### 3.4 no hidden schedule config（文档/工具面）

- 代码中 schedule 配置唯一来源是 `AppSettings.agent.schedule`；env `ZEOKEEL_ENABLE_SCHEDULE_AUTORUN` 仅 fallback（automation.rs、docs/38:33）。无其它 schedule env/常量隐藏配置（`DEFAULT_SCAN_SECONDS` 10、扫描周期 config）。
- **P2**：`packages/client/tool-schedule/src/lib.rs:77` 的 tool description 仍写“Auto-run is opt-in (ZEOKEEL_ENABLE_SCHEDULE_AUTORUN=1)”，README 仍写“执行器（定时触发）后续接；当前只管理定义”（实际已实现、并已改 settings）；这是对模型/用户可见的隐藏配置残留。docs/38 已更新，但 tool spec/README 未同步。见 P2-1。
- **P2**：`docs/19`（客户端交互/设置）未同步 schedule 设置说明；非阻塞，但 docs truth 不完整。

### 3.5 CI integration job 诚实性

- `.github/workflows/ci.yml` 新增 `integration` job（YAML 解析通过）：`runs-on ubuntu-24.04`、`timeout-minutes: 30`；步骤 `Local PostgreSQL`（apt install postgresql + systemctl start + `sudo -u postgres psql`）、`Build server`（`cargo build -p zk-server`）、`Server integration`（`bash scripts/srv-auth-test.sh`）。
- 本地同命令 `bash scripts/srv-auth-test.sh` **EXIT 0**（真实本机 PostgreSQL）；脚本使用 `sudo -u postgres` 的本地 PG，不是 Docker service；与 job 的本地安装路径一致。**没有把“本地跑过”说成“CI 已绿”**：acceptance 只列命令，未声称 GitHub 实际结果。
- UNVERIFIED：无法在本机执行 GitHub runner；`sudo systemctl start postgresql` 在 hosted runner 的 systemd/集群状态、apt 版本、30m 超时下是否稳定，需要在真实 CI 上验证；P2/UNVERIFIED，不是代码缺陷。spec 写 `services.postgres` 而实现为 apt 本地安装，属于实现选择（task 明确本地 PG 路径），不影响诚实性。

### 3.6 matrix/docs truth

- `scripts/agent-tool-matrix.py` 的 schedule 行改为 `AppSettings.agent.schedule {enabled,scan_seconds} (env fallback)`；call-timeout 行改为 `AppSettings.tools.tool_timeout_ms (ZEOKEEL_TOOL_TIMEOUT_MS fallback)`；`docs/generated/AGENT-TOOL-MATRIX.md` 同步；`--check` 0；`docs/38` schedule 段落改为 settings/hot-apply/env fallback，并保留 matrix “对照参考、不是产品 SSOT”的措辞。
- P2：tool-schedule README/description 滞后（P2-1）；tasks 状态（P2-3）。

### 3.7 false-green 评估

- acceptance 声称的 fmt/clippy/test、zk/schema/docs/matrix、pnpm、agent/m1/login/admin e2e、srv-auth 均独立复跑成立（416 tests；srv-auth 0；e2e 全 0）。
- 未验证：GitHub `integration` job 实际运行（只有 YAML 解析 + 本地同脚本）；不构成“CI 已绿”的声明。
- P2：tasks.md 仍全部 `pending`、acceptance 仍 `in-progress`；本评审通过后应更新；当前不阻塞。
- P1-1 现有单测把错误语义当期望，属于测试与 spec 的 false-green（测试绿但语义错）。

## 4. P0 / P1 / P2

**P0：无。**
**P1：1（env fallback 优先级反例；首评阻塞，已于 d83d965 修复，见 §8）。**

| # | 位置 | 问题 | 修法 |
| --- | --- | --- | --- |
| P1-1 | `apps/desktop/src/automation.rs:65-76`（`(*schedule != default).then_some` 清 override）；`apps/desktop/src/ipc.rs:238-246`（文件存在也 apply default）、`:410-415`（settings.set）；`apps/desktop/src/automation.rs:88-103`（env fallback）；docs/38:33、spec §1；测试 `ipc.rs:1473-1510`/`automation.rs:422+` | **显式保存默认 schedule 无法覆盖 env fallback**：env=1 + 用户保存 `{enabled:false,scan_seconds:10}` → `apply_settings` 清空 override → env fallback enabled → 自动调度继续运行；用户无法通过设置关闭，违反“显式 settings > env（仅未保存时 fallback）”，并保留隐藏配置。 | 用“settings 文件是否存在/本次是否显式保存”决定 override，而不是值是否等于 default；文件存在或 settings.set 时 override 恒为 Some（包括 disabled/10）；仅无设置文件时 clear。更新测试：env=1 + 显式 default → disabled；env=1 + enabled/30 → enabled/30；boot 文件存在默认也 disabled。同步 docs/38 语义。 |

**P2（非阻塞加固/证据/文档）**

| # | 位置 | 问题 | 修法 |
| --- | --- | --- | --- |
| P2-1 | `packages/client/tool-schedule/src/lib.rs:77`（tool description 仍 env-only）；`packages/client/tool-schedule/README.md`（“执行器后续接”）；docs/38 已改 | 工具 spec/README 与 settings 真相不一致，模型/用户仍看到旧“env opt-in/未接执行器”说明，属隐藏配置文档残留。 | 更新 tool description/README 为 `AppSettings.agent.schedule`（env fallback）；与 docs/38 同步。 |
| P2-2 | `packages/contracts/config/src/lib.rs:281`（`AgentSettings::validate` 未调用 `self.schedule.validate()`）；`shared/contracts/schema/schedule-settings.json`（scan_seconds 无 5..3600 bounds） | 契约/schema 边界校验不完整：desktop settings.set/UI/apply 有 range 检查，但 `AgentSettings::validate`/`AppSettings::validate`/JSON schema 接受 4/3601（只有 minimum 0，无 max）；外部消费者依赖 schema 时校验不足。 | `AgentSettings::validate` 调 schedule.validate；为 scan_seconds 生成 min/max（schemars range 或手写 schema overlay）；补 conformance/schema 测试（4/3601 对 AgentSettings/AppSettings/schema 均拒绝）。 |
| P2-3 | `specs/changes/2026-10-31-round9-schedule-config/tasks.md`；acceptance | tasks 仍全 `pending`；acceptance 仍 in-progress；实现已完成/已验证，状态滞后。 | C19 通过后标 done 并填证据。 |
| P2-4 | `.github/workflows/ci.yml:58-78`（integration job）；本机执行 | GitHub runner 的 `sudo systemctl start postgresql`/本地 apt PG 是否在 hosted runner 稳定、job 是否实际绿未验证（只解析 YAML + 本地同脚本 0）；不应把本地 srv-auth 0 当成 CI 已绿。 | 在真实 CI 跑一次并记录；若 systemd 不可用改用 `pg_ctlcluster`/service；保持 evidence 诚实。 |
| P2-5 | `apps/desktop/src/automation.rs` 测试（spawn_with/ENV_TEST_LOCK）；`apps/desktop/web/tests/app-settings.test.tsx` | 热更新只有单测（spawn_with + IPC apply），没有真实 desktop/serve e2e 验证 settings.set→executor enable/disable/period/boot；UI 只测提交 payload，不测实际 effective state。 | 增加 e2e/serve：settings UI 保存 → executor 行为；boot 文件存在默认/非默认；env fallback disable 用例。 |
| P2-6 | `docs/19`（无 schedule 设置说明）；`ui-settings` UI 不展示 env fallback effective 状态 | 文档/UX truth 不完整：用户看到关闭但 env 可能启用（P1 修复后 env 仍 fallback；需提示或 meta.get effective）。 | docs/19 增加 schedule 设置；UI 显示“环境变量 fallback”提示或 effective 状态（可选）。 |

## 5. UNVERIFIED（环境限制，不计入本轮缺陷）

- GitHub `integration` job 实际运行（本机无 GitHub runner/CI）；只验证 YAML 解析 + 本地 srv-auth 0。
- Tauri 真机/Windows、真实模型调度压测、Docker/多平台、cargo-deny、真实私有 OEM。
- `systemctl start postgresql` 在 ubuntu-24.04 hosted runner 的实际行为。

## 6. 证据路径

- 门禁：`target/evidence/audit/round9/{fmt,clippy,workspace-tests,pnpm-and-schema}.log`
- e2e：`target/evidence/audit/round9/{agent,m1,admin,login}-e2e.log`
- srv-auth：`target/evidence/audit/round9/srv-auth.log`（本机真实 PG EXIT 0）
- CI YAML 解析/步骤：见 §2/§3.5（`python3 yaml.safe_load(.github/workflows/ci.yml)`）

## 7. 结论与状态

- 最新结论：**CHANGES-REQUESTED**（P0=0；P1=1；P2=6）。ScheduleSettings/UI/热更新主体成立；P1-1（显式默认值被 env fallback 重新启用，无法关闭自动调度）阻塞 APPROVED。
- 写本记录前 `review-check.py` 报 1（缺本记录）；写入后仍 **1**（CHANGES-REQUESTED），P1 修复并复评 APPROVED 后应为 0。
- 本记录是唯一新增文件；未修改产品代码/生成物/committed 文件。

## 8. 复评（HEAD d83d965）

- 复评对象：`HEAD d83d965`（`fix(schedule): explicit saved defaults override env fallback; schema bounds`；父 e45b100）。工作树 clean（唯一未跟踪文件 = 本记录）。
- 复评结论：**APPROVED（P0=0 / P1=0）**。首评 P1-1 已修复并独立复跑通过；P2-1/P2-2 已闭环，P2-3/4/5/6 作为非阻塞残留保留。

### 8.1 P1-1 复核（已修复）

| 项 | 证据 |
| --- | --- |
| 优先级 | `apps/desktop/src/automation.rs:65-83`：`apply_settings` 现无论值是否 default 都写 `Some(ScheduleRuntimeConfig)`；`clear_settings_override` 仅由 fresh-install/测试调用。`apps/desktop/src/ipc.rs:225-245`：`apply_persisted_schedule(settings, file_exists)`，文件存在即 apply（默认 disabled/10 也 pin），仅文件不存在时 clear；boot 调用在 executor start 前（main.rs:174 vs :212）。 |
| 测试 | `automation::tests::schedule_config_precedence_settings_env_default`：env=1 + `apply_settings(default)` → disabled/10；enabled/30、disabled/45 均正确。`ipc::tests::schedule_settings_apply_path_and_env_fallback`：boot no file + env=1 → enabled；boot file exists + env=1 + default → **disabled**；settings.set 默认 disabled 胜 env；无 file + 无 env → disabled。均 pass（416 passed/0 failed）。 |
| 文档 | `docs/38:33` 明确“settings 文件/显式保存（含默认）> env；env 仅新装首启”；`tool-schedule` description/README 已改为 `AppSettings.agent.schedule`（env fallback）。 |
| 契约/schema | `AgentSettings::validate` 现在调用 `self.schedule.validate()`（config lib:350）；`ScheduleSettings.scan_seconds` 增加 `#[schemars(range(min=5,max=3600))]`；生成 schema `minimum=5.0/maximum=3600.0` 且 `export_schema` 测试断言；conformance 覆盖 AgentSettings 4/3601 拒绝。 |

### 8.2 复评门禁（d83d965，独立执行）

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` | **0 / 0** |
| `cargo test --workspace` | **EXIT 0**（418 blocks / **416 passed** / 0 failed；含 schedule precedence/validation/schema 测试） |
| `cargo test --workspace export_schema` + `git diff --exit-code` | **0 / 0**（bounds 确定性） |
| `bash scripts/schema-check.sh` | **0** |
| `zk validate` / `compose personal --check` / `capability` / `docs` / `db lock` / `lint --duplicates` | **0 / 0 / 0 / 0 / 0 / 0** |
| `python3 scripts/agent-tool-matrix.py --check` | **0** |
| `pnpm -r typecheck` / `lint` / `test:unit` / `build` | **0 / 0 / 0 / 0** |
| `bash scripts/srv-auth-test.sh`（真实 PG） | **EXIT 0** |
| `bash scripts/agent-e2e.sh` / `m1-e2e.sh` | **0 / 0**（9 passed / 7+7 passed；复评重跑） |
| `python3 scripts/docs-check.py` / `spec-check.py` | **0 / 0** |
| `python3 scripts/review-check.py` | 写本记录前 1；写入后应为 **0**（latest APPROVED） |

### 8.3 剩余 P2（非阻塞，Lead 已确认保留）

- **P2-3**：`specs/changes/2026-10-31-round9-schedule-config/tasks.md` 仍全 pending，acceptance 仍 in-progress；C19 通过后应更新。
- **P2-4**：GitHub `integration` job 未在真实 hosted runner 执行（YAML/步骤正确、本地 srv-auth 0；`systemctl start postgresql`/apt PG/30m 稳定性 UNVERIFIED），不应声称 CI 已绿。
- **P2-5**：热更新仍只有单测（spawn_with/IPC apply），无真实 desktop/serve e2e 验证 settings.set→executor enable/disable/period/boot；UI 只测 payload，不测 effective state。
- **P2-6**：`docs/19` 无 schedule 设置说明；fresh install + env=1 时 UI 显示默认 disabled，但 executor 实际 enabled（env fallback），UI 未展示 effective 状态；建议 docs/19 + UI 提示/meta.get。
- （可选兼容性备注）已有 settings 文件的用户在升级后 env=1 将不再启用 auto-run；这是新优先级语义，docs/38 已写明“settings 文件 > env”，如需提示可在 release notes 说明。

### 8.4 复评证据

- `target/evidence/audit/round9-rereview/{fmt,clippy,workspace-tests,export-schema,pnpm}.log`
- `target/evidence/audit/round9-rereview/{srv-auth,agent-e2e,m1-e2e}.log`

- 终评结论：**APPROVED（HEAD d83d965；P0=0 / P1=0；P2-3/4/5/6 已记录）**。
