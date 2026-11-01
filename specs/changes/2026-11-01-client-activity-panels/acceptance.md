# 2026-11-01-client-activity-panels · 验收与证据

> 本 spec 已通过 C19 独立评审：原 `docs/reviews/2026-11-01-client-phase2-specs.md` §15；2026-11-01 scope 修订（plan 移交 plan-mode、goal 改 `goal.get`）经 `docs/reviews/2026-11-01-client-activity-panels-revision.md` 复评 APPROVED。

- 验收人：待独立评审 / 实现者回填
- 日期：2026-11-01（预填；实现后回填实际日期）
- PR/commit：待填
- 证据根：`target/evidence/2026-11-01-client-activity-panels/`（日志 `logs/`、截图 `screenshots/`、`axe.json`、`golden/`）

> 状态：`draft`（未开工）。实现完成后按任务表逐项填证据路径；本文件不预写「已通过」。
> 范围提示：右栏 activity tab（jobs/subagent/agent-team/goal(goal.get)/todo/schedule/present）+ chat turnTail（仅 present 交付卡）；plan 由 `client-plan-mode` 负责（2026-11-01 修订）。
> 不展示 plan、不做 plan mode（已由 client-plan-mode 实现）；不新增第二套状态机/持久化。

## 1. 验收命令与结果

| 命令 | 结果 | 证据路径 |
| --- | --- | --- |
| `cargo test -p zk-contract-ipc -p zk-desktop` | 待跑 | `.../logs/contract-desktop-tests.txt` |
| `cargo test --workspace export_schema && git diff --exit-code` | 待跑 | `.../logs/export-schema.txt` |
| `bash scripts/schema-check.sh` | 待跑 | `.../logs/schema-check.txt` |
| `zk compose --edition personal --profile default --channel official --check` | 待跑 | `.../logs/compose-check.txt` |
| `zk capability --check`（无新增条目） | 待跑 | `.../logs/capability-check.txt` |
| `cargo check --workspace --all-targets` | 待跑 | `.../logs/workspace-check.txt` |
| `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build` | 待跑 | `.../logs/pnpm-gates.txt` |
| `bash scripts/i18n-check.sh` | 待跑 | `.../logs/i18n-check.txt` |
| `pnpm e2e:ui --grep activity`（工具已落地） | 待实现时跑 | `.../interaction/<ID>.json`、`.../axe.json` |
| `python3 scripts/spec-check.py && python3 scripts/docs-check.py` | 待跑 | `.../logs/spec-gates.txt` |
| `python3 scripts/ui-interaction-gate.py --self-test && python3 scripts/ui-interaction-gate.py` | 待实现时跑 | `.../logs/interaction-gate.txt` |

## 1.5 交互验收（ID → 测试 → 证据）

> 状态：`planned`；实现时把 `interaction.md` 改 `active` 并逐项填实际测试/证据。

| ID | 优先级 | 测试 | 证据 |
| --- | --- | --- | --- |
| ACT2-SNAPSHOT-01 | P0 | `e2e/interaction/activity.spec.ts`（待实现） | 待实现 |
| ACT2-POLL-01 | P0 | 同上 | 待实现 |
| ACT2-JOB-STOP-01 | P0 | 同上 | 待实现 |
| ACT2-SUBAGENT-INTERRUPT-01 | P0 | 同上 | 待实现 |
| ACT2-SCHEDULE-EDIT-01 | P0 | 同上 | 待实现 |
| ACT2-SCHEDULE-DELETE-01 | P0 | 同上 | 待实现 |
| ACT2-TURNTAIL-PRESENT-01 | P0 | `e2e/interaction/activity.spec.ts`（待实现） | 待实现 |
| ACT2-STALE-01 | P1 | `e2e/interaction/activity.spec.ts`（待实现） | 待实现 |
| ACT2-ERROR-01 | P1 | 同上 | 待实现 |
| ACT2-A11Y-01 | P1 | 同上 | 待实现 |
| ACT2-VIS-01 | P2 | 同上 | 待实现 |

## 1.6 直觉验收（C20）

- 文件：`intuition.md`（planned，12 条 INT，P0 全覆盖）+ `intuition-walkthrough.md`（3 persona × 3 task + 摩擦表，无 OPEN P0）。
- P0 摩擦：job 作用域/已结束误停、job 停止/subagent 中断不可逆误操作（已改内联二次确认）、schedule 删除无确认/未知 id、轮询旧响应串会话——均已在 spec/design/interaction/tasks 落最小修法并关闭。
- 3×3 persona × top task 矩阵已满格（N/A 格有理由+替代验证）；P0 无 OPEN。
- 用户测试：**UNVERIFIED**（未执行；发布前 ≥5 人 think-aloud，证据 `target/evidence/2026-11-01-client-activity-panels/usability/`）；**未完成前 acceptance/发布文案不得声称「符合直觉/直观」**。
- 评审：待 fresh-context 评审员按走查复核（P0 摩擦未关闭 → 不通过）。

## 2. 手工检查

- [ ] activity tab：order 40（files→changes→terminal→activity）、打开即加载、2s 轮询、手动刷新、动作后刷新、切会话/关闭停止
- [ ] jobs：列表/输出摘要/状态；仅当前 workspace cwd；停止运行中 job；未知/已结束错误提示；空态
- [ ] subagent：roster（id/depth/status/prompt）按当前会话过滤；中断运行中；agent-team 最近结果只读；空态
- [ ] goal：只读 `goal.get` 的 objective/status/rounds/max_rounds/reason；无 goal 空态；`.zeokeel/goal.json` 损坏降级；**不再从工具行派生、不写**
- [ ] todo：list/add/update/remove replay 正确；只读；损坏/legacy 提示「由会话工具记录推导，可能不完整」
- [ ] schedule：列表/暂停/启用/编辑 prompt/interval/删除（确认）；文件断言；非法 cron/interval/未知 id 拒绝；文件不存在/损坏降级
- [ ] turnTail：present 卡去重、点击打开右栏 files 预览、文件不存在错误；**不展示/折叠 plan**；无 present 不显示空卡
- [ ] 六态/stale：空/加载/错误/离线/流式中/已取消不白屏；轮询失败保留旧数据 + stale 提示
- [ ] a11y/i18n：键盘可达 tab/动作/确认；axe 0 serious/critical；i18n 无硬编码
- [ ] 边界：不修改 `tool-todo`/`tool-goal`/`tool-agent-team`/`tool-schedule` 行为；无新持久化/DB 迁移；**plan 不展示（plan-mode 负责）**

## 3. 文档与决策

- [ ] `docs/16` §2.2/§2.4：右栏 tab order 增加 activity 40；不新增 slot
- [ ] `KNOWN-LIMITATIONS.md`：jobs/subagent 进程内/重启丢失；todo 工具行推导（legacy 不完整）；goal 只读 goal.get；schedule 无新建/历史；**plan 由 plan-mode 负责（activity 不展示）**；2s 轮询无事件推送；输出/条数截断
- [ ] proposal §7 D1–D4 拍板记录已回填（用户 2026-11-01 批准推荐项）
- [ ] 已定项：activity 位置 = 右栏 tab order 40 + chat turnTail；**plan 展示移交 client-plan-mode（2026-11-01 修订）**；goal 只读 goal.get
- [ ] `docs/26` §5：`activity.snapshot`/`activity.action` 命令 + `activity/*` 错误码登记（SSOT）
- [ ] 无新增 ADR/D；`docs/README.md` 不适用；无新增 capability/INDEX 条目

## 4. 回滚

- 回滚步骤：回退 commit；无数据库迁移/新持久化需清理；schedule 文件由既有工具继续读写
- 数据影响：无；goal 只读 goal.get；todo/present 只读派生；jobs/subagent 进程内状态随重启清空
- 已验证回滚：待实现后填（旧客户端忽略新 tab/命令；schedule/工具行不受影响）

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

- [ ] `done`：证据齐全，可合并（且复评通过）
- [ ] `abandoned`：原因：待填

## 7. 复用证据（C17）

- 搜索记录：`rg -n "job_list|job_kill|subagent|agent_team|goal|todo|schedule|present|turnTail" packages apps crates docs`；
  `./target/debug/zk find kit.jobs`、`zk find kit.subagent`；capability 索引 `kit.jobs`（`shared/contracts/capability-index.json:107`）、
  `kit.subagent`、`contract.ipc`（`:43`）、`kit.tools`（`:131`）
- 复用清单：`zk_kit_jobs::{list,output,kill}`、`zk_kit_subagent::{list,runner}` + `SubagentRunner::{status,result,interrupt}`、
  goal.get（command-surface；goal 单 owner）、既有持久化工具行（todo/present/agent_team）、`zk_tool_schedule::tool().call_json`（list/update/delete）、Phase 2b 右栏/预览/turnTail 容器
- 新增包内实现（不登记 INDEX）：`activity.snapshot`/`activity.action` IPC + host 派生 helper；ui-inspector activity tab；ui-chat turnTail
- 无第二份实现：jobs/subagent 复用 kit registry；goal 复用 goal.get；todo/present 复用持久化工具行；schedule 复用工具包校验/读写；未修改工具状态模型
