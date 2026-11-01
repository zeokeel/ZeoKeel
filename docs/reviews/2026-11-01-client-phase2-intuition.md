# 2026-11-01-client-phase2-intuition · C20 直觉走查复核

- 状态：**CHANGES-REQUESTED**（P0=0 / P1=2 / P2=3；见 §3/§4）。
- 评审对象：HEAD `3bcfceea5f56c6e95d8d8bcf2634a73e5883053f` + 6 文件 SHA256（side/activity 各 `intuition.md`、`intuition-walkthrough.md` + 各自 `interaction.md`；§0.1）。
- 评审者：`spec-reviewer`（C20/C19 fresh-context 非实现者；未参与两份 spec/走查/交互契约的写作或实现）。
- 独立性：3-persona 走查由本评审者独立进行；只读 AGENTS.md、`docs/27` §2.7/§2.8、`docs/19` §11.1、`docs/11` §8.6、模板与两份 spec/interaction/intuition 文件；唯一写入为本记录。
- 评审日期：2026-11-01。
- 结论：**CHANGES-REQUESTED（P0=0 / P1=2 / P2=3）**，不可在本轮声称「符合直觉」或进入实现；修复 P1 后由本评审者复评。
- 关联 spec：`2026-11-01-client-side-panels`、`2026-11-01-client-activity-panels`。
- 复评安排：P1-1/P1-2 修复后，同一评审者复评两份 intuition/walkthrough 与 interaction 映射；P0 摩擦保持 closed；用户测试仍为 UNVERIFIED 时不得声称直觉已证实。

## 0. 方法与对象

### 0.1 对象 SHA256（6 文件）

```text
25947d6a4ade7e13eb7eceede772d39b28a5e2d96f0d8084ba8aed394d76abb5  specs/changes/2026-11-01-client-side-panels/intuition.md
2ac91eeb1a9fc1a7dadf6436fed873bf2867e2fc56b8daca96d9de75fd25538c  specs/changes/2026-11-01-client-side-panels/intuition-walkthrough.md
590d42be434179941f461431922a5cf852ed87b6670ea8aecc483f0da53c6fcd  specs/changes/2026-11-01-client-side-panels/interaction.md
5f1b19c1de10c07165adc99b7387301469d0257586e6db1bb165495cdb5b5fb9  specs/changes/2026-11-01-client-activity-panels/intuition.md
5976d13bba50f19a3124a935f25f2fcc4fd74edf45ae2da712c636d11021d7f8  specs/changes/2026-11-01-client-activity-panels/intuition-walkthrough.md
0d9f3e83e7ed70500f56124d01522331e238c0ab04a8cd057279ebafafac4c22  specs/changes/2026-11-01-client-activity-panels/interaction.md
```

### 0.2 复审依据

`AGENTS.md` C20/C19；`docs/27` §2.7（INT 判据、全仓唯一 ID、验证方式）与 §2.8（3 persona × 3 top task × 4 问、摩擦表、P0 未解决不得实现）；`docs/19` §11.1（第六/七件、P0 映射、planned 口径、用户测试 UNVERIFIED）；`docs/11` §8.6（证据口径、e2e 不能证明直觉）；`specs/templates/intuition*.md`；两份 spec/design/tasks/interaction 与已批准的 Phase 2 评审记录（`docs/reviews/2026-11-01-client-phase2-specs.md`）。

## 1. 机械核对（存在性/字段/映射）

| 检查 | side-panels | activity-panels | 结论 |
| --- | --- | --- | --- |
| `intuition.md` 状态 | `planned`（头/尾） | `planned` | 符合在途 spec 口径；实现开始必须改 `active` |
| INT 行数（≥3） | 12（INT-01…12） | 12（INT-01…12） | 通过 |
| 每行含场景/top task、可观察期望、反例、验证方式、证据 | 有 7 列，逐行完整 | 有 7 列，逐行完整 | 通过 |
| 用户测试未执行 | **UNVERIFIED**（`intuition.md` 覆盖说明 + walkthrough §4） | **UNVERIFIED** | 通过；不得声称“符合直觉” |
| interaction P0 ID 列表 | 7：TREE-01、TREE-02、PREVIEW-01、TERMINAL-01、PERSIST-01、SES2-OPS-01、SEARCH-01 | 7：SNAPSHOT-01、POLL-01、JOB-STOP-01、SUBAGENT-INTERRUPT-01、SCHEDULE-EDIT-01、SCHEDULE-DELETE-01、TURNTAIL-PRESENT-01 | 与交互契约一致 |
| 每个 P0 ID 至少 1 条 INT 映射（逐 ID 脚本核对） | 7/7 覆盖 | 7/7 覆盖 | **通过**（不只看覆盖说明；缺失集合为空） |
| INT ID 语义 | INT-01…12 为 docs/27 原则 ID | 同左 | 通过（原则集复用，与既有 composer/baseline 一致；非实例唯一 ID） |
| walkthrough 3 persona | P1/P2/P3 列全 | P1/P2/P3 列全 | 部分通过（见 P1-1） |
| walkthrough 3 top task | T1/T2/T3 列出 | T1/T2/T3 列出 | 通过（任务定义本身） |
| 每步行 4 问 | 表含 ①②③④ 四列 | 同 | 通过 |
| 摩擦表字段 | INT/严重度/最小修法/状态齐 | 同 | 通过 |
| P0 摩擦状态 | W4/W6/W7 P0 均 `closed`；无 OPEN | W2/W4/W7 P0 均 `closed`；无 OPEN | 通过 |
| P0 修法与 spec/design/tasks 引用 | W4→design §2.3/tasks #6/12；W6→design §2.4/spec R5/tasks #9/10；W7→design §2.3/§2.4/tasks #5/8/12 | W2→design §2.3/§2.4/tasks #4/6/9；W4→spec R6/design/tasks #4/6；W7→design/tasks #6 | 基本一致（activity W2 的 version 表述见 P2-1） |

> `ui-interaction-gate.py` 对 planned 契约只 WARN；本次质量判断不依赖 gate（gate 只查存在性，见 §4）。

### 1.1 P0 ID → INT 逐条映射（脚本核对，不只看覆盖说明）

| change | P0 ID | 映射 INT |
| --- | --- | --- |
| side | `INS2-TREE-01` | INT-01/02/07/09/11 |
| side | `INS2-TREE-02` | INT-03/06/10 |
| side | `INS2-PREVIEW-01` | INT-03/07/10 |
| side | `INS2-TERMINAL-01` | INT-01/03/05/09 |
| side | `INS2-PERSIST-01` | INT-06/11 |
| side | `INS2-SES2-OPS-01` | INT-01/04/05/12 |
| side | `INS2-SEARCH-01` | INT-03/07/11 |
| activity | `ACT2-SNAPSHOT-01` | INT-01/03/07/10/12 |
| activity | `ACT2-POLL-01` | INT-02/06/11 |
| activity | `ACT2-JOB-STOP-01` | INT-03/05/06 |
| activity | `ACT2-SUBAGENT-INTERRUPT-01` | INT-03/05/06 |
| activity | `ACT2-SCHEDULE-EDIT-01` | INT-03/04/05 |
| activity | `ACT2-SCHEDULE-DELETE-01` | INT-02/04/05/09 |
| activity | `ACT2-TURNTAIL-PRESENT-01` | INT-01/06/10 |

- 核对结果：side 7/7、activity 7/7，缺失集合为空；无“覆盖说明声称覆盖但实际漏 ID”的情况。


## 2. 独立 3-persona 走查（fresh-context）

### 2.1 复核结论

- **P0 摩擦**：两份 walkthrough 各自声明的 P0 均与 spec/design 的修法一致，且状态 `closed`，无 OPEN；机械覆盖 7/7 通过。
- **P1-1（阻断）**：walkthrough 未覆盖 **3 persona × 3 top task 的完整交叉矩阵**，只覆盖了部分组合（详见 §3.1）；这是 C20 / docs/27 §2.8 的结构性缺口，不能用现有行“覆盖所有人/任务”代替。
- **P1-2（阻断）**：activity 的「停止 job / 中断 subagent」是不可逆动作，但 spec/design/interaction 没有二次确认或撤销/恢复；`intuition.md` INT-05 以「仅 running 可用」替代确认，walkthrough W3 已承认误操作风险却未落修法（详见 §3.2）。这是 fresh-context 危险默认问题。
- **P2**：见 §3.3（activity W2 的 snapshot version 表述、INT 证据列可更具体、planned/active 门禁口径提醒）。
- 说明：side/activity 的 INT-01…12 是 `docs/27` §2.7 的**原则 ID**（两文件复用同一原则集，与既有 composer/`ui-interaction-verification` 一致），不是 per-change 实例 ID；不构成唯一性缺陷（本评审已核对，未误报）。

### 2.2 Persona/Task 实测（本评审者）

- **P1 首次使用者**：side 的 T1（右栏/files/预览）可完成，T2（终端/会话操作）在 walkthrough 无 P1 行；activity 的 T1/T3 有 P1 行，T2（schedule）无 P1 行。
- **P2 高频键盘**：side 的 T2/T3 有行，T1 无 P2 行；activity 的 T1/T2/T3 均有 P2 行。
- **P3 辅助技术/出错**：side 的 T1/T3 有行，T2 无 P3 行；activity 的 T2/T3 有行，T1 无 P3 行。
- 因矩阵缺口，本轮不能给出「所有 persona 在所有 top task 上均无 P0 摩擦」的结论。

## 3. 问题清单（P0/P1/P2，文件:行 + 问题 + 修法 + 状态）

### P1（阻断；修复后复评）

**P1-1 walkthrough 未覆盖 3 persona × 3 top task 完整矩阵**
- 位置：`side-panels/intuition-walkthrough.md` §3 表（W1–W9）与结论；`activity-panels/intuition-walkthrough.md` §3 表（W1–W10）与结论；要求见 `docs/27` §2.8、`docs/19` §11.1、任务说明「3 persona × 3 task × 4 问」。
- 问题：side 的 persona/task 组合只有 `P1/T1、P2/T2、P2/T3、P3/T1、P3/T3`，缺 **P1/T2、P1/T3、P2/T1、P3/T2**；activity 只有 `P1/T1、P1/T3、P2/T1、P2/T2、P2/T3、P3/T2、P3/T3`，缺 **P1/T2、P3/T1**。因此「3×3」未被完整走查；在缺格上无法证明「目标/控件/反馈/恢复」四问，结论不能覆盖所有 persona/top task。
- 修法：补齐缺失组合（side 4 行、activity 2 行），逐行填写 4 问与卡点；若某组合确实不适用，必须在该格写明「N/A + 理由 + 替代验证」，不能隐去。补后更新 §3 P0 结论与 §5 复核状态；建议在 walkthrough 顶部加 3×3 勾选表（P1/P2/P3 × T1/T2/T3）防止再次漏格。
- 状态：OPEN。

**P1-2 activity 的「停止 job / 中断 subagent」是不可逆动作，但无确认/撤销（危险默认）**
- 位置：`activity-panels/spec.md:20-23`（R2 可停止运行中 job）、`:28-32`（R3 中断 subagent）、`activity-panels/interaction.md` 的 `ACT2-JOB-STOP-01`/`ACT2-SUBAGENT-INTERRUPT-01`、`activity-panels/intuition.md:20` 的 INT-05（默认安全，只写「仅 running 可用且不误伤已完成对象」）、`activity-panels/intuition-walkthrough.md` W2/W3（W3 自己写「停止不可逆且误操作风险」但仅列为 P1 并关闭，`W3` 最小修法的「确认/错误就地」未落到 spec/design）。
- 问题：kill job/中断 subagent 会直接结束进程/丢弃工作，属于 docs/27 §2.7 INT-05 定义的「危险/不可逆操作需确认或可撤销」；当前 spec/design/interaction 均未要求二次确认或撤销/恢复，INT-05 反而用「仅 running」替代确认，W3 已承认误操作风险却未给确认修法。用户误点一个正在跑的长任务即不可恢复；这是 fresh-context 走查发现的 P1 危险默认（不是已关闭的 P0 作用域误停）。
- 修法（择一，并落 spec/design/interaction/INT）：
  (a) 对 kill/interrupt 加内联二次确认（复用 schedule delete 的确认模式；默认焦点在「取消」）；或
  (b) 提供明确 undo/restart/恢复入口并在 INT-05 写清；或
  (c) 若产品决定不确认，必须在 INT-05/acceptance 记录明确风险接受（Lead 决策）并改掉「危险动作需确认或可撤销」的判据表述。
  同时补 `ACT2-JOB-STOP-01`/`ACT2-SUBAGENT-INTERRUPT-01` 的确认/撤销断言与 walkthrough W2/W3 关闭证据。
- 状态：OPEN。

### P2（非阻断；建议本轮顺手修）

1. **P2-1 activity W2 的 P0 修法引用未定义 `snapshot version`**：`activity-panels/intuition-walkthrough.md` W2 写「动作前 version/重新 snapshot」，`activity/design.md` §2.3 也写「动作前记录 snapshot version」，但 `ActivitySnapshot`/`ActivityActionPayload` DTO 与 spec 均无 `version` 字段（design §2.2）。P0 的实际安全修法（cwd/`Path::starts_with`、仅 running、busy）已足够；建议要么删除 version 表述改为「动作后重新 snapshot + 以 host 当前状态校验」，要么在 contract 中正式加 `version` 并进 schema/tasks。状态：OPEN。
2. **P2-2 INT「证据」列多为 interaction ID/走查泛称**：两份 `intuition.md` 的证据列引用 `INS2-*`/`ACT2-*` 或「走查」，没有具体证据路径/测试名；建议按模板补 `target/evidence/<change>/interaction/<ID>.json` 或「review: walkthrough Wn」以便验收追踪。状态：OPEN。
3. **P2-3 planned→active 与用户测试边界提醒**：两份 intuition 状态均 `planned`，docs/11 §8.6 允许 planned 只 WARN；实现开始必须同时把 `interaction.md`/`intuition.md` 改 `active`、补 P0/P1 标签测试与 evidence；用户测试（≥5 人）仍为 **UNVERIFIED**，不得在 acceptance/发布文案声称「符合直觉/直观」。建议在 tasks/acceptance 增加这一行。状态：OPEN。

## 4. 门禁与证据口径

| 命令 | exit | 结果 |
| --- | --- | --- |
| `python3 scripts/spec-check.py` | **0** | 0 errors / 2 warnings（`ui-interaction-verification` in-progress、`ui-intuition-heuristics` draft，均非本两份 spec） |
| `python3 scripts/docs-check.py` | **0** | 0 errors / 0 warnings / 41 files |
| `python3 scripts/review-check.py` | **0** | ok（31 records；本记录写入后将被计数） |
| `python3 scripts/ui-interaction-gate.py --self-test` | **0** | 12 checks OK（存在性/反例门禁） |
| `python3 scripts/ui-interaction-gate.py` | **0** | 1 intuition contract / 12 rows；side/activity interaction `planned` → WARN skip；gate 只查存在性，不判定 INT 质量 |

- 未验证（不冒充）：≥5 名真实用户 think-aloud；两份 spec 的 P0/P1 interaction 实测证据；e2e 实现。用户测试按 `docs/27` §2.8 在发布前执行，证据路径 `target/evidence/<change>/usability/`，当前 **UNVERIFIED**。
- 质量判定：本次以 fresh-context 走查 + 机械映射核对为准，不把 `ui-interaction-gate` 的 exit 0 当作直觉已证明。

## 5. 结论与后续

- **结论：CHANGES-REQUESTED（P0=0 / P1=2 / P2=3）**。
- P1-1（3×3 矩阵）与 P1-2（job stop/subagent interrupt 无确认/撤销的危险默认）修复前，不得声称走查完成，不得据此开始 side/activity 实现；用户测试未跑前也不得声称「符合直觉」。
- 修复后同一评审者复评（本记录 §0.1 hash 失效；修复会改 4 个 intuition 文件及可能的 interaction 引用），确认 P0 摩擦仍 closed、P0 映射仍 7/7、矩阵满格、INT 唯一。
- **若后续复评 APPROVED**：side-panels 可在 composer-surface 之后开始实现，activity-panels 再在其后；实现开始时把 `interaction.md`/`intuition.md` 从 `planned` 改 `active` 并补 P0/P1 标签测试与 evidence；用户测试仍按发布前 ≥5 人执行。
- 评审记录字段：评审对象（HEAD+6 hash）、评审者、独立性、结论、P0/P1/P2、复评安排、关联 spec（两个 change id 均出现）。
- 复评日期/结论：**2026-11-01 / CHANGES-REQUESTED**（首轮 C20 复核；P0=0、P1=2、P2=3）。

## 6. 复评（第二轮，2026-11-01）

- 复评对象：HEAD `3bcfceea5f56c6e95d8d8bcf2634a73e5883053f` + 三份文件共 6 个 SHA256（side/activity 的 `intuition.md`、`intuition-walkthrough.md`、`interaction.md`；§6.1）。
- 评审者/独立性：`spec-reviewer`（同一评审员；未参与 spec/走查/实现；唯一写入仍为本记录）。
- 复评日期/结论：2026-11-01 / **APPROVED**（P0=0 / P1=0 / P2=0；用户测试仍 UNVERIFIED，见 §6.5）。
- 变更核对：与首评 §0.1 对照，6 文件中 **5 个变化、1 个未变**（`side/interaction.md` 未变）；变化仅限作者声明的 P1-1/P1-2/P2-1/P2-2/P2-3 修复与对应 acceptance/tasks 同步，无其它范围扩张。

### 6.1 本轮对象 SHA256（6 文件）

```text
f6d7906f6b0687b19fb0d4b83a1a5a005a7f3c8bdf735e6b3260d701da9403e9  specs/changes/2026-11-01-client-side-panels/intuition.md
d2e45820bd5e3e7dcf52c94857631436db644ba94ae6fd31152a286313d79575  specs/changes/2026-11-01-client-side-panels/intuition-walkthrough.md
590d42be434179941f461431922a5cf852ed87b6670ea8aecc483f0da53c6fcd  specs/changes/2026-11-01-client-side-panels/interaction.md
a371b7ba25a3e3fb6f23770fb669afb6026d88d481b37eb827967dbdc7d340c0  specs/changes/2026-11-01-client-activity-panels/intuition.md
709e248382fbf32415615cdbe61748241d25bf709cb61a0f7dbddb51b7b4795d  specs/changes/2026-11-01-client-activity-panels/intuition-walkthrough.md
c5562cc3a00ab2d4bbdbeab8932505d6af89506a38a08dce6928f7b662256dd8  specs/changes/2026-11-01-client-activity-panels/interaction.md
```

### 6.2 首评 P0/P1/P2 关闭核对

| 条目 | 状态 | 复评依据 |
| --- | --- | --- |
| **P1-1** 3 persona × 3 top task 矩阵 | **RESOLVED** | side walkthrough 新增 W10–W13，activity 新增 W11–W12；脚本核对两份组合集合均为完整 9 格（P1/P2/P3 × T1/T2/T3）；N/A 格（side W10/W11、activity W11）写明「不适用 + 理由 + 替代验证」，4 问列齐全；两份 §3 P0 摩擦结论更新为「矩阵已满格」。 |
| **P1-2** job 停止/subagent 中断危险默认 | **RESOLVED** | activity spec R2:23/R3:34 冻结内联二次确认：显示 id/状态/影响、默认焦点「取消」、确认后才发 action、取消/Esc 不变；`role=alertdialog` + aria-label；interaction `ACT2-JOB-STOP-01`/`ACT2-SUBAGENT-INTERRUPT-01` 写明取消不发 action；design:60/146 复用 ui-kit 既有确认模式、不新增第二套 Dialog；INT-05 改为三动作均有确认；walkthrough W2/W3/W4、tasks #6/#9、acceptance 已同步；P0 状态 closed。 |
| **P2-1** 未定义 snapshot version | **RESOLVED** | activity design:58 / tasks:20 改为「动作后重新 snapshot，并以 host 当前状态校验（不新增 version 字段）」；walkthrough W2 不再提 version；`ActivitySnapshot` DTO 未新增字段，无残留矛盾。 |
| **P2-2** INT 证据路径具体化 | **RESOLVED** | 两份 `intuition.md` 的 INT 行证据列已补 `target/evidence/<change>/interaction/<ID>.json` + `e2e/interaction/<surface>.spec.ts` + 走查行号（如 side INT-01/02/03；activity INT-02/05/09）；路径命名与 interaction 的 test/evidence 约定一致，待实现时创建/写入（planned）。 |
| **P2-3** planned/active 与用户测试边界 | **RESOLVED** | 两份 `intuition.md` 保持 planned 并写明实现开始时与 interaction 同时改 active；acceptance 已加「用户测试 UNVERIFIED；未完成前 acceptance/发布文案不得声称符合直觉/直观」；tasks 验收项同步。 |
| **P0 摩擦** | **保持 closed** | side W4/W6/W7、activity W2/W3/W4/W7 均 closed；无 OPEN；P0 修法与 spec/design/interaction/tasks 引用一致；P0 交互 ID 映射仍为 side 7/7、activity 7/7（本轮复跑脚本）。 |
| **INT 原则 ID** | **不构成本轮缺陷** | 两文件复用 docs/27 §2.7 的 INT-01…12 原则 ID（与 composer/ui-interaction-verification 既有基线一致），不是 per-change 实例唯一 ID；首评未将其列为问题，复评确认无新增唯一性冲突。 |

### 6.3 确认交互复核要点

- `role=alertdialog`、可读 aria-label（「确认停止任务」/「确认中断子代理」）、默认焦点「取消」、取消/Esc 不发 action、确认后 `aria-busy` + 状态刷新，均在 spec/design/interaction/INT/任务/走查中一致出现。
- 复用 ui-kit 按钮 + 既有内联确认模式，**不新增第二套 Dialog**（C17）；无同时存在两套确认模式的路径。
- 确认动作只对 running job/subagent 开放；未知/已结束仍映射可读错误；不产生工具行/审批。

### 6.4 矩阵/证据/映射复核

- side 组合集合：P1/T1、P1/T2、P1/T3、P2/T1、P2/T2、P2/T3、P3/T1、P3/T2、P3/T3（9/9）；activity 同（9/9）。
- N/A 组合均有理由与替代验证（side W10/W11、activity W11），不是空行。
- P0 映射脚本：side 7/7、activity 7/7（缺失集合为空）；证据路径语法可解析，指向 implementation 时需创建的 e2e/evidence 文件。
- 用户测试：两份均 **UNVERIFIED**（未执行）；≥5 人 think-aloud 证据路径已规划；未执行前不得声称「符合直觉/直观」。

### 6.5 门禁（2026-11-01）

| 命令 | exit | 结果 |
| --- | --- | --- |
| `python3 scripts/spec-check.py` | **0** | 0 errors / 2 warnings（`ui-interaction-verification` in-progress、`ui-intuition-heuristics` draft，非本两份 spec） |
| `python3 scripts/docs-check.py` | **0** | 0 errors / 0 warnings / 41 files |
| `python3 scripts/review-check.py` | **0** | ok（33 records；本记录追加后为最新 APPROVED） |
| `python3 scripts/ui-interaction-gate.py --self-test` | **0** | 12 checks OK（存在性/反例门禁） |
| `python3 scripts/ui-interaction-gate.py` | **0** | 4 contracts / 7 tests / 34 tags / 54 ids；side/activity interaction planned → WARN skip；intuition=1 contract/12 rows（gate 只查存在性，不判定直觉质量） |

- 未验证（不冒充）：两份 spec 的 P0/P1 interaction 实测证据、≥5 名真实用户 think-aloud、e2e 实现；均为 implementation/release 前置。

### 6.6 结论与后续

- **首评 P0/P1/P2 全部 RESOLVED**；本轮未发现新 P0/P1/P2。**复评结论：APPROVED（P0=0 / P1=0 / P2=0）**。
- **可实现性**：side-panels 与 activity-panels 的直觉走查/交互契约在当前 approved spec 下可进入实现；顺序 **composer-surface → side-panels → activity-panels**（activity 依赖 side 的 `previewRequest`/file_read/预览）。
- 实现开始时：两份 `interaction.md` 与 `intuition.md` 由 `planned` 改 `active`；补 P0/P1 `@interaction:<ID>` 测试与 `target/evidence/...` 证据；确认交互增量测试 `job-stop-confirm`/`subagent-interrupt-confirm` 落地。
- 用户测试：发布前完成 ≥5 人 think-aloud 并写 `target/evidence/<change>/usability/`；在此之前 acceptance/发布文案不得声称「符合直觉/直观」（仍 UNVERIFIED）。
- 失效条件：§6.1 的 6 文件任一 hash 变化（除批准后的行政性回填/本记录自身）→ 本复评失效；实现阶段 §6.3 的确认交互若改动 spec/interaction，需重新走查/复评。
- 唯一写入：本记录；未改 spec/代码/生成物。
