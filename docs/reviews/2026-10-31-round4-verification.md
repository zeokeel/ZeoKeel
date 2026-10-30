# 2026-10-31 round-4 独立评审记录（usage 持久化 / settings 分 section / workspace.changes / T22）

- 评审对象：HEAD **`5e53177a0aa1414512ef0ce9caf36985a36bdb43`**（`docs(spec): add round-4 usage persistence/settings/changes proposal`）+ 未提交工作树（36 个改动文件 + 5 个新增文件，见 §5 哈希表）。评审者未修改任何产品代码；仅新增本记录。
- 评审者：**fixer-server**（独立只读评审者）。**与实现者关系/独立性**：本评审者没有参与 round-4 的任何实现（round-4 由 Lead 与其他 agent 编写）；本评审者此前负责的是服务端 billing/auth/audit（task-8/task-10），与本轮客户端数据/UI 路径无重叠，故为独立评审。工作树可能被其他并行任务继续修改，复评时须重新执行 `git status` 并重算 §5 哈希。
- 关联 spec：`2026-10-31-round4-data-and-ui`（proposal `approved`；acceptance 仍 `in-progress`）。
- 结论：**CHANGES-REQUESTED**（P0 = 0；P1 = 4；P2 = 10）。latest 结论以本行为准。
- 复评日期/结论：pending（Lead 修复 P1 后由同一评审者复评本记录并更新本行）。

## 1. 可执行门禁（本机实跑，HEAD `5e53177` + 上述工作树）

| 命令 | rc | 说明 |
| --- | --- | --- |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 0 | |
| `cargo test --workspace` | 0 | 全部单测/契约测试；desktop/store/agent-rig 新增测试通过 |
| `cargo test --workspace export_schema` | 0 | 执行后 `git status shared/contracts` 与本轮工作树一致（无额外漂移：2 改 + 2 新 schema + contracts.ts） |
| `bash scripts/schema-check.sh` | 0 | `schema-ts: up to date` |
| `cargo run -q -p zk -- validate` | 0 | 114 packages |
| `cargo run -q -p zk -- compose --edition personal --profile default --channel official --check` | 0 | 16 files up to date |
| `cargo run -q -p zk -- capability --check` | 0 | 45 entries |
| `cargo run -q -p zk -- docs --check` | 0 | 2 files |
| `cargo run -q -p zk -- db lock --check` | 0 | 16 migrations |
| `cargo run -q -p zk -- lint --duplicates` | 0 | 94 files，61 clone windows ≤ baseline 61 |
| `python3 scripts/docs-check.py` / `spec-check.py` | 0 / 0 | 40 files；0 warnings |
| `python3 scripts/review-check.py` | **1** | 预期且真实：记录结论为 CHANGES-REQUESTED，报 `latest review conclusion is CHANGES-REQUESTED (need APPROVED)`；Lead 修复 P1 后复评改 APPROVED 才转绿 |
| `pnpm -r typecheck` / `lint` / `test:unit` / `build` | 0 / 0 / 0 / 0 | 含 `inspector-changes.test.tsx` 5 例、`validation.test.ts` 新负向例 |
| e2e（m1/agent/login/admin/workspace.changes 场景） | **未运行（有意）** | 见 §7 UNVERIFIED：`scripts/{admin,login,oem,updates}-e2e.sh` 的 cleanup 调 `restore-baseline.sh`（`git checkout -- Cargo.lock` + 生成物），会回滚本轮未提交的生成物；且端口/共享工作树可能与并行任务冲突。评审者未跑 e2e，不以 acceptance 的“已通过”自述当证据 |
| `cargo deny check` | **UNVERIFIED** | 本机未安装（沿用前轮） |

工具链：`rustc 1.99.1` / `cargo 1.99.1` / `node 26.13.0` / `pnpm 12.16.1` / PostgreSQL 16.15（本机可用，但本轮产品改动不涉及服务端）。

## 2. P1（必须修复后才可 APPROVED）

### P1-1 Usage 已持久化但 Web 加载路径丢弃 usage，重启后 UI 仍不可见（spec §1/成功指标未达成）
- 位置：
  - `apps/desktop/web/src/protocol.ts:237-257` `parseMessageRecord`：校验 `session_messageSchema`（已含 `usage`）后构造对象时**没有读取/回填 `usage`**；
  - `apps/desktop/web/src/state/reducer.ts:135-147` `toChatMessage`：`messages_loaded` 走此映射，**没有 `usage` 字段**；
  - 使用链：`apps/desktop/web/src/state/controller.ts:263-276`（`session.messages` → `parseMessages` → `messages_loaded` → `toChatMessage`）。
- 规格依据：`specs/changes/2026-10-31-round4-data-and-ui/spec.md:9`（"UI 从消息恢复 usage 展示"）、`design.md:7`（"web parseMessages 已有 usage 字段则直接恢复"）、`proposal.md:33`（成功指标："usage 重启后可见"）。当前只完成了契约 + SQLite + host 写入与 store 读取；**UI 恢复显示缺失**，重启后 `ChatMessage.usage` 恒 `undefined`，`packages/ui/ui-chat/web/src/runtime.ts:69` 不会渲染 usage data part。
- 修法：`parseMessageRecord` 解析并校验 `usage`（可复用 `parseUsage`/生成的 `AgentUsage` 类型，非法值降级 `null`）；`toChatMessage` 增加 `usage: message.usage ?? null`；补一条 reducer/protocol 单测（加载带 usage 的历史消息 → ChatMessage.usage 非空 → UI 有 usage part），并把它作为 acceptance"usage 重启后可见"的证据（当前 e2e 缺该场景）。

### P1-2 "tools-only 保存 + env agent 生效"在真实 settings.set 路径仍是 false-green（spec §2/成功指标未达成）
- 位置：`apps/desktop/src/ipc.rs:380`（`SETTINGS_SET` **无条件**调用 `apply_agent_settings`）→ `apps/desktop/src/ipc.rs:167-180` `zk_contract_agent::set_default_agent_settings(&settings.agent)`；而 `packages/client/agent-rig/src/config.rs:375-393` 的优先级是 `published.agent > legacy registry > env > default`。因此仅保存 tools（agent 仍为默认）时：snapshot 的 `agent` 为 `None`，但 contract-agent registry 被写成默认值，env `ZEOKEEL_AGENT_*` 被默认 registry 覆盖。
- 实测（评审者临时探针，`/tmp/r4probe`，未改仓库）：设 `ZEOKEEL_AGENT_TEMPERATURE=0.42`，模拟 desktop 行为（`set_default_agent_settings(default)` + `RuntimeSettings{agent:None, tools:Some(2000/9000)}`），`zk_agent_rig::default_agent_config()` 输出 **`temperature=None`**（期望 `Some(0.42)`），`tool_timeout=2000` 正确。规格 `spec.md:15`、`proposal.md:34` 明确要求该场景 env 生效。
- 单测为何未发现：`packages/client/agent-rig/src/config.rs:556-610` 的 `per_section_precedence_between_published_settings_and_env` 只调用 `set_runtime_settings`，**没有模拟 desktop 的 `apply_agent_settings` registry 写入**；`apps/desktop/src/ipc.rs` 的测试只测 `runtime_snapshot_for` 构造，未组合 `apply_agent_settings` + `default_agent_config`。
- 修法（二选一，需保持旧 host 兼容）：
  1) 在 `SETTINGS_SET` 仅在 agent section 非默认时写 contract-agent registry；但需先提供 registry 清除 API（当前 `zk-contract-agent` 只有 set/get，`agent/src/lib.rs:323-339`），否则从"非默认"改回"默认"会留下陈旧非默认值；
  2) 桌面路径不再写 contract-agent registry（仅 `set_runtime_settings` 分 section；registry 仅留给旧 host），并确保 hook sink 的设置仍生效；或在 `default_agent_config` 中把 registry 视为低于 env（与本轮"published section > legacy registry/env > default"的措辞一致需重新定义优先级）。
  修复后补一个真实路径测试：`apply_agent_settings(default)` + tools-only snapshot + env agent → env 生效；同时覆盖"先保存非默认 agent 再改回默认"。

### P1-3 `workspace.changes` 在仓库内执行 git 配置命令（`core.fsmonitor`）→ 本地任意代码执行
- 位置：`apps/desktop/src/changes.rs:70-76`：`Command::new("git").arg("-C").arg(root).arg("status")...`，未禁用仓库本地配置驱动的命令。`git status` 会执行 repo-local `core.fsmonitor` 指定的程序（git 2.43）：
- 复现（评审者实测，临时目录）：
  ```
  git init -q && git config core.fsmonitor /tmp/zk-fsm.sh   # 脚本写 /tmp/zk-fsm-marker
  git status --porcelain -z >/dev/null
  # → FSMONITOR_EXECUTED=YES
  git -c core.fsmonitor=false status --porcelain -z >/dev/null
  # → MITIGATED_EXECUTED=NO
  ```
  工作区可能来自克隆/解压（含 `.git/config`）的不可信仓库；仅打开右栏"变更"tab 就会执行其中的命令，与 `docs/26/28` 的只读/安全预期不符（弱管控≠本地静默执行）。
- 修法：命令行加 `-c core.fsmonitor=false`（必要时同时 `-c core.pager=cat`），并设 `GIT_OPTIONAL_LOCKS=0`（或 `--no-optional-locks`）；更稳妥是给子进程一个最小环境（清除/固定 `GIT_DIR/GIT_WORK_TREE/GIT_CONFIG_*`，`GIT_CONFIG_NOSYSTEM=1`）。补负向测试：带 `core.fsmonitor` 的临时仓库调用 `workspace_changes` 不得产生副作用文件。

### P1-4 T22 host DTO 清理未实现（spec §4 / design §4 明确交付项缺失）
- 位置（host 仍手写 `json!`，未使用生成的契约 DTO）：
  - `apps/desktop/src/ipc.rs:236-241` `META_GET` 手写 `{edition,packages,version,agent_config_error}`（未用 `MetaInfo`）；
  - `apps/desktop/src/ipc.rs:432-445` `WORKSPACE_FILES` 手写 `{"items": items}`（未用 `WorkspaceFilesResult`/`FileEntry`）；
  - `apps/desktop/src/ipc.rs:1043-1047` `chat.send` 手写 `{stream_id,conversation_id,workspace_id}`（未用 `ChatSendResult`）。
  `grep -rn "WorkspaceFilesResult\|ChatSendResult\|MetaInfo" apps/desktop/src` 无结果；`packages/contracts/ipc` 的 DTO 只在 web/测试使用。
- 规格依据：`spec.md:25`（"桌面 host 返回 JSON 改用生成的 DTO 形状…"）、`design.md:19`（"host 构造 DTO struct 后 serde_json::to_value"）、`proposal.md:24`（T22 剩余 host JSON 清理）。本轮只完成了其中的 `question_asked` 校验与 `FileEntry` 镜像移除，host 三项未做 → 规格任务 4 不完整，且测试无法发现 host wire 与 schema 的漂移（false-green 风险）。
- 修法：为 `MetaInfo`/`WorkspaceFilesResult`/`ChatSendResult` 构造契约 struct 后 `serde_json::to_value`（或 `CommandResponse::ok(serde_json::to_value(dto))`）；补 host-vs-schema conformance 单测/契约测试；`shared/contracts/ts/contracts.ts` 已生成对应类型，可直接用于 web 侧类型收窄。

## 3. P2（建议修复/澄清，不阻塞本轮关闭，但 P1 修复后应顺带处理）

1. **`docs/28` inspector props 仍过时**：`docs/28-ui-module-map-and-skeleton.md:40` 的 props 列仍写 `open/tab/files/onTab/onClose`；实际 `InspectorPanelProps`（`packages/ui/ui-inspector/web/src/index.tsx:10-21`）已新增 `changes/changesState/changesError/changesWarning/onRetryChanges`。文档应与代码一致（docs/27 的表格已更新）。
2. **spec 状态未回填**：`specs/changes/2026-10-31-round4-data-and-ui/tasks.md` 6 项仍全 `pending`；`acceptance.md:1` 仍 `in-progress`、§1 待填。C16 要求完成时填 acceptance 并归档；Lead 应在 P1 修复后回填真实证据（含本轮 reviewer 复跑命令）。
3. **workspace.changes 无 e2e**：只有 `apps/desktop/web/tests/inspector-changes.test.tsx`（fake transport 单测）与 `apps/desktop/src/changes.rs` 的临时仓库单测；`e2e/` 无 `workspace.changes` 场景，spec task 3 验证列写"pnpm + e2e"、acceptance 表要求"workspace.changes 场景"e2e。建议加一条 serve 载体 Playwright 场景（打开右栏→变更 tab→空/有变更/错误重试，真实 git 临时仓库）。
4. **IPC 同步执行 5s git**：`apps/desktop/src/main.rs:59-69` 的 `#[tauri::command] ipc_call` 是同步函数，`serve.rs:145` 也在 async handler 内直接 `dispatch`；`workspace.changes` 最长阻塞 5s（`changes.rs:22`），会阻塞 webview 主线程/HTTP worker。建议 `spawn_blocking`/异步 command，或把超时降到 ~1–2s 并缓存。
5. **"只读"仍会写 `.git/index`**：实测（临时仓库，`git 2.43`）fresh commit 后普通 `git status` 会刷新并重写 index（hash 变化）；`--no-optional-locks`/`GIT_OPTIONAL_LOCKS=0` 下 hash 不变。P1-3 修法同源，可一并加；否则 docs/27/28 的"只读"措辞不精确。
6. **超限/截断语义边界**：`changes.rs:114-116` 用 `bytes.len() >= MAX_OUTPUT_BYTES` 判截断，恰好 2 MiB 的完整输出会被误报 `truncated`；而输出 >2 MiB 时 reader 停止读取，git 可能阻塞 5s 后走 timeout 分支，用户看到"timed out"而非"已截断的部分列表"。建议：只有 `> MAX` 才标记；超限时 `git status -uno`/分批或先 `--porcelain=v1 -z --untracked-files=normal` 依然可能超；至少补测试并统一 warning 文案。
7. **`question_asked` 兼容性收紧**：`apps/desktop/web/src/protocol.ts:493-500` `parseQuestions` 对 `questions` 非数组直接返回 null，整个事件被丢弃；旧实现接受 `questions:null`。当前 `tool-ask-user` 恒发非空数组（`packages/client/tool-ask-user/src/lib.rs:57-63`），故风险低；建议对 null 降级为空数组或显式记录"schema-required，legacy null 不再支持"。
8. **SQLite v3→v4 无备份**：`packages/client/store-session-sqlite/src/lib.rs:46-49` 仅 `version < 3` 备份；v4 是 `ALTER TABLE ... ADD COLUMN`（安全），但与 spec"备份既有机制"措辞相比应显式说明 v4 免备份，或在 v3→v4 前也做一次拷贝（成本低）。v2→v3 重建路径已含 backup，单测覆盖 v3→v4 与损坏 usage 降级。
9. **`From<&AppSettings> for RuntimeSettings` 仍 Some/Some**：`packages/contracts/config/src/lib.rs:399-406`；树上无调用者，但作为公开兼容 API 会重新引入"整文件显式"语义，建议文档标注"仅供 legacy 全量快照"或移除（C4 契约稳定需评估）。
10. **`set_default_agent_config` 仍发布两段**：`packages/client/agent-rig/src/config.rs:359-371` 显式写 `agent:Some,tools:Some`；对程序化 host 可接受，但与"未保存 section 不发布"的 desktop 语义不同，建议注释说明调用方语义。

## 4. 已核对通过 / 无问题项

- **契约兼容**：`Message.usage: Option<...> #[serde(default)]`（`packages/contracts/session/src/lib.rs:66-72`）向后兼容；legacy message JSON 无 usage → `None`（conformance 测试）；`RuntimeSettings` 两段 `Option + serde(default)`，旧全量 snapshot（agent/tools 均为对象）仍可解析（`packages/contracts/config/tests/conformance.rs:157-186`）；`workspace-change(s)` DTO 纯新增，`warning` 省略时前端按 null 处理（conformance 覆盖）。
- **生成物**：`session-message.json` 含 `AgentUsage` definition 与 nullable usage；`runtime-settings.json` 两段 nullable；`workspace-change.json`/`workspace-changes-result.json` 新增；`shared/contracts/ts/contracts.ts` 同步（`schema-check.sh` 0，`export_schema` 后无额外 diff）。session 契约依赖 agent 契约无环（`cargo test --workspace` 绿）。
- **SQLite v4**：fresh install 直接 v4；v2/v3→v4 路径含 legacy rebuild / `ALTER ADD COLUMN`（`store-session-sqlite/src/lib.rs:205-310`）；`append_message` 序列化 usage（`:608-624`）、`messages_in` 读取并容错（`parse_usage`，损坏→None+stderr；`:91-104`、`:648-664`）。测试：跨实例重启 usage 保留、schema v4、损坏 JSON 降级、v3 旧库升级（`tests/conformance.rs:214-310`）。
- **settings 分 section 的 snapshot 层语义**：`runtime_snapshot_for` 只包含非默认 section（`ipc.rs:185-207`）；agent-rig `default_agent_config` 逐段合并 published > registry/env > 默认（`config.rs:375-393`）；snapshot 层单测覆盖 tools-only/agent-only/both（`config.rs:556-610`）。P1-2 是 desktop `settings.set` 组合路径的遗漏。
- **workspace.changes host**：固定 argv 无 shell、`-z` 解析（rename 消费 orig path、`??/D` 粗粒度计数）、5s 超时 kill、2 MiB/500 项上限、非仓库/无 git → 空 items + warning、路径 canonicalize、`#![forbid(unsafe_code)]`；单测覆盖解析/rename/上限/临时仓库真 git/非仓库 warning。IPC 契约新增 command 常量与 DTO，web Ajv 校验 + 映射 + 空/加载/错误重试 5 例 UI 测试；P1-3/P2.4/P2.5 为加固项。
- **T22 web 侧**：`protocol.ts` 改用生成的 `FileEntry/WorkspaceFilesResult/QuestionAskedPayload` 类型；`validate.ts` 增加 `workspaceChangesResult` 边界；`question_asked` 使用生成 schema 校验（不再是仅测试的死 validator）；`types.ts` 删除手写 `FileEntry` 镜像；负向测试覆盖错容器/错类型/非法 kind/未知 questions。
- **admin app.tsx 门禁修复**：`Partial<Record<AdminPage, AdminPageComponent>>` 兼容 personal 空 registry 与 enterprise 全量 registry（`apps/admin/web/src/app.tsx`）；`pnpm -r typecheck/lint/build` 绿。
- **docs 27/28 部分**：inspector 变更 tab 的"可用/只读 git status/空加载错误重试"描述与实现一致（除 P2.1 props 列表）。
- **schema/config/store/agent-rig/ipc/desktop 单测**：`cargo test --workspace` 0；`pnpm -r test:unit` 0；`zk lint --duplicates` 94 files/61 ≤ baseline；无新增重复实现。

## 5. 评审对象哈希（工作树文件，SHA256；`HEAD=5e53177`）

| 路径 | SHA256 |
| --- | --- |
| `packages/contracts/session/src/lib.rs` | `e0cb342faa80856ea279c95477464fa657b062507a25fa7da0c576bec927168a` |
| `packages/contracts/config/src/lib.rs` | `cae3a4cee564705a5b9e15b4629610a4a2907a57031e79545265745041dfdc17` |
| `packages/contracts/ipc/src/lib.rs` | `486259cdfc4738daa37bbe5e3edeb9820a8040fb343ae180ea6ac462c98bee7b` |
| `packages/client/store-session-sqlite/src/lib.rs` | `10b8fed635bc6557d0e43beab6e801481594fd731f4f913e80f28b16b9d19e6c` |
| `packages/client/store-session-sqlite/tests/conformance.rs` | `7a9fdf4ee66c140ba50eef1165945bb0a4a4583dac747e7ce35c7ec4d547b955` |
| `packages/client/agent-rig/src/config.rs` | `6dfbf128fe7f934467ea3ee13f7dddc9619c959d928f7218e1b5dcd673442fa5` |
| `apps/desktop/src/ipc.rs` | `5b5122deac0e536dce4542c8725511e4f0e027feb218d75fb37d4d662d9f2713` |
| `apps/desktop/src/changes.rs`（新增） | `78aa177cd82a4d15d45f8d9c3a572a18b107bb97e58011c7fad510d5ba5bb25d` |
| `apps/desktop/src/events.rs` | `4765878d7c30008973a31d2a142e02b503e622e41672ccc981e0737b62c10714` |
| `apps/desktop/src/main.rs` | `8e08d25ac48a31aac6e57908630d155312ad2421f6e0252acdc584be98bccaf8` |
| `apps/desktop/web/src/protocol.ts` | `4606c80c4c659a04c7ed9f26bb964ffd6a9bf915a585b3afaa5b54bbcb6965b4` |
| `apps/desktop/web/src/validate.ts` | `10b658b98f5fceb17b4c0dc8a3288615ed31d52f898feb57bd88bc301b9225ef` |
| `apps/desktop/web/src/App.tsx` | `158694e003dbff54a9ae3af116374c2ef4b2aed632a8744e4847b4bba8db120b` |
| `apps/desktop/web/src/state/reducer.ts` | `2f53287c9b967bd729ca2c2983928a810ea3276e03bd8c1a3a30bcd8eee4a896` |
| `apps/desktop/web/src/types.ts` | `8ad27cb7ec24f6f5179cb278c653155cb57ccbe5686d1f064cf3edef54aff787` |
| `packages/ui/ui-inspector/web/src/index.tsx` | `9c332efe9c1104fd38d78e8b0fcfbdb4552acb9891a5ff76e24150cc1c831316` |
| `docs/27-ui-design-system.md` | `1c69eeb9702db5455d841ba1ce784c7ba1acbfdfbc0b3cc626187996afb40479` |
| `docs/28-ui-module-map-and-skeleton.md` | `d28e940f570862b9ee5dc4d3da3cc16e8573d9662c1d575145f996091b9c50e9` |
| `shared/contracts/schema/session-message.json` | `460ad2e9e5e74d46253362be9e1b7265ffe0b8ede3cd9f112fa770a38baf10ca` |
| `shared/contracts/schema/runtime-settings.json` | `38385d692d1b5efda69854f5c246d099f0262db682f24f5ad7782d6d791181cb` |
| `shared/contracts/schema/workspace-changes-result.json`（新增） | `5241f5f1a398e042af00ee94e69e8cc586267e06aca5f8ab173c21fa790e2d1d` |
| `shared/contracts/ts/contracts.ts` | `4de69c1e55f4bcb0b54ddf59003c6fe7aed6541f2315f87f20705874e0a9735f` |
| `scripts/updates-e2e.sh` | `bcf44f38b06cafc1fb8ce95dfb5def35cb0e43a11f3251bd29463e84bcbff730` |
| `scripts/oem-e2e.sh` | `f42c4a102cebb3b8ef64ff4fe87ebac01bbc053b4d84873f60e872776838240e` |

其余本轮改动文件（`apps/admin/web/src/app.tsx`、`apps/desktop/web/src/i18n.tsx`、`apps/desktop/web/tests/*`、`apps/desktop/web/src/transport/mock.ts`、`packages/contracts/*/tests/*`、`packages/client/tool-session-query/tests/tool.rs`、`packages/ui/ui-inspector/web/src/index.tsx` 之外的 UI 测试、`scripts/{admin,apps,login}-e2e.sh`、`shared/contracts/schema/workspace-change.json`）在同一 worktree 快照中；复评以 `git status` + 重算为准。本记录自身不在评审对象哈希表内（评审后新增，会使 docs 工作树变化，但不改变被评产品对象）。

## 6. 评审结论摘要

- **P0 = 0**：未发现丢数据/破坏契约/安全边界突破级问题；旧 DB/旧消息兼容路径有测试。
- **P1 = 4**：usage 的 UI 恢复缺失、settings.set 组合路径仍覆盖 env（单测 false-green）、workspace.changes 的 git 配置命令执行风险、T22 host DTO 未清理。
- **P2 = 10**：docs props、spec/acceptance 回填、changes e2e 缺口、IPC 阻塞、只读/index、截断边界、question null 兼容、v4 备份说明、From/set_default 语义。
- 修复 P1-1/P1-2/P1-4 后可只读复跑 §1 门禁 + 新增单测/负向测试；P1-3 修复后补"fsmonitor 不执行"负向测试。复评由同一评审者在本记录追加"复评日期/结论"并改最新结论行为 **APPROVED**（或列剩余项）。

## 7. UNVERIFIED（不得写成已验证）

- e2e（m1/agent/login/admin/workspace.changes 场景、usage 重启显示）：本轮**未运行**。原因：e2e cleanup 的 `restore-baseline.sh` 会 `git checkout -- Cargo.lock` 与生成物，可能回滚本轮未提交的 round-4 生成物；且端口与共享工作树受并行任务影响。acceptance 若写 e2e passed，需附可复现命令/日志路径，且在本轮 P1 修复后重跑。
- `cargo deny check`（本机无工具；CI pin 版本无本地输出）。
- Tauri **窗口模式**下 `workspace.changes`/usage 显示（e2e 走 serve 载体或 fake transport，未覆盖 `tauri::command` 线程阻塞行为）。
- 真实大型仓库（>2 MiB status / >500 changes / 5s 超时）行为只有代码路径与解析单测，无真实大仓库证据。
- Docker/多平台/真实上游 provider/Windows/macOS 打包：沿用前轮 UNVERIFIED。

## 8. 复评记录

- 复评日期/结论：**pending**（等待 Lead 修复 P1-1..P1-4 并存证；复评时更新本节与文首"latest 结论"行，重跑 §1 门禁，必要时重算 §5 哈希）。

- 最新结论（复评前）：**CHANGES-REQUESTED**（P0 = 0；P1 = 4；等待 Lead 修复后由同一评审者复评并改本行为 APPROVED）。
