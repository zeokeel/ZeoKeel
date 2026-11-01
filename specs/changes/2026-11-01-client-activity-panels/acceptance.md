# 2026-11-01-client-activity-panels · 验收与证据

> 本 spec 已通过 C19 独立评审：原 `docs/reviews/2026-11-01-client-phase2-specs.md` §15；2026-11-01 scope 修订（plan 移交 plan-mode、goal 改 `goal.get`）经 `docs/reviews/2026-11-01-client-activity-panels-revision.md` 复评 APPROVED。

- 验收人：待独立评审 / 实现者回填
- 日期：2026-11-01（预填；实现后回填实际日期）
- PR/commit：待填
- 证据根：`target/evidence/2026-11-01-client-activity-panels/`（日志 `logs/`、截图 `screenshots/`、`axe.json`、`golden/`）

> 状态：`done`（2026-11-01 实现完成；门禁与证据见 §1/§1.5；PR commit 由 Lead 统一提交后回填）。
> 范围提示：右栏 activity tab（jobs/subagent/agent-team/goal(goal.get)/todo/schedule/present）+ chat turnTail（仅 present 交付卡）；plan 由 `client-plan-mode` 负责（2026-11-01 修订）。
> 不展示 plan、不做 plan mode（已由 client-plan-mode 实现）；不新增第二套状态机/持久化。

## 0. 预算例外

> Lead 2026-11-01 追认：实际 20 个手写文件 + 15 生成物（规划 15），单目标且无第二套实现，不拆分。

## 1. 验收命令与结果

> 最终合并门禁（对最终工作树顺序重跑）：`.../logs/final-gates.txt`（typecheck/lint/test:unit/build、i18n、e2e 等价序列、`pnpm e2e:ui` 85 passed、compose --check、interaction gate --strict 全部 exit 0）。

| 命令 | 结果 | 证据路径 |
| --- | --- | --- |
| `cargo test -p zk-contract-ipc -p zk-desktop` | exit 0（contract schema 1 + desktop 90） | `.../logs/contract-desktop-tests.txt` |
| `cargo test --workspace` | exit 0（443 组 `test result: ok`，无 FAILED） | `.../logs/cargo-test-workspace.txt` |
| `cargo test --workspace export_schema` ×2 + 状态哈希对比 | exit 0；两次运行 `git status` 哈希不变（生成物幂等）。说明：工作树含本 change 未提交改动，故字面 `git diff --exit-code` 由 Lead 提交后复跑；此处用「重生成前后 status 哈希一致」证明幂等 | `.../logs/export-schema.txt` |
| `bash scripts/schema-check.sh` | exit 0（149 schemas / SCHEMAS 全嵌入） | `.../logs/schema-check.txt` |
| `zk validate` | exit 0（edition=personal profile=default channel=official packages=58） | `.../logs/zk-validate.txt` |
| `zk compose --edition personal --profile default --channel official --check` | exit 0（up to date 17 files） | `.../logs/compose-check.txt` |
| `zk capability --check` | exit 0（47 entries，无新增） | `.../logs/capability-check.txt` |
| `zk docs --check` | exit 0（2 files up to date） | `.../logs/zk-docs-check.txt` |
| `cargo check --workspace --all-targets` | exit 0 | `.../logs/workspace-check.txt` |
| `cargo fmt --all -- --check` | exit 0（含修复 HEAD 已存在的 `agent-rig` 未格式化漂移 2 行） | `.../logs/fmt-check.txt` |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | exit 0（含修复 HEAD 已存在的 `result_large_err`） | `.../logs/clippy.txt` |
| `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build` | exit 0（21 test files / 134 tests；最终重跑 exit 0） | `.../logs/pnpm-gates.txt`、`.../logs/final-gates.txt` |
| `bash scripts/i18n-check.sh` | exit 0 | `.../logs/i18n-check.txt` |
| `pnpm e2e`（字面命令） | **等价序列 exit 0**（见 §1.1）：mock 段 8 passed + serve 段 24 passed（最终重跑同结果）；`restore-baseline.sh` 在未提交工作树上 exit 1 属预期。字面 `pnpm e2e` 提交后已由 Lead 复跑：`HEAD 9a5f586` → **24 passed / exit 0**（`.../logs/pnpm-e2e-final.log`）；`export_schema && git diff --exit-code` 0 变更；`pnpm e2e:ui` 85 passed | `.../logs/e2e.txt` |
| `pnpm e2e:ui`（全量交互 e2e） | exit 0（85 passed，含本 change 11 条 ACT2-*；最终重跑同样 85 passed） | `.../logs/e2e-ui.txt`、`.../logs/final-gates.txt` |
| `python3 scripts/ui-interaction-gate.py --self-test && --strict` | exit 0（self-test 12 checks；strict：enforced=9 contracts，P0/P1=82 ids 全有 tag+证据，0 error） | `.../logs/interaction-gate.txt` |
| `python3 scripts/spec-check.py` | exit 0（0 errors；2 warnings 为其它 in-progress spec） | `.../logs/spec-gates.txt` |
| `python3 scripts/docs-check.py` | exit 0（0 errors / 41 files） | `.../logs/spec-gates.txt` |
| `python3 scripts/review-check.py` | exit 0（37 records） | `.../logs/spec-gates.txt` |
| 视觉基线（ACT2-VIS-01，P2） | **人工确认待办（UNVERIFIED）**：仅产出候选截图，未生成/接受 `toHaveScreenshot` 基线（C20：不得自动接受） | `.../screenshots/activity-light-1280.candidate.png` |

### 1.1 `pnpm e2e` 等价序列（restore-baseline 依赖已提交基线）

`cargo run -q -p zk -- compose --profile mock` → `ZEOKEEL_E2E_SKIP_SERVE=1 playwright test --grep-invert "serve carrier"`（8 passed）→ `bash scripts/restore-baseline.sh`（生成物回退到 HEAD 基线；其末尾 `compose --check` 因本 change 生成物未提交而预期红，脚本继续）→ `cargo run -q -p zk -- compose --profile default`（恢复本 change 的 personal 基线生成物）→ `playwright test e2e/m1-serve.spec.ts e2e/m1-serve-security.spec.ts e2e/m1-a11y.spec.ts e2e/mcp-config.spec.ts`（24 passed）。总 exit 0；日志 `.../logs/e2e.txt`。

## 1.5 交互验收（ID → 测试 → 证据）

> 状态：`active`（2026-11-01 实现）。**ID 合规修正**：依据 `interaction.md` §3「`<SURFACE>-<AREA>-<NN>`（AREA 如 SNAPSHOT/POLL/JOB/SUBAGENT/SCHEDULE/TURNTAIL/STALE/ERROR/A11Y/VIS）」与 `scripts/ui-interaction-gate.py` 的 ID 正则（两段 AREA），实现期把含连字符的 AREA 规范为单段：`ACT2-JOB-STOP-01→ACT2-JOB-01`、`ACT2-SUBAGENT-INTERRUPT-01→ACT2-SUBAGENT-01`、`ACT2-SCHEDULE-EDIT-01→ACT2-SCHEDULE-01`、`ACT2-SCHEDULE-DELETE-01→ACT2-SCHEDULE-02`、`ACT2-TURNTAIL-PRESENT-01→ACT2-TURNTAIL-01`；语义/优先级/测试/证据一一对应，评审记录保留历史 ID。

| ID | 优先级 | 测试 | 证据 |
| --- | --- | --- | --- |
| ACT2-SNAPSHOT-01 | P0 | `e2e/interaction/activity.spec.ts`（`@interaction:ACT2-SNAPSHOT-01`） | `target/evidence/2026-11-01-client-activity-panels/interaction/ACT2-SNAPSHOT-01.json` |
| ACT2-POLL-01 | P0 | 同上 | `.../interaction/ACT2-POLL-01.json` |
| ACT2-JOB-01 | P0 | 同上（job-stop-confirm：取消/Esc 不发 action；确认后停止；不产生工具行） | `.../interaction/ACT2-JOB-01.json` |
| ACT2-SUBAGENT-01 | P0 | 同上（subagent-interrupt-confirm：同口径，roster 按 parent） | `.../interaction/ACT2-SUBAGENT-01.json` |
| ACT2-SCHEDULE-01 | P0 | 同上（toggle/edit；interval<60 行内错误且不写盘） | `.../interaction/ACT2-SCHEDULE-01.json` |
| ACT2-SCHEDULE-02 | P0 | 同上（删除内联确认/键盘；未知 id/`deleted 0` → `activity/schedule-invalid`） | `.../interaction/ACT2-SCHEDULE-02.json` |
| ACT2-TURNTAIL-01 | P0 | 同上（present 去重 + 「在右栏查看」预览；无 present 不显示空卡；不展示 plan） | `.../interaction/ACT2-TURNTAIL-01.json` |
| ACT2-STALE-01 | P1 | 同上（轮询失败保留旧数据 + stale；恢复后消失） | `.../interaction/ACT2-STALE-01.json` |
| ACT2-ERROR-01 | P1 | 同上（加载失败 role=alert + 重试；各分区空态不白屏） | `.../interaction/ACT2-ERROR-01.json` |
| ACT2-A11Y-01 | P1 | 同上（axe 0 serious/critical + 键盘 tab/动作/确认；axe 汇总见 `.../axe.json`） | `.../interaction/ACT2-A11Y-01.json` |
| ACT2-VIS-01 | P2 | 同上（候选截图，不比对；人工确认后 `pnpm e2e:ui:update`） | `.../screenshots/activity-light-1280.candidate.png` |

## 1.6 直觉验收（C20）

- 文件：`intuition.md`（**active**，12 条 INT，P0 全覆盖）+ `intuition-walkthrough.md`（**active**，3 persona × 3 task + 摩擦表，无 OPEN P0）。
- P0 摩擦：job 作用域/已结束误停、job 停止/subagent 中断不可逆误操作（已改内联二次确认）、schedule 删除无确认/未知 id、轮询旧响应串会话——均已在 spec/design/interaction/tasks 落最小修法并关闭。
- 3×3 persona × top task 矩阵已满格（N/A 格有理由+替代验证）；P0 无 OPEN。
- 用户测试：**UNVERIFIED**（未执行；发布前 ≥5 人 think-aloud，证据 `target/evidence/2026-11-01-client-activity-panels/usability/`）；**未完成前 acceptance/发布文案不得声称「符合直觉/直观」**。
- 交互门禁：`python3 scripts/ui-interaction-gate.py --strict` exit 0（本 spec 10 个 P0/P1 id 全有 `@interaction` 标签 + 证据 + intuition/walkthrough 存在性与 ≥3 INT 行）。
- 评审：待 fresh-context 评审员按走查复核（P0 摩擦已关闭；e2e 只作代理，不代表直觉已证明）。

## 2. 手工检查

> 由可执行证据替代（e2e / host 单测），不依赖人工点击。

- [x] activity tab：order 40（files→changes→terminal→activity→plan）、打开即加载、2s 轮询、手动刷新、动作后刷新、切会话/关闭停止（ACT2-SNAPSHOT-01 / ACT2-POLL-01 e2e）
- [x] jobs：列表/输出摘要/状态；仅当前 workspace cwd；停止运行中 job；未知/已结束错误提示；空态（ACT2-JOB-01 e2e + `activity_jobs_are_workspace_scoped_and_kill_is_state_checked`、`activity_cwd_containment_is_component_wise` 单测）
- [x] subagent：roster（id/depth/status/prompt）按当前会话过滤；中断运行中；agent-team 最近结果只读；空态（ACT2-SUBAGENT-01 / ACT2-SNAPSHOT-01 e2e + `activity_subagent_roster_filters_parent_and_interrupt_checks_state`、`activity_snapshot_replays_tool_rows_and_excludes_goal` 单测）
- [x] goal：只读 `goal.get` 的 objective/status/rounds/max_rounds/reason；无 goal 空态；损坏降级；**不从工具行派生、不写**（ACT2-SNAPSHOT-01 e2e 断言 snapshot 无 `goal` 字段 + goal.get 读取；`activity_snapshot_*` 单测）
- [x] todo：list/add/update/remove replay 正确；只读；损坏/legacy 提示「由会话工具记录推导，可能不完整」（`activity_snapshot_replays_tool_rows_and_excludes_goal` 单测含 legacy/损坏行 + `derived_unavailable`）
- [x] schedule：列表/暂停/启用/编辑 prompt/interval/删除（确认）；文件断言；非法 interval/未知 id 拒绝；文件不存在/损坏降级（ACT2-SCHEDULE-01/02 e2e + `activity_schedule_update_delete_use_tool_and_reject_invalid` 单测含损坏文件 `activity/schedule-unreadable`）
- [x] turnTail：present 卡去重、点击打开右栏 files 预览；**不展示/折叠 plan**；无 present 不显示空卡（ACT2-TURNTAIL-01 e2e）
- [x] 六态/stale：空/加载/错误/离线/流式中/已取消不白屏；轮询失败保留旧数据 + stale 提示（ACT2-STALE-01 / ACT2-ERROR-01 e2e；离线/流式中为面板级 note + 保留旧数据，**未单独 e2e 断言 → UNVERIFIED**）
- [x] a11y/i18n：键盘可达 tab/动作/确认；axe 0 serious/critical；i18n 无硬编码（ACT2-A11Y-01 e2e + `bash scripts/i18n-check.sh` exit 0）
- [x] 边界：不修改 `tool-todo`/`tool-goal`/`tool-agent-team`/`tool-schedule` 行为；无新持久化/DB 迁移；**plan 不展示（plan-mode 负责）**（diff 无这些包改动；无 migration；e2e 断言 activity/turnTail 不出现 plan）

## 3. 文档与决策

- [x] `docs/16` §2.2/§2.4：右栏 tab order 文件：files 10 / changes 20 / terminal 30 / activity 40 / plan 50；不新增 slot
- [x] `KNOWN-LIMITATIONS.md` §3.4/§3.4.1：jobs/subagent 进程内/重启丢失；todo 工具行推导（legacy 不完整）；goal 只读 goal.get；schedule 无新建/历史；**plan 由 plan-mode 负责（activity 不展示）**；2s 轮询无事件推送；输出/条数截断；视觉基线候选未人工确认
- [x] proposal §7 D1–D4 拍板记录已回填（用户 2026-11-01 批准推荐项）
- [x] 已定项：activity 位置 = 右栏 tab order 40 + chat turnTail；**plan 展示移交 client-plan-mode（2026-11-01 修订）**；goal 只读 goal.get
- [x] `docs/26` §5.6：`activity.snapshot`/`activity.action` 命令 + `activity/*` 错误码登记（SSOT）
- [x] 无新增 ADR/D；`docs/README.md` 不适用；无新增 capability/INDEX 条目（`zk capability --check` 47 entries 不变）

## 4. 回滚

- 回滚步骤：回退 commit；无数据库迁移/新持久化需清理；schedule 文件由既有工具继续读写
- 数据影响：无；goal 只读 goal.get；todo/present 只读派生；jobs/subagent 进程内状态随重启清空
- 已验证回滚：无 DB migration/新持久化（`git status` 无 migrations 变更）；旧客户端忽略新 tab/命令；schedule 文件/工具行由既有工具继续读写；`cargo test -p zk-contract-ipc -p zk-desktop` 覆盖 additive 契约兼容。字面回滚演练由 Lead 提交后按需执行（未单独演练 → 语义 UNVERIFIED）。

## 5. 剩余风险 / 后续

| 项 | 影响 | 后续 |
| --- | --- | --- |
| jobs/subagent 是进程内状态，重启即失，面板不持久 | 中 | KNOWN；Phase 3 持久化/历史 |
| todo 由工具行 replay，可能不含 legacy 行/与进程内全局状态不完全一致 | 中 | KNOWN；后续可另立 spec 改 tool-todo per-conversation 持久化 |
| goal 与真实状态不一致 | 低 | 改读 goal.get；空/损坏测试 |
| schedule 无新建/执行历史；写入复用 tool-schedule 校验 | 低 | 后续 schedule 管理 spec |
| activity 2s 轮询（无事件推送） | 低 | Phase 3 事件通道 |
| 与 side-panels/composer 共享文件竞写 | 中 | 严格顺序（§tasks §5） |

## 5.5 评审条目关闭（2026-11-01 Phase 2 评审 §7）

| 条目 | 处理 | 验证 |
| --- | --- | --- |
| P1-2 IPC/错误码 SSOT | task 0/§3 增 `docs/26` §5 登记 activity 命令与错误码；预算 15（Lead 例外） | docs-check + `rg` 对照 contract |
| P1-3 schedule delete 冲突 | host 先 `list` 预检 id（不存在 → `activity/schedule-invalid`）；delete 输出 `deleted 0` 同映射；design §2.3/§2.4 唯一映射 | host 单测（未知 id/`deleted 0`） |
| P1-4 activity interaction.md | 新增本目录 `interaction.md`（planned，ACT2-* ID）；P0/P1 映射 `e2e/interaction/activity.spec.ts` + `target/evidence/.../interaction/<ID>.json`；tasks/acceptance 已列 | 交互门禁（工具已落地：`pnpm e2e:ui --grep activity` + `pnpm e2e:ui:gate`） |
| P1-5 交互门禁不可执行 | `e2e:ui`/gate 已落地（`d0b7741`）；实现时接入并跑 | gate exit 0 |
| P2-8 决策矛盾 | spec §6 改为已定（勾选 D1–D4，引用 proposal §7） | 文档对照 |
| P2-10 args 二次解析 | design 明确工具行 `args` 是 JSON 字符串 → `serde_json::from_str` 后再取字段；损坏/legacy 行测试 | host 单测 |
| P2-11 @generated 模板 | 改生成器源 `tools/zk/src/main.rs` 生成 facade；不手改模板/产物 | `zk compose --check` |
| P2-12 app 直接依赖 tool | 前端不 import；host 经 `zk-composed` 生成 facade；不新增 app→tool 直接依赖 | 代码评审 + compose |
| P2-13 schedule prompt 上限 | host 预校验 prompt ≤32 KiB（tool 无此校验） | host 单测 |
| P2-14 jobs cwd 过滤 | canonicalize + `Path::starts_with`（组件级），拒绝 `/workspace-evil` 前缀 | host 负向单测 |
| P2-15 docs/11 基线/门禁命令 | 本 spec 的 `interaction.md` 已列为 planned；docs/11 门禁命令/基线已由 `ui-interaction-verification` 落地，本 spec 不越界改 docs/11 | 复评核对 |

## 6. 结论

- [x] `done`：证据齐全（§1/§1.5），门禁 exit 0；待 Lead 统一提交 + fresh-context 复评确认，且用户测试仍 UNVERIFIED
- [ ] `abandoned`：原因：N/A

## 7. 复用证据（C17）

- 搜索记录：`rg -n "job_list|job_kill|subagent|agent_team|goal|todo|schedule|present|turnTail" packages apps crates docs`；
  `./target/debug/zk find kit.jobs`、`zk find kit.subagent`；capability 索引 `kit.jobs`（`shared/contracts/capability-index.json:107`）、
  `kit.subagent`、`contract.ipc`（`:43`）、`kit.tools`（`:131`）
- 复用清单：`zk_kit_jobs::{list,output,kill}`、`zk_kit_subagent::{list,runner}` + `SubagentRunner::{status,result,interrupt}`、
  goal.get（command-surface；goal 单 owner）、既有持久化工具行（todo/present/agent_team）、`zk_tool_schedule::tool().call_json`（list/update/delete）、Phase 2b 右栏/预览/turnTail 容器
- 新增包内实现（不登记 INDEX）：`activity.snapshot`/`activity.action` IPC + host 派生 helper；ui-inspector activity tab；ui-chat turnTail
- 无第二份实现：jobs/subagent 复用 kit registry；goal 复用 goal.get；todo/present 复用持久化工具行；schedule 复用工具包校验/读写；未修改工具状态模型

## 8. 实现记录（2026-11-01）

### 8.1 文件清单（写作用域内）

| # | 文件 | 说明 |
| --- | --- | --- |
| 1 | `packages/contracts/ipc/src/lib.rs` | `activity.snapshot`/`activity.action` 命令 + Activity* DTO（goal 不进 snapshot；`schedules_error`/`derived_unavailable` 为满足 R5/R6 的分区级字段） |
| 2 | `packages/contracts/ipc/tests/schema.rs` | 新增 12 个 schema 导出（生成器输入，非 @generated 产物） |
| 3 | `tools/zk/src/main.rs` | 生成 `zk-composed` 的 `zk-kit-jobs`/`zk-kit-subagent`/`zk-tool-schedule` facade（不手改 @generated） |
| 4 | `apps/desktop/src/ipc.rs` | `activity.snapshot`/`activity.action` + 派生 helper（tool 行 args 二次解析/todo replay/present 去重/team/截断/cwd 组件级过滤/schedule 预检）+ 5 个单测 |
| 5 | `packages/ui/ui-kit/web/src/types.ts` | Activity DTO 镜像 |
| 6 | `packages/ui/ui-kit/web/src/context.tsx` | `UiActions.activitySnapshot/activityAction/openInspectorPreview` |
| 7 | `packages/ui/ui-kit/web/src/primitives.tsx` | `Button` 支持 ref（确认区默认焦点取消；复用同一确认原语，C17） |
| 8 | `packages/ui/ui-inspector/web/src/index.tsx` | activity tab（order 40）/轮询/六态/确认/schedule 编辑删除/goal.get/previewRequest + tab 键盘焦点修复 |
| 9 | `packages/ui/ui-chat/web/src/AssistantThread.tsx` | turnTail present 交付卡（不含 plan） |
| 10 | `apps/desktop/web/src/{App.tsx,protocol.ts,validate.ts,i18n.tsx}` | actions 注入/previewRequest/Ajv 校验/zh+en 文案 |
| 11 | `apps/desktop/web/tests/ui-fixtures.tsx`、`packages/ui/ui-chat/web/tests/composer.test.tsx` | UiActions fixture 补齐 |
| 12 | `e2e/serve-m1.sh` | 假上游剧本（后台任务/子代理/待办+交付；ThreadingHTTPServer） |
| 13 | `e2e/interaction/activity.spec.ts` | 11 条 ACT2-* 交互测试（10 条 P0/P1 门禁 + P2 候选截图） |
| 14 | `docs/16-ui-packages.md`、`docs/26-workspace-and-conversations.md`、`KNOWN-LIMITATIONS.md` | 文档回写 |
| 15 | 生成物（`shared/contracts/{schema,ts}`、`crates/zk-composed/{Cargo.toml,src/lib.rs}`、`Cargo.lock`） | 由生成器写；`compose --check`/`schema-check` 绿 |

预算：15 + 4（schema 导出源、ui-kit primitives、2 个测试 fixture）+ 生成物/Cargo.lock，超 proposal §8 的 15 文件预算（未拆分；Lead 可裁定）。`Cargo.lock` 因 zk-composed 新增两个 workspace kit 依赖而更新。

### 8.2 与 spec/design 的差异（诚实记录）

1. **交互 ID 规范**：原 `ACT2-JOB-STOP-01` 等含连字符 AREA 不匹配 `ui-interaction-gate.py` 的 ID 正则（两段 AREA），实现期按 `interaction.md` §3「`<SURFACE>-<AREA>-<NN>`」规范化为 `ACT2-JOB-01`/`ACT2-SUBAGENT-01`/`ACT2-SCHEDULE-01`/`ACT2-SCHEDULE-02`/`ACT2-TURNTAIL-01`（语义不变；评审记录保留历史 ID）。
2. **Snapshot DTO 增补**：design §2.2 未列 `derived_unavailable`、`schedules_error` 两字段。R5（legacy/损坏提示）与 R6（schedule 损坏=分区错误而非整栏白屏）要求分区级信号，故 additive 增补；schema/TS 已生成。
3. **`ActivityTeamRun` 补充**：design §2.2 DTO 缺 objective/completed/failed/cancelled/total，而 spec R3 要求展示，故补齐。
4. **视觉基线**：只产出候选截图，未生成/接受 `toHaveScreenshot` 基线（C20：人工确认后才 `pnpm e2e:ui:update`）。

### 8.3 门禁前置修复（HEAD 已存在，非本 change 引入）

- `cargo fmt --all -- --check`：`packages/client/agent-rig/src/lib.rs`（commit 54149f8 的 summarizer 代码）未格式化，已 `cargo fmt`（2 行）。
- `cargo clippy -D warnings`：同文件 `result_large_err`（`Err` 变体 304 字节），已改为在闭包内 `map_err(|e| e.to_string())` 收窄错误类型（行为不变）。

### 8.4 UNVERIFIED / 遗留

- 用户测试（≥5 人 think-aloud）未执行；视觉/aria 基线未人工确认。
- 离线/流式中/已取消六态只做了「不白屏 + 保留旧数据」的面板级实现，未逐态 e2e 断言。
- 真实 Tauri WebView/三平台渲染未跑（平台矩阵属 M6）。
- axe：serious/critical = 0（门禁口径）；有 1 条 `heading-order` **moderate**（activity 分区标题用 h3，页面 h1 之下无 h2；不违反 0 serious/critical 口径，记录于 `.../axe.json`）。
- 字面 `pnpm e2e` 与字面 `git diff --exit-code` 需 Lead 提交基线后复跑（本 change 未提交状态下用等价序列/哈希对比证明）。
