# 2026-10-31 round-4 独立评审记录（usage 持久化 / settings 分 section / workspace.changes / T22）

- 评审对象（初评）：HEAD **`5e53177a0aa1414512ef0ce9caf36985a36bdb43`** + 未提交工作树（round-4 初版）。
- 评审对象（复评，当前有效）：HEAD **`67a7e7ae4f892c7b83d1fcfa8ac6cf46345adf23`**，工作树 **clean**（`git status --short` 为空；round-4 修复已提交，`git log 5e53177..67a7e7a` 共 6 个提交）。复评对象路径 + SHA256 见 §5。
- 评审者：**fixer-server**（独立只读评审者）。**与实现者关系/独立性**：本评审者没有参与 round-4 的任何实现（实现由 Lead 与其他 agent 完成）；本评审者此前负责服务端 billing/auth/audit（task-8/task-10），与本轮客户端数据/UI 路径无重叠，故为独立评审。复评仍由同一评审者执行。
- 关联 spec：`2026-10-31-round4-data-and-ui`（proposal `approved`；acceptance 待 Lead 在本次 APPROVED 后改 done）。
- 结论：**APPROVED**（复评后；P0 = 0、剩余 P1 = 0；初评 P1 = 4 已全部修复并独立复核）。latest 结论以本行为准。
- 复评日期/结论：**2026-10-31，APPROVED**（HEAD `67a7e7a`；详见 §8）。

## 1. 可执行门禁（初评：HEAD `5e53177` + 当时工作树）

| 命令 | rc | 说明 |
| --- | --- | --- |
| `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets --all-features -- -D warnings` / `cargo test --workspace` | 0 / 0 / 0 | |
| `cargo test --workspace export_schema` | 0 | 当时工作树 generated 无额外漂移 |
| `bash scripts/schema-check.sh` | 0 | |
| `cargo run -q -p zk -- validate` / `compose --edition personal --profile default --channel official --check` / `capability --check` / `docs --check` / `db lock --check` / `lint --duplicates` | 0 / 0 / 0 / 0 / 0 / 0 | lint：94 files，61 clone windows ≤ baseline 61 |
| `python3 scripts/docs-check.py` / `spec-check.py` | 0 / 0 | |
| `python3 scripts/review-check.py` | 1 | 初评 CHANGES-REQUESTED（预期） |
| `pnpm -r typecheck` / `lint` / `test:unit` / `build` | 0 / 0 / 0 / 0 | |
| e2e | 未运行 | 当时工作树未提交；e2e cleanup 的 `restore-baseline.sh` 会回滚未提交生成物 |

### 1.1 复评门禁（HEAD `67a7e7a`，worktree clean，本机实跑）

| 命令 | rc | 说明 |
| --- | --- | --- |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 0 | |
| `cargo test --workspace` | 0 | 含新增/修复单测（store v4、agent-rig precedence、changes fsmonitor 负向、host DTO、delivery updates/bootstrap/channel） |
| `cargo test --workspace export_schema` | 0 | 执行后 `git status` 干净（无 generated 漂移） |
| `bash scripts/schema-check.sh` | 0 | `schema-ts: up to date` |
| `cargo run -q -p zk -- validate` | 0 | 114 packages |
| `cargo run -q -p zk -- compose --edition personal --profile default --channel official --check` | 0 | 16 files up to date |
| `cargo run -q -p zk -- capability --check` | 0 | 45 entries |
| `cargo run -q -p zk -- docs --check` | 0 | 2 files |
| `cargo run -q -p zk -- db lock --check` | 0 | 16 migrations |
| `cargo run -q -p zk -- lint --duplicates` | 0 | 94 files，61 clone windows ≤ baseline 61 |
| `python3 scripts/docs-check.py` / `spec-check.py` | 0 / 0 | 40 files；0 warnings |
| `python3 scripts/review-check.py` | 0 | 本次记录改为 APPROVED 后（复评最终跑） |
| `pnpm -r typecheck` / `lint` / `test:unit` / `build` | 0 / 0 / 0 / 0 | desktop web 77 tests / 14 files（含 usage-restore、inspector-changes） |
| `bash scripts/m1-e2e.sh`（复评重跑） | 0 | **14 passed / 15 skipped** |
| `bash scripts/agent-e2e.sh`（复评重跑） | 0 | **9 passed** |
| `bash scripts/login-e2e.sh`（复评重跑） | 0 | **1 + 3 = 4 passed**（first-run + logged-in） |
| `bash scripts/admin-e2e.sh`（复评重跑） | 0 | **11 passed (50.1s)** |
| `updates/apps/oem e2e` | 未由评审者重跑 | 交付轮证据 `target/evidence/audit/round4-delivery.md`（updates 0、apps 0、oem 0），Lead 复跑 0 |
| `cargo deny check` | **UNVERIFIED** | 本机未安装（沿用前轮） |

复评日志：`target/evidence/audit/round4-rereview/*.log`；复评结束后工作树 `git status --short` 为空。

## 2. P1（初评必须修复项；复评已全部关闭）

### P1-1 Usage 已持久化但 Web 加载路径丢弃 usage（**已修复@67a7e7a**）
- 初评问题：`apps/desktop/web/src/protocol.ts:237-257`（`parseMessageRecord` 未回填 usage）、`apps/desktop/web/src/state/reducer.ts:135-147`（`toChatMessage` 无 usage）；规格 `spec.md:9`、`proposal.md:33`“usage 重启后可见”不成立。
- 复评确认修复：
  - `apps/desktop/web/src/protocol.ts:237-267` 解析并校验 `usage`（`usage = parseUsage(value["usage"])`；非法 usage 降级 null 且不丢消息）；`apps/desktop/web/src/state/reducer.ts:142` `usage: message.usage ?? null`；
  - 新增 `apps/desktop/web/tests/usage-restore.test.tsx`：parseMessages（含 malformed usage 降级）、reducer `messages_loaded`、以及 **App 真实渲染恢复后的 usage part**（输入 5/输出 2/缓存 1/合计 7/成本 $0.000010）。
- 状态：**closed**。

### P1-2 “tools-only 保存 + env agent 生效”在真实 settings.set 路径仍覆盖 env（**已修复@67a7e7a**）
- 初评问题：`apps/desktop/src/ipc.rs:380` 无条件 `apply_agent_settings` → registry 写入默认值；`agent-rig/src/config.rs:375-393` registry 高于 env。评审探针实测 env 被默认 registry 覆盖。
- 复评确认修复：
  - `packages/contracts/agent/src/lib.rs:349` 新增 `clear_default_agent_settings()`；
  - `apps/desktop/src/ipc.rs:167-187`：agent section 为默认时 **clear registry**（非默认才 set），hook sink 逻辑保留；
  - `packages/client/agent-rig/src/config.rs:390` 起优先级改为 **published section > explicit env > legacy registry > defaults**（`env_agent_is_overridden` 仅 env 显式变化时生效）；
  - 测试：`apps/desktop/src/ipc.rs:1360-1404`（真实 `apply_agent_settings` + snapshot：tools-only 清 registry、save-custom→reset 清 registry）；`packages/client/agent-rig/tests/config.rs:221` `desktop_ipc_apply_sequence_keeps_env_authoritative`（tools-only 保存后 env max_steps=21/temp=0.31 生效；非默认 agent 覆盖 env；reset 回 env）。
- 状态：**closed**。

### P1-3 `workspace.changes` 在仓库内执行 git 配置命令（**已修复@67a7e7a**）
- 初评问题：`apps/desktop/src/changes.rs:70-76` 直接跑 `git status`，repo-local `core.fsmonitor` 可执行任意命令（初评实测 FSMONITOR_EXECUTED=YES）。
- 复评确认修复：
  - `apps/desktop/src/changes.rs:76-93` `git_status_command`：`env_clear()` 仅保留 PATH/HOME；`GIT_CONFIG_NOSYSTEM=1`、`GIT_CONFIG_GLOBAL=/dev/null`、`GIT_OPTIONAL_LOCKS=0`；argv 固定 `-c core.fsmonitor=false --no-optional-locks -C <root> status --porcelain -z`；
  - `apps/desktop/src/changes.rs:327-377` `repo_local_fsmonitor_is_not_executed`：先证明 fixture 敏感（plain git status 执行 marker），再断言 `workspace_changes` 不产生 marker；
  - 同时关闭 P2.5（只读：`--no-optional-locks`/`GIT_OPTIONAL_LOCKS=0` 防 index 刷新写入）。
- 状态：**closed**。

### P1-4 T22 host DTO 清理未实现（**已修复@67a7e7a**）
- 初评问题：`apps/desktop/src/ipc.rs:236-241`（MetaInfo）、`:432-445`（WorkspaceFilesResult）、`:1043-1047`（ChatSendResult）手写 `json!`。
- 复评确认修复：
  - `apps/desktop/src/ipc.rs:246-255` 用 `MetaInfo` struct + `serde_json::to_value`；`:552-594` `list_workspace_files` 返回 `WorkspaceFilesResult`，dispatch `:458` `serde_json::to_value`；`:1064-1069` `ChatSendResult` struct + `to_value`；
  - DTO 归属调整：`FileEntry/FileKind/WorkspaceFilesResult` 从 ipc 契约移到 `zk-contract-session`（`packages/contracts/session/src/lib.rs:94-117`；schema 仍由 `session/tests/schema.rs` 生成同一 `workspace-files-result.json`）；`MetaInfo/ChatSendResult` 仍在 `zk-contract-ipc::results`；host 与 web 都使用生成契约类型；
  - 测试：`apps/desktop/src/ipc.rs:1445` `host_result_dtos_serialize_the_generated_contract_shapes`（MetaInfo/ChatSend/WorkspaceFiles 与生成 schema/字段一致）。
- 状态：**closed**。

## 3. P2（初评建议项；复评状态）

1. **`docs/28` inspector props 过时** → **已修复@67a7e7a**：`docs/28-ui-module-map-and-skeleton.md:40` 现列 `open/tab/files/changes/changesState/changesError/changesWarning/onRetryChanges/onTab/onClose`，与 `InspectorPanelProps` 一致。
2. **spec 状态/acceptance 回填** → **已修复@67a7e7a**：`tasks.md` 6 项 done；`acceptance.md` 已补实现/P1 修复/复评证据（状态仍 `in-progress`，Lead 在本次 APPROVED 后改 `done`）。建议把 acceptance 的“## 1. 待填”标题改为“验收结果”（纯文案）。
3. **workspace.changes 无 e2e** → **仍开放（P2）**：当前覆盖为 `inspector-changes.test.tsx`（fake transport，5 例）+ Rust 临时仓库单测；`e2e/` 仍无 `workspace.changes` 场景。不阻塞，建议后续补 serve 载体 Playwright 场景。
4. **IPC 同步执行 5s git** → **仍开放（P2）**：`apps/desktop/src/main.rs:60` 的 `ipc_call` 仍为同步命令，`serve.rs` 直接 dispatch；`workspace.changes` 最长阻塞 5s。建议后续 `spawn_blocking`/降低超时。
5. **“只读”会写 `.git/index`** → **已修复@67a7e7a**（`--no-optional-locks` + `GIT_OPTIONAL_LOCKS=0`，见 P1-3）。
6. **超限/截断语义边界** → **仍开放（P2）**：`changes.rs:149` 仍用 `>= MAX_OUTPUT_BYTES`（恰好 2 MiB 误报 truncated）；>2 MiB 仍可能走 5s timeout。未阻塞。
7. **`question_asked` 对 null questions 收紧** → **仍开放（P2）**：`protocol.ts` `parseQuestions` 非数组即丢弃事件；wire 契约要求数组，`tool-ask-user` 恒发非空数组，风险低。
8. **SQLite v3→v4 无备份** → **仍开放（P2，文档澄清）**：`store-session-sqlite/src/lib.rs` 仍仅 `version < 3` 备份；v4 只是 `ALTER ADD COLUMN`，建议 docs 写明免备份依据。
9. **`From<&AppSettings>` 仍 Some/Some** → **仍开放（P2）**：`packages/contracts/config/src/lib.rs` 保留兼容实现；树上无调用者，建议文档标注 legacy。
10. **`set_default_agent_config` 仍发布两段** → **仍开放（P2）**：`agent-rig/src/config.rs` programmatic API 语义与 desktop 不同；建议注释说明。

## 4. 已核对通过 / 无问题项

- **契约兼容**：`Message.usage: Option<AgentUsage> #[serde(default)]` 向后兼容；legacy message 无 usage → `None`；`RuntimeSettings` 两段 `Option + serde(default)`，旧全量 snapshot 可解析；`workspace-change(s)` 纯新增。复评 `cargo test --workspace` + `export_schema` 0。
- **生成物**：`session-message.json`（AgentUsage nullable）、`runtime-settings.json`（nullable sections）、`workspace-change.json`/`workspace-changes-result.json`、`workspace-files-result.json`、`contracts.ts` 与 Rust 契约一致（`schema-check.sh` 0；export_schema 后 `git status` clean）。
- **SQLite v4**：fresh v4；v2/v3→v4 rebuild/ALTER；append 序列化 usage、messages_in 容错解析；测试覆盖跨实例、schema v4、损坏 JSON 降级、v3 旧库升级。复评未发现回退。
- **settings 分 section（修后）**：snapshot 只含非默认 section；真实 desktop IPC 路径 tools-only 保存清 registry → env 生效；agent-only/env-tools 对称；save-custom→reset 清 registry（测试见 P1-2）。`default_agent_config` 文档化优先级与测试一致。
- **workspace.changes（修后）**：固定 argv、无 shell、5s 超时 kill、2 MiB/500 项上限、非仓库/无 git → 空 items + warning、rename 解析、fsmonitor/env 硬化 + 负向测试；web Ajv 校验 + 映射 + 空/加载/错误重试 5 例 UI 测试；mock transport。
- **T22**：host 三处结果改用契约 DTO（P1-4）；web 使用生成类型（`FileEntry/WorkspaceFilesResult/QuestionAskedPayload/WorkspaceChangesResult`），`question_asked` 用生成 schema 校验；负向测试覆盖错容器/错类型/非法 kind/未知 questions；手写 `FileEntry` 镜像已删除。
- **交付 P1（随本 HEAD 一并提交，非本记录初评 P1）**：SIGTERM 总预算 30s + 有界 bootstrap shutdown；updates ETag/304、forced/min_supported、签名字段+配置密钥 fail-closed、Range/206/416、426；bootstrap 目录/品牌/权限/config_etag/ETag/304/426/503；portal `/`+平台卡片+HTML 转义；`/api/channel` public+ETag/304。复评门禁 + e2e（m1/agent/login/admin）独立重跑通过；updates/apps/oem 未由评审者重跑（交付轮证据 + Lead 复跑 0）。交付项属 task-34/35 记录范围，本记录只做集成复核。
- **admin app.tsx 门禁修复**：personal 空 registry / enterprise 全量 registry 均 type-safe（`pnpm typecheck/build` 0）。
- **docs**：docs/27、docs/28、docs/04/24/38 与实现一致（`docs-check` 0）；`zk docs --check` 0；重复门禁 0。

## 5. 复评对象路径 + SHA256（HEAD `67a7e7a`，worktree clean）

| 路径 | SHA256 |
| --- | --- |
| `packages/contracts/agent/src/lib.rs` | `828dcacd34ad8e39c0312200f771881b3f41efe3b2b9866e6c055913fdb4bc7c` |
| `packages/contracts/session/src/lib.rs` | `e0cb342faa80856ea279c95477464fa657b062507a25fa7da0c576bec927168a` |
| `packages/contracts/config/src/lib.rs` | `cae3a4cee564705a5b9e15b4629610a4a2907a57031e79545265745041dfdc17` |
| `packages/contracts/ipc/src/lib.rs` | `486259cdfc4738daa37bbe5e3edeb9820a8040fb343ae180ea6ac462c98bee7b` |
| `packages/client/store-session-sqlite/src/lib.rs` | `10b8fed635bc6557d0e43beab6e801481594fd731f4f913e80f28b16b9d19e6c` |
| `packages/client/store-session-sqlite/tests/conformance.rs` | `7a9fdf4ee66c140ba50eef1165945bb0a4a4583dac747e7ce35c7ec4d547b955` |
| `packages/client/agent-rig/src/config.rs` | `e8af0d63d681815a1c5683d4176567dd4a7c99a787649bb2fe4f5cdeef341e0d` |
| `packages/client/agent-rig/tests/config.rs` | `a9933047d1b36669afb724693fb99dccf9babb7c20265bf75ffe2e9555b16054` |
| `apps/desktop/src/ipc.rs` | `4c6b815b25913e8f3d2dacb4fc89d6dcfd79f86f056ef4f35cc57ac398013140` |
| `apps/desktop/src/changes.rs` | `95c247bcd9d70ce92c5469bfa76e0fc5816042c52868ed819891867dc7b53e58` |
| `apps/desktop/web/src/protocol.ts` | `ad02e701a198f7391ca18ce44ff3323715ba3a9a3806912b761b202fbf2742f7` |
| `apps/desktop/web/src/validate.ts` | `10b658b98f5fceb17b4c0dc8a3288615ed31d52f898feb57bd88bc301b9225ef` |
| `apps/desktop/web/src/App.tsx` | `158694e003dbff54a9ae3af116374c2ef4b2aed632a8744e4847b4bba8db120b` |
| `apps/desktop/web/src/state/reducer.ts` | `22e1b96272620ae591cbce8aebca5e048744123979f28fe40e6101fda1222d2c` |
| `apps/desktop/web/tests/usage-restore.test.tsx` | `7a91e02a59ca6321f16215f671024a31e2b6b74f8b563f98bf95fd9038d14dcd` |
| `apps/desktop/web/tests/inspector-changes.test.tsx` | `ead9a1b72d8b28c29ae01b2abda29084ba5839c4725514043d741234b4574bd5` |
| `packages/ui/ui-inspector/web/src/index.tsx` | `9c332efe9c1104fd38d78e8b0fcfbdb4552acb9891a5ff76e24150cc1c831316` |
| `docs/27-ui-design-system.md` | `1c69eeb9702db5455d841ba1ce784c7ba1acbfdfbc0b3cc626187996afb40479` |
| `docs/28-ui-module-map-and-skeleton.md` | `6f6e44cc31808cc9d4b6d8ce76438ff6998e9c2767950b0a57776812f83ff39f` |
| `specs/changes/2026-10-31-round4-data-and-ui/acceptance.md` | `4572e97f1187968503636f99433a56df406e8b3428bc71baf6cc63c18b766ec1` |
| `shared/contracts/schema/session-message.json` | `460ad2e9e5e74d46253362be9e1b7265ffe0b8ede3cd9f112fa770a38baf10ca` |
| `shared/contracts/schema/runtime-settings.json` | `38385d692d1b5efda69854f5c246d099f0262db682f24f5ad7782d6d791181cb` |
| `shared/contracts/ts/contracts.ts` | `4de69c1e55f4bcb0b54ddf59003c6fe7aed6541f2315f87f20705874e0a9735f` |
| `packages/server/updates/src/lib.rs` | `9345dc39724b3e0cd852b7ee49bb2d9a3497a53d062d4a559f4b0024344cf91f` |
| `packages/server/updates/tests/http.rs` | `9a297ab3452ca41f93808b0831fcb9d6552497c93b9a8997be6ab58168dd7275` |
| `packages/server/settings/src/lib.rs` | `da3506638d973736c16263f0493e4f7075581636cc8571707208b1a15655a226` |
| `apps/server/src/main.rs` | `f906bb08f407274273b3ede97bee9903828cd373b71dc1184766ac9630b3b6a6` |

> 复评后若这些路径再被修改（包括 Lead 把 acceptance 改 done 时只改 acceptance 状态/标题），本记录对被改文件失效需重算；哈希由 `sha256sum <path>` 复现。

## 6. 评审结论摘要

- **P0 = 0**：未发现丢数据/破坏契约/安全边界突破级问题；旧 DB/旧消息兼容路径有测试。
- **P1 = 0（初评 4 项全部关闭）**：usage UI 恢复、真实 desktop settings 优先级、workspace.changes git 配置执行、T22 host DTO。
- **P2 剩余（不阻塞本轮关闭，建议后续）**：changes e2e 缺口、IPC 同步 5s、截断语义边界、question null 兼容、v4 备份说明、From/set_default 语义、acceptance 标题文案。
- 结论：**APPROVED**（2026-10-31，复评 HEAD `67a7e7a`）。

## 7. UNVERIFIED（不得写成已验证）

- **全局业务面 426 `CLIENT_TOO_OLD` 层**：本轮未验证（updates manifest 的 426 已实现并有单测）。
- **真实签名密钥/证书**：updates 签名字段与配置密钥 fail-closed 有单测；真实 ed25519 密钥轮换/证书链未验证。
- Docker/多平台/installers/SBOM、真实媒体 provider、真实 OEM 私有索引/submodule（当前 dir_hash 语义）、`cargo deny check` 本地：沿用前轮 UNVERIFIED。
- Tauri **窗口模式**下 `workspace.changes`/usage 显示（e2e 走 serve 载体，未覆盖 `tauri::command` 线程行为）。
- 真实大型仓库（>2 MiB status / >500 changes / 5s 超时）行为：仅代码路径 + 解析单测，无真实大仓库证据。
- updates/apps/oem e2e 本轮未由评审者重跑（Lead 与交付轮证据 rc 0；评审者复跑了 m1/agent/login/admin）。

## 8. 复评记录：HEAD `67a7e7a`（round-4 C19 修复）

- 复评对象：HEAD **`67a7e7ae4f892c7b83d1fcfa8ac6cf46345adf23`**，工作树 **clean**；修复提交区间 `5e53177..67a7e7a`：`15edd29`（usage/settings/changes IPC 初版）、`f3ae67e`（usage UI 恢复/生成 DTO）、`3365f63`（docs truth/acceptance）、`0985025`（admin registry fallback）、`1ed2b76`（updates/bootstrap/channel + 有界 shutdown）、`67a7e7a`（portal e2e 标签）。
- 独立性：同一独立评审者（fixer-server），未参与实现；复评只读 + 更新本记录。
- 复评门禁（本机实跑，全部 0）：`cargo fmt --all -- --check`；`cargo clippy --workspace --all-targets --all-features -- -D warnings`；`cargo test --workspace`；`cargo test --workspace export_schema`（`git status` clean）；`scripts/schema-check.sh`；`zk validate` / `compose personal --check` / `capability --check`（45）/ `docs --check` / `db lock --check`（16）/ `lint --duplicates`（94 files，61 ≤ baseline 61）；`docs-check.py`（40 files）/ `spec-check.py`；`review-check.py`（本记录 APPROVED 后 0）；`pnpm -r typecheck/lint/test:unit/build`（desktop web 77 tests / 14 files）。日志：`target/evidence/audit/round4-rereview/`。
- e2e（评审者重跑）：m1 **14 passed / 15 skipped**；agent **9 passed**；login **4 passed**（1 + 3）；admin **11 passed (50.1s)**；随后工作树 clean。updates/apps/oem 未重跑（引用交付轮 rc 0 证据；本轮 updates/bootstrap 新增单测已通过）。
- P1 关闭确认：P1-1 见 §2（protocol/reducer + `usage-restore.test.tsx` 含 App 渲染）；P1-2 见 §2（`clear_default_agent_settings` + `apply_agent_settings` 清 registry + agent-rig env>legacy + 双测试）；P1-3 见 §2（`git_status_command` env/config 硬化 + fsmonitor 负向测试）；P1-4 见 §2（host DTO struct + to_value + 契约序列化测试）。
- P2 变化：初评 P2.1/2.5 **已修**（docs/28 props、只读 index）；P2.2 部分完成（tasks/acceptance 已回填，标题文案待 Lead 改）；其余 P2 保留（不阻塞）。
- 交付 P1（Lead 同批提交，初评记录未列）：SIGTERM 30s 总预算 + 有界 bootstrap shutdown、updates ETag/签名/Range/426、bootstrap catalogs/etag/304/426/503、`/api/channel`、portal `/`+平台卡片+转义——复评门禁 + m1/agent/login/admin e2e 通过；updates/apps/oem e2e 引用交付轮证据（未由本评审者重跑）。上述交付项属于 task-34/35 记录范围，本记录只做集成复核，不替代其验收记录。
- 复评结论：**APPROVED**（2026-10-31；P0 = 0、剩余 P1 = 0；最终评审对象 HEAD `67a7e7a`，worktree clean）。
- Lead 后续：把 `specs/changes/2026-10-31-round4-data-and-ui/acceptance.md` 状态改为 `done`，并把 "## 1. 待填" 标题改为“验收结果”；acceptance 的 C19 验收人字段建议写 `fixer-server`。

- 最新结论（复评后）：**APPROVED**（2026-10-31，HEAD `67a7e7a`；P0 = 0、P1 = 0；剩余 P2 不阻塞本轮关闭）。
