# 2026-11-01-client-activity-panels · 修订版快速评审

- 状态：**APPROVED（P0=0 / P1=0 / P2=2，P2 非阻断）**。
- 评审对象：HEAD `22b453e97b87e3b8b2d8d21e9f573d95018349a1` + `2026-11-01-client-activity-panels` 8 文件 SHA256（§0.1）。
- 对象：HEAD `22b453e97b87e3b8b2d8d21e9f573d95018349a1` + `2026-11-01-client-activity-panels` 8 文件 SHA256（§0.1）。
- 关联 spec：`2026-11-01-client-activity-panels`（revision，2026-11-01）。
- 评审者：`spec-reviewer`（C19/C20 fresh-context，非实现者）。
- 独立性：仅读 spec/code/docs/reviews；未参与本 revision 写作/实现；唯一写入本记录。
- 评审日期：2026-11-01。
- 结论：**APPROVED（P0=0 / P1=0 / P2=2）**；可进入实现。P2 为文档/门禁引用更新，不阻塞。
- 复评安排：本记录为修订版评审；实现开始前 `interaction.md`/`intuition.md` 改 `active`；用户测试 UNVERIFIED。

## 0. 对象

### 0.1 SHA256（8 文件）

```text
1ae225277e175f3eef3cc1437f1c9d2dcfb1c7ec5274362f96ce8ea14ab0cd26  specs/changes/2026-11-01-client-activity-panels/acceptance.md
c61a1dc161c238e57e515f0fd31ea73262b05d5afd673f6e611b1ba4be67d19d  specs/changes/2026-11-01-client-activity-panels/design.md
084bc69e735107b5382ac69aac20ecd80272f2f994e6a8fac07135d6040c22e9  specs/changes/2026-11-01-client-activity-panels/interaction.md
acd67abd76916dd8405fb201f63129b7ce039111690beaee14bd828558949ad7  specs/changes/2026-11-01-client-activity-panels/intuition.md
7feae7c1542152afdf3f3b8f0fbec22280882c7afce0ac66552a6ac4524a8f65  specs/changes/2026-11-01-client-activity-panels/intuition-walkthrough.md
f2d9d3c53106045e2f502cb7fd95f872cd235e10adeb2a6fe56ea09c04709f4d  specs/changes/2026-11-01-client-activity-panels/proposal.md
1838d0015ec2d384a4b30c939d95d26ab5526894653e8b17596206fe812e3636  specs/changes/2026-11-01-client-activity-panels/spec.md
c537a4b71d676fade63a2ce2b6175f5b2804939f5d5caf12a92f532497d1ef01  specs/changes/2026-11-01-client-activity-panels/tasks.md
```

## 1. 修订点逐条确认

| # | 修订点 | 结论 | 依据 |
| --- | --- | --- | --- |
| 1 | plan 移出 activity：R4/D3/interaction/INT/walkthrough/tasks/acceptance 删除 plan；plan tab order 50/PlanCard/plan.get/update/exit_plan_mode 归 `client-plan-mode`；activity order 40 不变 | **一致** | `spec.md:128` D3 superseded；`spec.md:69-71` R1/R7 不含 plan；`design.md:12` 明确 plan 由 plan-mode `.zeokeel/plans.json`；`interaction.md` 无 TURNTAIL-PLAN/plan 条目；`intuition.md` INT-08/INT-12 说明 plan 不展示；`intuition-walkthrough.md` W6/W10 无 plan；`tasks.md:0`/`acceptance` 同步；`plan-mode` 已 order50。 |
| 2 | goal 只读 `goal.get`（command-surface host IPC，`.zeokeel/goal.json` 单 owner）；snapshot 不含 goal；无工具行派生/写面 | **一致** | `spec.md:37-42` R4；`design.md:24/42/2.3` goal 不派生；`design.md:2.2 ActivitySnapshot` 无 goal 字段；`interaction` `ACT2-SNAPSHOT-01` goal 单独 goal.get；`tasks.md` #4 goal 读 goal.get；`acceptance` 手工检查同；`tasks.md` 不写 goal 工具行/写面。 |
| 3 | turnTail 仅 present deliverables；不扫描 plan 标题 | **一致** | `spec.md:63-73` R7；`design.md:154` 仅 present；`interaction` `ACT2-TURNTAIL-PRESENT-01`（无 TURNTAIL-PLAN）；`intuition` W6/INT-08；`walkthrough` W6；`tasks.md` #7。 |
| 4 | 数据源表：jobs/subagent registry；todo/agent-team 工具行 replay；present 工具行；schedule `.zeokeel/schedules.json`；`activity.snapshot/action` 保留 | **一致** | `spec.md:11/12/98-100`（§4 数据源）；`design.md:21-27/2.3`；`tasks.md` #1/#4；`acceptance` §3；无 goal 进 snapshot、无 plan 读取。 |
| 5 | C20：P0 7/7 映射；interaction 删除 TURNTAIL-PLAN；INT/walkthrough 同步；用户测试 UNVERIFIED | **通过** | interaction P0 = SNAPSHOT/POLL/JOB/SUBAGENT/SCHEDULE-EDIT/DELETE/TURNTAIL-PRESENT 共 7；`intuition.md` 覆盖映射 7/7（脚本核）；walkthrough 3×3（W1–W12，含 W11 N/A+替代）；无 TURNTAIL-PLAN；用户测试 UNVERIFIED。 |
| 6 | 与已合并接口一致 | **通过（见 §2）** | `goal.get`/`GoalView`/`GoalResult` 当前代码；`plan.get/update` 当前代码；`ui-inspector` 现有 files/changes/terminal/plan，activity 插入 order40 在 plan50 前；`tool-schedule::call_json` 公开；`zk-kit-jobs`/`zk-kit-subagent` 现有；新 `activity.snapshot/action` 为 additive。 |
| 7 | C1–C20 无新冲突 | **通过** | C2/C4/C10/C11/C17/C18/C20 见 §3；无新 slot/包；host 经 generated facade；无第二套状态机。 |
| 8 | 预算 ≤15 | **通过** | `proposal.md:108-118` 15 文件（契约1+生成器1+desktop1+ui-kit2+inspector1+ui-chat1+desktop-web3+e2e2+docs3）；`tasks.md:53` 预算 15；实际清单一致；Lead 例外 14+docs/26 已记录。 |

## 2. 接口核对（已合并/当前 code，path:line）

| 接口 | 现状 | activity revision 使用 | 结论 |
| --- | --- | --- | --- |
| `goal.get` / `GoalResult`/`GoalView` | `contracts/ipc/src/lib.rs:170/696-726`；`command_surface.rs:401-456`；`App.tsx:819` 已用 | activity goal 分区只读 `goal.get`；GoalResult.goal: Option<GoalView>（objective/status/rounds/max_rounds/reason/last_result）；**不放进 ActivitySnapshot** | 一致 |
| `plan.get`/`plan.update` | `contracts/ipc/src/lib.rs:190/192/839-846`；`plan.rs`；`ui-inspector` plan tab | activity 不读 plan；R7/INT 明确 plan 由 plan-mode 负责；plan tab order 50 | 一致 |
| `activity.snapshot`/`activity.action` | 当前 contract 尚未新增（本 spec task1 新增；additive） | ActivitySnapshot/ActivityActionPayload DTO；不含 goal | 待实现，方向一致 |
| `ui-inspector` tabs | 当前 `InspectorTab` files/changes/terminal/plan（`index.tsx:9/381`）；side-panels order 10/20/30，plan-mode 50 | tasks #6 增加 activity order40，顺序 files→changes→terminal→activity→plan；不新增 slot | 一致 |
| `tool-schedule` 公开 `call_json` | `tool-schedule/src/lib.rs:76-160`；`zk-composed` facade 由生成器提供 | host 经 facade 调 list/update/delete；不 front import | 一致 |
| `zk-kit-jobs` / `zk-kit-subagent` | `crates/zk-kit-jobs` list/output/kill；`zk-kit-subagent` list/runner/status/interrupt | host snapshot/action 复用 | 一致 |
| `goal.get` UI 读取 | `ui-kit/context.tsx:87-90/157` 已有 `goal`/`refreshGoal`；App 已有 goal 状态 | activity goal 分区可复用现有 `goal` 状态/`refreshGoal`；无需第二套 goal IPC | 一致（实现时 ensure tab 打开时 refresh） |

## 3. C1–C20

| 约束 | 结论 |
| --- | --- |
| C2 构建期组合 | 通过（无新 slot/包/运行期加载；tab 内联 ui-inspector） |
| C4 契约缝 | 通过（app 不 import tool；host 经 `zk-composed` generated facade；UI 经 UiActions/IPC） |
| C10 外部输入 | 通过（新增 IPC DTO/schema；Ajv/generated schema 校验；activity action id/updates 白名单） |
| C11 生成物 | 通过（生成器源 `tools/zk/src/main.rs`；generate schema/TS；不手改模板/产物） |
| C17 单一实现 | 通过（jobs/subagent kit、goal.get、todo/present 工具行、tool-schedule 公开 API；无第二套状态/轮询/确认） |
| C18 workspace/会话归属 | 通过（jobs cwd canonical `Path::starts_with`；subagent parent；schedule 当前 workspace；goal/get conversation） |
| C20 | 通过机械项（P0 7/7、3×3、P0 摩擦 closed、用户测试 UNVERIFIED）；实现开始前 active |
| 其他 | 无新 DB migration/网络/审批/企业策略；plan 已移交，无重复实现 |

## 4. 问题清单

### P0 / P1

- **无**（P0=0 / P1=0）。

### P2（文档性，不阻塞）

1. **P2-1 门禁工具状态文案仍停在“未落地”**：`interaction.md` §5 仍写 `pnpm e2e:ui`/`ui-interaction-gate.py`「当前尚不存在，UNVERIFIED」；`acceptance.md` §1/§1.5 也写「待工具落地/不存在」。实际 `package.json` 已有 `e2e:ui`、`e2e:ui:gate`，`scripts/ui-interaction-gate.py` 已落地（task 已用）；建议实现前更新为可用命令，避免实施者误判。
2. **P2-2 acceptance 评审出处仍是旧评审**：`acceptance.md:3` 引用 `docs/reviews/2026-11-01-client-phase2-specs.md §15` 作为已通过 C19；修订版应以本记录（`2026-11-01-client-activity-panels-revision.md`）为准，置 approved 前更新链接/复评结论。

## 5. 门禁

- `python3 scripts/spec-check.py`: exit 0（0 errors / 2 warnings，均为其他 in-progress/draft spec）。
- `python3 scripts/docs-check.py`: exit 0。
- `python3 scripts/review-check.py`: exit 0（本记录追加后计数 +1；最新结论 APPROVED）。
- 未验证：真实 e2e/serve/axe、用户测试；实现后补证据；不冒充。

## 6. 结论

- **P0=0 / P1=0 / P2=2（非阻塞）→ APPROVED**。
- 修订版与已合并 `goal.get`/`plan.get/update`/`ui-inspector` tab/`tool-schedule`/kit 接口一致；plan 已移出 activity，turnTail 仅 present，goal 不进 snapshot，预算 15 真实。
- **可进入实现**（Phase 2b → activity-panels；前置 side-panels/composer 已合并）。实现开始时 `interaction.md`/`intuition.md` 改 `active`，补 P0/P1 标签 e2e 证据；用户测试 UNVERIFIED，发布前 ≥5 人 think-aloud，此前不得声称“符合直觉”。

- 对象：HEAD `22b453e97b87e3b8b2d8d21e9f573d95018349a1` + 8 hash（§0.1）；评审者/独立性/结论/P0/P1/P2/复评/关联 spec 均已填。
- 复评日期/结论：**2026-11-01 / APPROVED（P0=0、P1=0、P2=2）**。
