# 2026-10-31 round-3 独立评审记录（config/vector/T22/kernel/UI）

- 评审对象：HEAD **`e37cca6`**（round-3 提交区间 `3996707..e37cca6`：`e2e3390` 工具/审批超时、`201b3b0` 向量持久化、`cce2fa9` T22 DTO、`c57466e` 重复门禁+文档、`ceabf65` UI、`79106c7` 内核取消、`4ff2201` admin fix、`e37cca6` acceptance）。评审时工作树 clean；评审者另独立复跑了 4 套 e2e，结束后 `git status` clean。
  受影响路径：`packages/contracts/{config,agent,ipc,session}`、`apps/desktop/{src,web}`、`packages/client/{agent-rig,cap-vector}`、`packages/server/jobs`、`apps/server/src/main.rs`、`apps/admin/*`、`packages/ui/*`、`tools/zk`、`shared/contracts/{schema,ts}`、`duplication-baseline.json`、`docs/25`、`docs/38`、round-3 spec。
- 评审者：auditor-rust（独立只读评审者）。**独立性声明**：round-3 内核 schedule/jobs 实现（提交 `79106c7`，本轮 task-27）由本评审者编写，该部分属自审；其余配置/向量/T22/UI/门禁部分为独立评审。C19 要求非实现者评审，建议 Lead 对 `79106c7` 追加一次只读确认（或由另一 reviewer 复核内核 diff）；本记录把该限制显式列出，不视为完全独立。
- 关联 spec：`specs/changes/2026-10-31-round3-completeness`。
- 结论：**CHANGES-REQUESTED**（P0 = 0；P1 = 1；P2 = 12）。latest 结论以本行为准。
- 复评日期/结论：2026-10-31，复评 **APPROVED**（HEAD `bcf74b4`；详见 §6；最终以文末 latest 结论行为准）。

## 1. 可执行门禁（HEAD e37cca6，本机实跑）

| 命令 | rc | 说明 |
| --- | --- | --- |
| `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets --all-features -- -D warnings` / `cargo test --workspace` | 0 / 0 / 0 | |
| `cargo test --workspace export_schema` | 0 | 62 个 schema；随后 `git status` 对 `shared/contracts` 干净 |
| `bash scripts/schema-check.sh` | 0 | SCHEMAS/TS 同步 |
| `zk validate` / `compose personal --check` / `capability --check` / `docs --check` / `db lock --check` | 0 / 0 / 0 / 0 / 0 | |
| `zk lint --duplicates` | 0 | 3 次独立运行均 `93 files, 61 clone windows <= baseline 61`；负向探针（新增 2×30 行重复文件）→ 64 > 61、rc=1；删除后恢复 61、rc=0（门禁真实生效且计数稳定） |
| `python3 scripts/docs-check.py` / `spec-check.py` | 0 / 0 | |
| `python3 scripts/review-check.py` | **1** | 本记录创建前：round-3 proposal 指向的 review record 不存在（预期；本记录创建后转为“latest conclusion needs APPROVED”） |
| `pnpm -r typecheck` / `lint` / `test:unit` / `build` | 0 / 0 / 0 / 0 | test:unit 合计 105 passed（与 acceptance 一致） |
| `bash scripts/agent-e2e.sh`（评审者复跑） | 0 | **9 passed** |
| `bash scripts/m1-e2e.sh`（评审者复跑） | 0 | **14 passed / 15 skipped**（29 tests） |
| `bash scripts/login-e2e.sh`（评审者复跑） | 0 | **4 passed**（1 first-run + 3 logged-in） |
| `bash scripts/admin-e2e.sh`（评审者复跑） | 0 | **11 passed** |
| `cargo deny check` | **UNVERIFIED** | 本机未安装 cargo-deny；CI pin 0.20.2（见 security.yml），无本地证据 |

## 2. P1（必须修复后才可 APPROVED）

### P1-1 Tauri setup 事件转发仍未由内核持有（spec 任务 5 标记 done，实际未实现）
- 位置：`apps/desktop/src/main.rs:359-367`：`.setup()` 内用 `tauri::async_runtime::spawn(async move { loop { receiver.recv().await … handle.emit("zk://event", …) } })` 启动 bus→Tauri 转发循环；该任务不在 `Bootstrap` effect 中，kernel abort/dispose 不会停止或等待它。`apps/desktop/src/main.rs:369-377` 的 `open_app_window` 一次性 spawn 同样未走内核（非本 P1 必需，但建议一并处理）。
- 规格依据：`specs/changes/2026-10-31-round3-completeness/spec.md:22`（“Tauri 事件转发由内核 task owner 持有”）、`design.md:19`（“改为 `bootstrap.spawn_task`（或在 shutdown 中等待）”）、`proposal.md:17/25`、`tasks.md:9`（任务 5 done）；当前实现与这些声明矛盾。
- 修法（任一）：
  1) 在 `run_window(state, bootstrap)` 的 setup 闭包捕获 `Arc::clone(&bootstrap)`，在 `tauri::async_runtime::spawn` 的一次性外壳内调用 `bootstrap.spawn_task("zk.desktop.tauri-events", async move { loop { … } })`。`spawn_task` 在 tokio 上下文内注册（setup 的 tauri runtime 已提供），长循环本身成为 kernel effect，`Bootstrap::shutdown()` 会 abort；记录 `Err` 到 stderr。一次性外壳不持有业务循环，不构成泄漏。
  2) 给 `zk-bootstrap` 增加 `spawn_task_with_handle(handle: tokio::runtime::Handle, …)`（本次 task-27 允许的 TaskHandle 变更范围），setup 内直接注册，避免嵌套 spawn。
  修完补一条可执行断言（如 `bootstrap.active_tasks()` 含 `zk.desktop.tauri-events`，或 shutdown 后 receiver 循环结束）并把 spec/task-5 的证据改为真实测试/冒烟；在此之前 acceptance 不能把任务 5 写成已验证。

## 3. P2（建议修复/澄清，不阻塞本轮关闭）

1. **`docs/24` §6 内部矛盾**：状态机（`docs/24-composition-and-runtime-spec.md:286-288`）写“结算/审计 flush → 停 cron/jobs”，同节实现状态（`:296`）写“停止 cron → 结算 → 审计校验”；当前代码（`apps/server/src/main.rs:157-159`）在 drain 后先停 jobs，再 `shutdown_sequence()`。以实现状态/代码为准回写状态机，避免运维文档误导。
2. **`docs/28`/`docs/27` inspector 文档过时**：`docs/28-ui-module-map-and-skeleton.md:40` 仍列 Inspector props `open/tab/files/changes/onTab/onClose`，`docs/27-ui-design-system.md:53` 仍写“变更 tab”；实际 `ui-inspector/web/src/index.tsx` 已移除 changes 输入并禁用该 tab（“coming later”）。spec §4 要求“明确禁用并文档化”，需在 docs 标记 changes 禁用/后置。
3. **`docs/25` 门禁表部分不实/不精确**：`:67` 写“60% shingle 候选 + 70% Jaccard”，实现（`tools/zk/src/main.rs:2577-2636`）是任意共享 7 行 shingle 为候选、Jaccard ≥70%，无 60% 阈值；`:65` 声称 `zk lint` 做 import 依赖图检查，实际 `lint()` 仅扫描禁止目录 + 重复代码（依赖边界未实现，属既有 gap）。建议改为准确描述并标注未实现项。
4. **重复门禁 ratchet 只做单边上限**：`tools/zk/src/main.rs:2512-2536` 只比较 `clone_windows > baseline`，忽略 baseline 的 `schema`/`files`；扫描范围/文件数变化或 baseline 陈旧时仍会通过。建议校验 `schema==1` 与 `files` 一致，并在低于 baseline 时提示 `--write-baseline`（真正 ratchet down）。
5. **`zk-contract-config` 全局 runtime 快照保存完整 `AppSettings`（含 provider plaintext `api_key`）**：`packages/contracts/config/src/lib.rs:383-390` 的 `set_runtime_settings` 克隆整个 settings；`apps/desktop/src/ipc.rs:364` 在 `settings.set` 时把已还原的真实 key 一起写入。任何链接了配置契约的包都能经 `runtime_settings()` 读取 key；建议只发布非敏感 section（tools/agent）或先清空 `provider.api_key`，并补单测。
6. **agent-rig 两个 settings 入口语义不一致**：`packages/client/agent-rig/src/config.rs:359-367` 的 `set_default_agent_config` 会把 `agent: AgentSettings::default()` 的 full snapshot 写回 `runtime_settings`，可能覆盖先前已发布的完整快照；`from_contract`（`:464` 起）把 `tools` 设为 `AgentRuntimeConfig::from_env().tools`，通过契约 `configure_agent` 无法表达已保存的 tool timeout。桌面当前不调用这两个 API，但公开 API 的语义需统一（或标注仅供外部 host）。
7. **`cap-vector` 与 spec §2 的原子性/权限措辞差距**：`insert_many`（`packages/client/cap-vector/src/lib.rs:365-372`）逐条 `insert`（每条一次 save），不是一次批量原子写；`ensure_dir`（`:558-570`）只对新建目录设 0700，已存在的宽松目录不会被收紧。另：无持久化 dir 时 `configure` 会清空内存索引（`:250-280`，文档已写但属 hot-swap 数据丢失风险）；建议把 spec/docs 对齐或批量 save + 现有目录 chmod。
8. **向量持久化缺模型/维度指纹**：`vector-index.json` 只存 `version/documents`，provider/model/维度变更后旧 embedding 仍被加载，同维不同模型会产生无意义相似度、异维恒 0；建议保存 model id + dims 并在 configure 时校验/降级。
9. **`cosine` 的产品级 lint 例外**：`packages/client/cap-vector/src/lib.rs:600` `#[allow(clippy::arithmetic_side_effects)]` 位于产品代码；docs/18 只明确允许测试模块 allow。改为 checked/局部数学封装，或在 docs/18 登记例外。
10. **`cap-vector` 容量检查在 embedding 之后**：`insert` 先 `embed(text)`（`:344`）后 `save_documents` 检查 10k/16MB，满索引时仍会发起 provider 调用；先查容量可省钱/延迟。
11. **jobs 关闭等待无上界、且 `JobsLoop` 无 Drop**：`packages/server/jobs/src/lib.rs:332` 的 `wait` 等当前 tick；一个长 job（retention/reconcile）可把 SIGTERM 关闭拖过 30s deadline；若 `JobsLoop` 在未 stop 的情况下 drop，内部 `tokio::spawn` 任务会 detach（不是 kernel effect）。建议有界 wait + 超时/中断日志，或让 `JobsLoop` Drop 时 stop。
12. **T22 DTO 仍非 host 唯一真源 + 一处死 API**：`apps/desktop/src/ipc.rs:217-222`（meta）、`:402`（workspace.files）、`:1003-1008`（chat.send）仍手写 `json!`，未序列化 `MetaInfo`/`WorkspaceFilesResult`/`ChatSendResult`，schema 与 host 有漂移风险；`apps/desktop/web/src/validate.ts:152` 的 `validateQuestionAsked` 生产代码未调用（仅测试），实际由 `agent_event` schema 覆盖 question_asked；`apps/desktop/web/src/types.ts:39` 仍手写 `FileEntry` 镜像。建议 host 使用契约 DTO 或加 host-vs-schema conformance 测试，删除未用 validator。
13. **文档/验收杂项**：`specs/changes/2026-10-31-round3-completeness/proposal.md:7-8` 有重复的 `- 评审：` 行（去掉一条）；`docs/38-agent-runtime.md:50` 写桌面经 `set_default_agent_config` 应用，实际经 `zk_contract_config::set_runtime_settings`/`set_agent_hook_sink`（修正措辞）；acceptance 的 admin/login e2e 结论缺持久化 Playwright 报告（`target/evidence/m2/admin` 为空，login 只有截图/home）；本次评审复跑日志在 `target/evidence/audit/logs-rust/r28-*-e2e.log`，acceptance 归档时应引用可复现路径而不是只写 passed 数字。acceptance 状态应在 P1 修复+证据回填后由 Lead 改为 done。

## 4. 已核对通过 / 无问题项

- **工具/审批超时**：`AppSettings.tools` serde default + schema default、范围 1000..600000 / 1000..1800000；agent-rig 每轮从 `effective_config().tools.tool_timeout()` 应用（测试 `configured_tool_timeout_is_enforced_per_turn`）；桌面 boot/settings.set 调 `set_runtime_settings` + `ApprovalBroker::set_timeout_ms`，fresh/default-only 不发布以保留 env fallback（`ipc.rs:1115-1140` 测试）；`settings.set` 越界值经 `validate_settings` 回传错误。
- **向量持久化**：configure 加载、insert/clear 原子写（同目录 temp 0600→rename、sync）、0600/0700（新目录）测试、损坏/超大/超量/缺失降级 + `persistence_reason`、跨实例恢复、clear 持久化、上限 typed `Capacity`，均通过；`cargo test` 绿。
- **T22**：新增 `FileEntry`/`WorkspaceFilesResult`/`MetaInfo`/`ChatSendResult`/`QuestionDef`/`QuestionAskedPayload` 契约+62 schema+TS；桌面 Ajv 在 `meta`、`workspace.files`、`chat.send`、`question_asked`（经 agent_event）边界校验，负向测试覆盖错容器/错类型/非法 kind/未知 questions 形状；host 现有 wire 形状（items/kind/meta 字段）与 schema 一致，旧字段只增不破坏。
- **内核**：`ScheduleExecutor` stop+join/Drop 测试通过；`JobsLoop` stop/wait 幂等测试通过；server SIGTERM smoke（repo 已有脚本）通过；`cargo test -p zk-desktop -p zk-srv-jobs -p zk-bootstrap` 绿。**自审限制见独立性声明**。
- **UI**：ui-sessions 本地 palette 删除，侧栏调用 `actions.openCommands()` + shell reducer `commandsOpen`（commands.test 通过）；inspector changes 输入移除并禁用（代码层）；admin 105 个 zh/en 字典 key 完整、无业务页面硬编码中文、admin e2e 11 passed；m1/agent/login e2e 复跑通过。
- **门禁**：compose/capability/docs/db lock/schema/duplicate 全绿；duplicate 计数稳定且负向探针能触发；pnpm 105 tests 绿。

## 5. UNVERIFIED（不得写成已验证）

- `cargo deny check`：本机无 cargo-deny，仅 CI pin 版本，无本地输出。
- 真实媒体 provider、Docker/多平台、真实上游 Responses、Windows/macOS 打包：沿用前轮 UNVERIFIED。
- Tauri **窗口模式**未被 e2e 覆盖（serve 载体为同一 dispatch，但不验证 `tauri::Builder::setup` 的转发任务）；P1-1 修复后应至少做一次 `cargo build -p zk-desktop` + 静态断言 bootstrap task 注册，或 Tauri 冒烟。
- 真实 git 变更数据源（inspector 已禁用，无实现）。

## 6. 复评记录：HEAD bcf74b4（round-3 P1/P2 fixes）

- 复评对象：HEAD **`bcf74b4`**（`2ba3c48` 非密 runtime snapshot + 内核 Tauri bridge、`85dfaff` 向量原子批/权限/指纹、`2973acb` JobsLoop bounded/Drop、`bcf74b4` 文档真源 + baseline schema 校验；工作树仅本 review record 未跟踪）。
- 复评门禁（本机实跑，HEAD bcf74b4）：`cargo fmt` 0 · `cargo clippy --workspace --all-targets --all-features -D warnings` 0 · `cargo test --workspace` 0 · `cargo test --workspace export_schema` 0（62→**63** schemas；`git status` 对 generated 干净） · `schema-check` 0 · `zk validate`/`compose personal --check`/`capability --check`(45)/`docs --check`/`db lock --check` 0 · `zk lint --duplicates` 0（93 files，61 clone windows ≤ baseline 61） · `docs-check`/`spec-check` 0 · `pnpm -r typecheck/lint/test:unit/build` 0/0/0/0（105 tests）。
- e2e（评审者重跑）：agent **9 passed**；m1 **14 passed / 15 skipped**；login **4 passed**（1+3）；admin **11 passed**；`restore-baseline` 0，随后工作树 clean。`cargo deny check` 仍 **UNVERIFIED**（本机无 cargo-deny；CI pin 0.20.2，无本地输出）。
- 交叉确认（内核 self-review 限制）：Lead P1 fix（`2ba3c48`）已由本评审者独立核对：`apps/desktop/src/main.rs:297-...` 捕获 `Arc<Bootstrap>`，setup 内经一次性 Tauri-runtime hop 调 `bootstrap.spawn_task("zk.desktop.tauri-events", loop)`，`zk.desktop.open-app` 同样注册；长循环归内核 effect，shutdown abort/dispose 生效。`cargo check -p zk-desktop` 0、desktop 单测 15 passed、`rustfmt --check`/`git diff --check` 0。内核提交 `79106c7` 仍标 self-review，不视为完全独立批准；本条只是对 Lead fix 的交叉确认。
- P1 关闭：原 P1-1（Tauri 事件转发非 kernel-owned）已修复并交叉确认；无剩余 P1。
- P2 关闭：非密 `RuntimeSettings{agent,tools}`（provider key/base_url 不进入 registry，`ipc.rs` 测试断言序列化无 secret）；agent-rig 合并到单入口；向量 insert_many 原子批/容量前置/目录 0700 收紧/model+dims+width 指纹不兼容降级/legacy 兼容/去除产品级算术 allow（新增 14+ 持久化测试）；JobsLoop wait 10s 有界 + 超时 abort + Drop stop（4 tests）；docs/24 关闭顺序、docs/27/28 inspector changes、docs/25 去重措辞/import 边界标“待实现”、docs/38 `set_default_agent_config` 措辞、proposal 重复评审行；baseline `schema` 校验；acceptance 补复评修复记录（状态仍 in-progress，待 Lead 在本记录 APPROVED 后改 done）。
- P2 仍留（不阻塞本轮关闭，建议后续）：
  1. **混合 settings 的 per-section 优先级边界**：`apps/desktop/src/ipc.rs:214-216` 在仅 tools 非默认时发布包含默认 `agent` 的 snapshot；`packages/client/agent-rig/src/config.rs:378-386` 让 snapshot.agent 优先于 env。评审探针（临时、已删除）：`ZEOKEEL_AGENT_MAX_STEPS=7` + `RuntimeSettings{agent:default, tools:{5000,9000}}` → `default_agent_config().max_steps == 50`。若语义是“保存任意 settings 即整文件显式”（当前实现），需在 docs/38/测试中明确；若沿用 round-2 的 per-section 语义，则应把 `RuntimeSettings` 的 agent/tools 改为 `Option` 并只发布显式 section（settings.set 同样适用）。
  2. `tools/zk/src/main.rs:2520-2535` 只校验 baseline `schema`，仍忽略 `files`；扫描范围变化/陈旧 baseline 不会被发现（计数 ratchet 本身稳定，负向探针 64>61 触发）。
  3. T22 host 仍手写 JSON（`apps/desktop/src/ipc.rs` meta/files/chat results），`validateQuestionAsked` 仍仅测试使用，`types.ts` FileEntry 仍是手写镜像；建议 host 用契约 DTO 或加 conformance，删除未用 validator。
  4. acceptance:11 写“62 schemas”，实际 **63**（新增 `runtime-settings.json`）；Lead 在把 acceptance 改 done 时同步该数字。
  5. `cargo deny check` 本地 UNVERIFIED；`docs/25` TS `jscpd` 门禁仍是声明未实现（本轮只实现 Rust duplicate）；usage 未持久化（§5.2/§5.4 已知）。
  6. Tauri bridge 目前只有静态+单测/编译证据，无 Tauri 窗口自动化断言（e2e 走 serve 载体）；建议后续 Tauri smoke 断言 `active_tasks()`/shutdown 后 receiver 停止。
  7. `zk-contract-config` 的进程级 runtime registry 仍位于契约 crate（架构 follow-up；已用非密 snapshot 降低风险）。

- 复评结论：**APPROVED**（2026-10-31；P0 = 0、剩余 P1 = 0；最终评审对象 HEAD `bcf74b4`；上述 P2 不阻塞本轮关闭）。
